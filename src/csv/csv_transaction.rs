use anyhow::{ensure, Error, Result};
use csv::StringRecord;
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::domain::{
    client_account::ClientId,
    transaction::{Deposit, Transaction, TransactionAction, TransactionId, Withdrawal},
};

#[derive(Debug, Deserialize)]
pub struct CsvTransaction {
    pub transaction_type: String,
    pub client_id: ClientId,
    pub transaction_id: TransactionId,
    pub amount: Decimal,
}

impl CsvTransaction {
    pub fn from_string_record(mut record: StringRecord) -> Result<CsvTransaction> {
        record.trim();
        record
            .deserialize::<CsvTransaction>(None)
            .map_err(|err| Error::msg(format!("Failed to deserialize CSV transaction: {}", err)))
    }
    pub fn to_transaction(self) -> Result<Transaction> {
        let CsvTransaction {
            transaction_type,
            client_id,
            transaction_id,
            amount,
        } = self;
        match transaction_type.as_str() {
            "deposit" => {
                let transaction = Transaction {
                    client_id,
                    transaction_id,
                    action: TransactionAction::Deposit(Deposit { amount }),
                };
                ensure!(
                    amount > Decimal::ZERO,
                    "Failed to read {}: Amount is negative or zero",
                    transaction.to_string()
                );
                Ok(transaction)
            }
            "withdrawal" => {
                let transaction = Transaction {
                    client_id,
                    transaction_id,
                    action: TransactionAction::Withdrawal(Withdrawal { amount }),
                };
                ensure!(
                    amount > Decimal::ZERO,
                    "Failed to read {}: Amount is negative or zero",
                    transaction.to_string()
                );
                Ok(transaction)
            }
            "dispute" => Ok(Transaction {
                client_id,
                transaction_id,
                action: TransactionAction::Dispute,
            }),
            "resolve" => Ok(Transaction {
                client_id,
                transaction_id,
                action: TransactionAction::Resolve,
            }),
            "chargeback" => Ok(Transaction {
                client_id,
                transaction_id,
                action: TransactionAction::Chargeback,
            }),
            _ => Err(Error::msg(format!(
                "Failed to deserialize transaction. Unknown type {}",
                transaction_type
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CsvTransaction;
    use crate::assert_err::assert_err;
    use anyhow::Result;
    use rust_decimal_macros::dec;

    #[test]
    fn fails_to_read_deposit_with_zero_amount() -> Result<()> {
        assert_err!(
            CsvTransaction::to_transaction(CsvTransaction {
                transaction_type: "deposit".to_string(),
                client_id: 1,
                transaction_id: 1,
                amount: dec!(0),
            }),
            "Failed to read deposit with transaction ID 1: Amount is negative or zero"
        );
        Ok(())
    }

    #[test]
    fn fails_to_read_deposit_with_negative_amount() -> Result<()> {
        assert_err!(
            CsvTransaction::to_transaction(CsvTransaction {
                transaction_type: "deposit".to_string(),
                client_id: 1,
                transaction_id: 1,
                amount: dec!(-1),
            }),
            "Failed to read deposit with transaction ID 1: Amount is negative or zero"
        );
        Ok(())
    }

    #[test]
    fn fails_to_read_withdrawal_with_zero_amount() -> Result<()> {
        assert_err!(
            CsvTransaction::to_transaction(CsvTransaction {
                transaction_type: "withdrawal".to_string(),
                client_id: 1,
                transaction_id: 1,
                amount: dec!(0),
            }),
            "Failed to read withdrawal with transaction ID 1: Amount is negative or zero"
        );
        Ok(())
    }

    #[test]
    fn fails_to_read_withdrawal_with_negative_amount() -> Result<()> {
        assert_err!(
            CsvTransaction::to_transaction(CsvTransaction {
                transaction_type: "withdrawal".to_string(),
                client_id: 1,
                transaction_id: 1,
                amount: dec!(-1),
            }),
            "Failed to read withdrawal with transaction ID 1: Amount is negative or zero"
        );
        Ok(())
    }
}
