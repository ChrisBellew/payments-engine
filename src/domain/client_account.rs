use super::transaction::{Transaction, TransactionId};
use crate::domain::transaction::{Deposit, TransactionAction, Withdrawal};
use anyhow::{Error, Result};
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::{hash_map::Entry, HashMap};

pub type ClientId = u16;

#[derive(Debug, Serialize)]
pub struct ClientAccount {
    #[serde(rename(serialize = "client"))]
    pub client_id: u16,

    #[serde(rename(serialize = "available"))]
    pub available_balance: Decimal,

    #[serde(rename(serialize = "held"))]
    pub held_balance: Decimal,

    #[serde(rename(serialize = "total"))]
    pub total_balance: Decimal,

    pub locked: bool,

    #[serde(skip_serializing)]
    pub last_transaction_id: Option<TransactionId>,

    #[serde(skip_serializing)]
    pub applied_deposits: HashMap<u32, Deposit>,

    #[serde(skip_serializing)]
    pub disputed_deposits: HashMap<u32, Deposit>,

    #[serde(skip_serializing)]
    pub chargedback_deposits: HashMap<u32, Deposit>,
}

impl ClientAccount {
    pub fn new(client_id: u16) -> ClientAccount {
        ClientAccount {
            client_id,
            available_balance: Decimal::ZERO,
            held_balance: Decimal::ZERO,
            total_balance: Decimal::ZERO,
            locked: false,
            last_transaction_id: None,
            applied_deposits: HashMap::new(),
            disputed_deposits: HashMap::new(),
            chargedback_deposits: HashMap::new(),
        }
    }
    pub fn apply_transaction(&mut self, transaction: Transaction) -> Result<()> {
        let transaction_id = transaction.transaction_id;
        let transaction_description = transaction.to_string();

        if self.locked {
            return Err(Error::msg(format!(
                "Failed to apply {}: Account is locked",
                transaction_description
            )));
        }

        match transaction.action {
            TransactionAction::Deposit(deposit) => self.apply_deposit(transaction_id, deposit),
            TransactionAction::Withdrawal(withdrawal) => {
                self.apply_withdrawal(transaction_id, withdrawal)
            }
            TransactionAction::Dispute => self.apply_dispute(transaction_id),
            TransactionAction::Resolve => self.apply_resolve(transaction_id),
            TransactionAction::Chargeback => self.apply_chargeback(transaction_id),
        }
        .map_err(|err| {
            Error::msg(format!(
                "Failed to apply {}: {}",
                transaction_description,
                err.to_string()
            ))
        })
    }

    fn apply_deposit(&mut self, transaction_id: u32, deposit: Deposit) -> Result<()> {
        if !self.is_transaction_in_order(transaction_id) {
            return Ok(());
        }

        // The total balance will always be at least as high as the
        // available balance so let's check the total balance won't overflow.
        // If it won't, we can be sure the available balance won't overflow

        self.total_balance = self
            .total_balance
            .checked_add(deposit.amount)
            .ok_or(Error::msg("Deposit would cause balance overflow"))?;

        self.available_balance += deposit.amount;
        self.applied_deposits.insert(transaction_id, deposit);
        self.last_transaction_id = Some(transaction_id);

        Ok(())
    }

    fn apply_withdrawal(&mut self, transaction_id: u32, withdrawal: Withdrawal) -> Result<()> {
        if !self.is_transaction_in_order(transaction_id) {
            return Ok(());
        }

        if withdrawal.amount.gt(&self.available_balance) {
            return Err(Error::msg("Insufficient available balance for withdrawal"));
        }

        // The available balance can never underflow due to a withdrawal because
        // a withdrawal cannot leave a negative balance. The total balance can
        // never underflow because it will always be at least as high as the available balance

        self.available_balance = self.available_balance - withdrawal.amount;
        self.total_balance -= withdrawal.amount;
        self.last_transaction_id = Some(transaction_id);

        Ok(())
    }

    fn apply_dispute(&mut self, transaction_id: u32) -> Result<()> {
        match self.applied_deposits.entry(transaction_id) {
            Entry::Occupied(entry) => {
                let deposit = entry.get();

                // The held balance could overflow if there are already active disputes.
                // The available balance cannot underflow because either the held balance
                // would overflow and get caught here or a chargeback would lock the account.

                let held_balance = self
                    .held_balance
                    .checked_add(deposit.amount)
                    .ok_or(Error::msg("Dispute would cause held balance overflow"))?;

                self.available_balance -= deposit.amount;
                self.held_balance = held_balance;
                self.disputed_deposits
                    .insert(transaction_id, entry.remove());

                Ok(())
            }
            Entry::Vacant(_) => Ok(()),
        }
    }

    fn apply_resolve(&mut self, transaction_id: u32) -> Result<()> {
        match self.disputed_deposits.entry(transaction_id) {
            Entry::Occupied(entry) => {
                let deposit = entry.get();

                // The available balance cannot overflow due to a resolve because the total
                // balance would have overflowed beforehand. The held balance cannot
                // underflow because it's not possible to have a negative held balance.

                self.available_balance += deposit.amount;
                self.held_balance -= deposit.amount;
                self.applied_deposits.insert(transaction_id, entry.remove());

                Ok(())
            }
            Entry::Vacant(_) => Ok(()),
        }
    }

    fn apply_chargeback(&mut self, transaction_id: u32) -> Result<()> {
        match self.disputed_deposits.entry(transaction_id) {
            Entry::Occupied(entry) => {
                let deposit = entry.get();

                // The held balance cannot underflow because it's not possible
                // to have a negative held balance. The total balance cannot underflow
                // because the available balance would have underflowed first.

                self.held_balance -= deposit.amount;
                self.total_balance -= deposit.amount;
                self.chargedback_deposits
                    .insert(transaction_id, entry.remove());
                self.locked = true;

                Ok(())
            }
            Entry::Vacant(_) => Ok(()),
        }
    }

    fn is_transaction_in_order(&self, transaction_id: u32) -> bool {
        match self.last_transaction_id {
            Some(last_transaction_id) => {
                if last_transaction_id >= transaction_id {
                    return false;
                }
            }
            _ => (),
        };
        return true;
    }
}

#[cfg(test)]
mod tests {
    use super::ClientAccount;
    use crate::{
        assert_err::assert_err,
        domain::transaction::{Deposit, Transaction, TransactionAction, Withdrawal},
    };
    use anyhow::Result;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    #[test]
    fn applies_deposits() -> Result<()> {
        let client_id = 1;
        let mut client_account = ClientAccount::new(client_id);

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Deposit(Deposit {
                amount: dec!(12.5555),
            }),
        })?;

        assert_eq!(dec!(12.5555), client_account.available_balance);
        assert_eq!(dec!(12.5555), client_account.total_balance);

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 2,
            action: TransactionAction::Deposit(Deposit { amount: dec!(1) }),
        })?;

        assert_eq!(dec!(13.5555), client_account.available_balance);
        assert_eq!(dec!(13.5555), client_account.total_balance);

        Ok(())
    }

    #[test]
    fn applies_withdrawals_with_sufficient_available_balance() -> Result<()> {
        let client_id = 1;
        let mut client_account = ClientAccount::new(client_id);

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Deposit(Deposit {
                amount: dec!(12.5555),
            }),
        })?;

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 2,
            action: TransactionAction::Withdrawal(Withdrawal {
                amount: dec!(11.5555),
            }),
        })?;

        assert_eq!(dec!(1), client_account.available_balance);
        assert_eq!(dec!(1), client_account.total_balance);

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 3,
            action: TransactionAction::Withdrawal(Withdrawal { amount: dec!(1) }),
        })?;

        assert_eq!(dec!(0), client_account.available_balance);
        assert_eq!(dec!(0), client_account.total_balance);
        Ok(())
    }

    #[test]
    fn applies_dispute() -> Result<()> {
        let client_id = 1;
        let mut client_account = ClientAccount::new(client_id);

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Deposit(Deposit {
                amount: dec!(12.5555),
            }),
        })?;

        assert_eq!(dec!(12.5555), client_account.available_balance);
        assert_eq!(dec!(12.5555), client_account.total_balance);

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Dispute,
        })?;

        assert_eq!(dec!(0), client_account.available_balance);
        assert_eq!(dec!(12.5555), client_account.held_balance);
        assert_eq!(dec!(12.5555), client_account.total_balance);

        Ok(())
    }

    #[test]
    fn applies_dispute_after_withdrawal() -> Result<()> {
        let client_id = 1;
        let mut client_account = ClientAccount::new(client_id);

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Deposit(Deposit {
                amount: dec!(12.5555),
            }),
        })?;

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 2,
            action: TransactionAction::Withdrawal(Withdrawal {
                amount: dec!(12.5555),
            }),
        })?;

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Dispute,
        })?;

        assert_eq!(dec!(-12.5555), client_account.available_balance);
        assert_eq!(dec!(12.5555), client_account.held_balance);
        assert_eq!(dec!(0), client_account.total_balance);

        Ok(())
    }

    #[test]
    fn applies_resolve() -> Result<()> {
        let client_id = 1;
        let mut client_account = ClientAccount::new(client_id);

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Deposit(Deposit {
                amount: dec!(12.5555),
            }),
        })?;

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Dispute,
        })?;

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Resolve,
        })?;

        assert_eq!(dec!(12.5555), client_account.available_balance);
        assert_eq!(dec!(0), client_account.held_balance);
        assert_eq!(dec!(12.5555), client_account.total_balance);

        Ok(())
    }

    #[test]
    fn applies_chargeback() -> Result<()> {
        let client_id = 1;
        let mut client_account = ClientAccount::new(client_id);

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Deposit(Deposit {
                amount: dec!(12.5555),
            }),
        })?;

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Dispute,
        })?;

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Chargeback,
        })?;

        assert_eq!(dec!(0), client_account.available_balance);
        assert_eq!(dec!(0), client_account.held_balance);
        assert_eq!(dec!(0), client_account.total_balance);
        assert_eq!(true, client_account.locked);

        Ok(())
    }

    #[test]
    fn fails_to_apply_deposit_due_to_overflow() -> Result<()> {
        let client_id = 1;
        let mut client_account = ClientAccount::new(client_id);

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Deposit(Deposit {
                amount: Decimal::MAX,
            }),
        })?;

        let result = client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 2,
            action: TransactionAction::Deposit(Deposit { amount: dec!(1) }),
        });

        assert_err!(
            result,
            "Failed to apply deposit with transaction ID 2: Deposit would cause balance overflow"
        );
        assert_eq!(Decimal::MAX, client_account.available_balance);
        assert_eq!(Decimal::MAX, client_account.total_balance);
        Ok(())
    }

    #[test]
    fn fails_to_apply_withdrawal_with_insufficient_available_balance() -> Result<()> {
        let client_id = 1;
        let mut client_account = ClientAccount::new(client_id);
        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Deposit(Deposit {
                amount: dec!(12.5555),
            }),
        })?;
        let result = client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 2,
            action: TransactionAction::Withdrawal(Withdrawal { amount: dec!(13) }),
        });

        assert_err!(
            result,
            "Failed to apply withdrawal with transaction ID 2: Insufficient available balance for withdrawal"
        );
        assert_eq!(dec!(12.5555), client_account.available_balance);
        assert_eq!(dec!(12.5555), client_account.total_balance);
        Ok(())
    }

    #[test]
    fn fails_to_apply_dispute_due_to_overflow() -> Result<()> {
        let client_id = 1;
        let mut client_account = ClientAccount::new(client_id);

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Deposit(Deposit {
                amount: Decimal::MAX,
            }),
        })?;

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 2,
            action: TransactionAction::Withdrawal(Withdrawal {
                amount: Decimal::MAX,
            }),
        })?;

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 3,
            action: TransactionAction::Deposit(Deposit {
                amount: Decimal::MAX,
            }),
        })?;

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Dispute,
        })?;

        let result = client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 3,
            action: TransactionAction::Dispute,
        });

        assert_err!(
            result,
            "Failed to apply dispute for transaction ID 3: Dispute would cause held balance overflow"
        );
        assert_eq!(dec!(0), client_account.available_balance);
        assert_eq!(Decimal::MAX, client_account.held_balance);
        assert_eq!(Decimal::MAX, client_account.total_balance);

        Ok(())
    }

    #[test]
    fn fails_to_act_on_a_locked_account() -> Result<()> {
        let client_id = 1;
        let mut client_account = ClientAccount::new(client_id);
        client_account.locked = true;

        let deposit_result = client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Deposit(Deposit {
                amount: dec!(12.5555),
            }),
        });

        let withdrawal_result = client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 2,
            action: TransactionAction::Withdrawal(Withdrawal {
                amount: dec!(12.5555),
            }),
        });

        let dispute_result = client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Dispute,
        });

        let resolve_result = client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Resolve,
        });

        let chargeback_result = client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Chargeback,
        });

        assert_err!(
            deposit_result,
            "Failed to apply deposit with transaction ID 1: Account is locked"
        );
        assert_err!(
            withdrawal_result,
            "Failed to apply withdrawal with transaction ID 2: Account is locked"
        );
        assert_err!(
            dispute_result,
            "Failed to apply dispute for transaction ID 1: Account is locked"
        );
        assert_err!(
            resolve_result,
            "Failed to apply resolve for transaction ID 1: Account is locked"
        );
        assert_err!(
            chargeback_result,
            "Failed to apply chargeback for transaction ID 1: Account is locked"
        );

        Ok(())
    }

    #[test]
    fn skips_applying_deposit_subsequently() -> Result<()> {
        let client_id = 1;
        let mut client_account = ClientAccount::new(client_id);

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Deposit(Deposit {
                amount: dec!(12.5555),
            }),
        })?;

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Deposit(Deposit {
                amount: dec!(12.5555),
            }),
        })?;

        assert_eq!(dec!(12.5555), client_account.available_balance);
        assert_eq!(dec!(0), client_account.held_balance);
        assert_eq!(dec!(12.5555), client_account.total_balance);

        Ok(())
    }

    #[test]
    fn skips_applying_withdrawal_subsequently() -> Result<()> {
        let client_id = 1;
        let mut client_account = ClientAccount::new(client_id);

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Deposit(Deposit {
                amount: dec!(12.5555),
            }),
        })?;

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 2,
            action: TransactionAction::Withdrawal(Withdrawal {
                amount: dec!(12.5555),
            }),
        })?;

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 2,
            action: TransactionAction::Withdrawal(Withdrawal {
                amount: dec!(12.5555),
            }),
        })?;

        assert_eq!(dec!(0), client_account.available_balance);
        assert_eq!(dec!(0), client_account.held_balance);
        assert_eq!(dec!(0), client_account.total_balance);

        Ok(())
    }

    #[test]
    fn skips_applying_dispute_to_unknown_transaction() -> Result<()> {
        let client_id = 1;
        let mut client_account = ClientAccount::new(client_id);

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Deposit(Deposit {
                amount: dec!(12.5555),
            }),
        })?;

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 100,
            action: TransactionAction::Dispute,
        })?;

        assert_eq!(dec!(12.5555), client_account.available_balance);
        assert_eq!(dec!(0), client_account.held_balance);
        assert_eq!(dec!(12.5555), client_account.total_balance);

        Ok(())
    }

    #[test]
    fn skips_applying_dispute_to_already_disputed_transaction() -> Result<()> {
        let client_id = 1;
        let mut client_account = ClientAccount::new(client_id);

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Deposit(Deposit {
                amount: dec!(12.5555),
            }),
        })?;

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Dispute,
        })?;

        client_account.apply_transaction(Transaction {
            client_id,
            transaction_id: 1,
            action: TransactionAction::Dispute,
        })?;

        assert_eq!(dec!(0), client_account.available_balance);
        assert_eq!(dec!(12.5555), client_account.held_balance);
        assert_eq!(dec!(12.5555), client_account.total_balance);

        Ok(())
    }
}
