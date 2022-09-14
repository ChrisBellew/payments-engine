use anyhow::{Error, Result};
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
    pub amount: Option<Decimal>,
}

impl CsvTransaction {
    pub fn from_string_record(mut record: StringRecord) -> Result<CsvTransaction> {
        record.trim();
        record
            .deserialize::<CsvTransaction>(None)
            .map_err(|err| Error::msg(format!("Failed to deserialize CSV transaction: {}", err)))
    }
    pub fn to_transaction(self) -> Result<Transaction> {
        let transaction_id = self.transaction_id;

        match self.transaction_type.as_str() {
            "deposit" => self.to_deposit(),
            "withdrawal" => self.to_withdrawal(),
            "dispute" => self.to_dispute(),
            "resolve" => self.to_resolve(),
            "chargeback" => self.to_chargeback(),
            _ => Err(Error::msg(format!(
                "Unknown type {}",
                self.transaction_type
            ))),
        }
        .map_err(|err| {
            Error::msg(format!(
                "Failed to read transaction with ID {}: {}",
                transaction_id, err
            ))
        })
    }
    fn to_deposit(self) -> Result<Transaction> {
        let amount = self.assert_positive_amount()?;

        Ok(Transaction {
            client_id: self.client_id,
            transaction_id: self.transaction_id,
            action: TransactionAction::Deposit(Deposit { amount }),
        })
    }
    fn to_withdrawal(self) -> Result<Transaction> {
        let amount = self.assert_positive_amount()?;

        Ok(Transaction {
            client_id: self.client_id,
            transaction_id: self.transaction_id,
            action: TransactionAction::Withdrawal(Withdrawal { amount }),
        })
    }
    fn to_dispute(self) -> Result<Transaction> {
        Ok(Transaction {
            client_id: self.client_id,
            transaction_id: self.transaction_id,
            action: TransactionAction::Dispute,
        })
    }
    fn to_resolve(self) -> Result<Transaction> {
        Ok(Transaction {
            client_id: self.client_id,
            transaction_id: self.transaction_id,
            action: TransactionAction::Resolve,
        })
    }
    fn to_chargeback(self) -> Result<Transaction> {
        Ok(Transaction {
            client_id: self.client_id,
            transaction_id: self.transaction_id,
            action: TransactionAction::Chargeback,
        })
    }
    fn assert_positive_amount(&self) -> Result<Decimal> {
        self.amount
            .ok_or(Error::msg("Amount is missing"))
            .and_then(|amount| {
                if amount > Decimal::ZERO {
                    Ok(amount)
                } else {
                    Err(Error::msg("Amount is negative or zero"))
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::CsvTransaction;
    use crate::assert_err::assert_err;
    use anyhow::Result;
    use rust_decimal_macros::dec;

    #[test]
    fn fails_to_read_deposit_with_missing_amount() -> Result<()> {
        assert_err!(
            CsvTransaction::to_transaction(CsvTransaction {
                transaction_type: "deposit".to_string(),
                client_id: 1,
                transaction_id: 1,
                amount: None,
            }),
            "Failed to read transaction with ID 1: Amount is missing"
        );
        Ok(())
    }

    #[test]
    fn fails_to_read_deposit_with_zero_amount() -> Result<()> {
        assert_err!(
            CsvTransaction::to_transaction(CsvTransaction {
                transaction_type: "deposit".to_string(),
                client_id: 1,
                transaction_id: 1,
                amount: Some(dec!(0)),
            }),
            "Failed to read transaction with ID 1: Amount is negative or zero"
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
                amount: Some(dec!(-1)),
            }),
            "Failed to read transaction with ID 1: Amount is negative or zero"
        );
        Ok(())
    }

    #[test]
    fn fails_to_read_withdrawal_with_missing_amount() -> Result<()> {
        assert_err!(
            CsvTransaction::to_transaction(CsvTransaction {
                transaction_type: "withdrawal".to_string(),
                client_id: 1,
                transaction_id: 1,
                amount: None,
            }),
            "Failed to read transaction with ID 1: Amount is missing"
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
                amount: Some(dec!(0)),
            }),
            "Failed to read transaction with ID 1: Amount is negative or zero"
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
                amount: Some(dec!(-1)),
            }),
            "Failed to read transaction with ID 1: Amount is negative or zero"
        );
        Ok(())
    }
}
