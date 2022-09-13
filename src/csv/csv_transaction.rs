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
            "deposit" => Ok(Transaction {
                client_id,
                transaction_id,
                action: TransactionAction::Deposit(Deposit { amount }),
            }),
            "withdrawal" => Ok(Transaction {
                client_id,
                transaction_id,
                action: TransactionAction::Withdrawal(Withdrawal { amount }),
            }),
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
