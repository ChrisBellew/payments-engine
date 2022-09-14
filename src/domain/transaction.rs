use super::client_account::ClientId;
use rust_decimal::Decimal;

pub type TransactionId = u32;

#[derive(Debug)]
pub struct Transaction {
    pub client_id: ClientId,
    pub transaction_id: TransactionId,
    pub action: TransactionAction,
}

impl Transaction {
    pub fn to_string(&self) -> String {
        match self.action {
            TransactionAction::Deposit(_) => {
                format!("deposit with transaction ID {}", self.transaction_id)
            }
            TransactionAction::Withdrawal(_) => {
                format!("withdrawal with transaction ID {}", self.transaction_id)
            }
            TransactionAction::Dispute => {
                format!("dispute for transaction ID {}", self.transaction_id)
            }
            TransactionAction::Resolve => {
                format!("resolve for transaction ID {}", self.transaction_id)
            }
            TransactionAction::Chargeback => {
                format!("chargeback for transaction ID {}", self.transaction_id)
            }
        }
    }
}

#[derive(Debug)]
pub enum TransactionAction {
    Deposit(Deposit),
    Withdrawal(Withdrawal),
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug)]
pub struct Deposit {
    pub amount: Decimal,
}

#[derive(Debug)]
pub struct Withdrawal {
    pub amount: Decimal,
}
