use super::client_account::ClientId;
use rust_decimal::Decimal;

pub type TransactionId = u32;

#[derive(Debug)]
pub struct Transaction {
    pub client_id: ClientId,
    pub transaction_id: TransactionId,
    pub action: TransactionAction,
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
