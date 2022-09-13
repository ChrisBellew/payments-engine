use std::collections::{hash_map::Entry, HashMap};

use super::transaction::{Transaction, TransactionId};
use crate::domain::transaction::{Deposit, TransactionAction, Withdrawal};
use anyhow::{Error, Result};
use rust_decimal::Decimal;
use serde::Serialize;

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
        let Transaction {
            transaction_id,
            action,
            ..
        } = transaction;

        match self.last_transaction_id {
            Some(last_transaction_id) => {
                if last_transaction_id >= transaction_id {
                    return Err(Error::msg(format!(
                        "Failed to process out of order or duplicate transaction with ID: {}",
                        transaction_id
                    )));
                }
            }
            _ => (),
        }

        match action {
            TransactionAction::Deposit(deposit) => self.apply_deposit(transaction_id, deposit),
            TransactionAction::Withdrawal(withdrawal) => self.apply_withdrawal(withdrawal),
            TransactionAction::Dispute => self.apply_dispute(transaction_id),
            TransactionAction::Resolve => self.apply_resolve(transaction_id),
            TransactionAction::Chargeback => self.apply_chargeback(transaction_id),
        }
        .map_err(|err| {
            Error::msg(format!(
                "Failed to apply transaction with ID {}: {}",
                transaction_id, err
            ))
        })?;

        self.last_transaction_id = Some(transaction_id);

        Ok(())
    }

    fn apply_deposit(&mut self, transaction_id: u32, deposit: Deposit) -> Result<()> {
        let available_balance = self
            .available_balance
            .checked_add(deposit.amount)
            .ok_or(Error::msg("Deposit would cause available balance overflow"))?;

        let total_balance = self
            .total_balance
            .checked_add(deposit.amount)
            .ok_or(Error::msg("Deposit would cause total balance overflow"))?;

        self.applied_deposits.insert(transaction_id, deposit);
        self.available_balance = available_balance;
        self.total_balance = total_balance;

        Ok(())
    }

    fn apply_withdrawal(&mut self, withdrawal: Withdrawal) -> Result<()> {
        if withdrawal.amount.gt(&self.available_balance) {
            return Err(Error::msg("Insufficient available balance for withdrawal"));
        }

        let available_balance = self
            .available_balance
            .checked_sub(withdrawal.amount)
            .ok_or(Error::msg(
                "Withdrawal would cause available balance underflow",
            ))?;

        let total_balance = self
            .total_balance
            .checked_sub(withdrawal.amount)
            .ok_or(Error::msg("Withdrawal would cause total balance underflow"))?;

        self.available_balance = available_balance;
        self.total_balance = total_balance;

        Ok(())
    }

    fn apply_dispute(&mut self, transaction_id: u32) -> Result<()> {
        match self.applied_deposits.entry(transaction_id) {
            Entry::Occupied(entry) => {
                let deposit = entry.get();

                let available_balance =
                    self.available_balance
                        .checked_sub(deposit.amount)
                        .ok_or(Error::msg(
                            "Dispute would cause available balance underflow",
                        ))?;

                let held_balance = self
                    .held_balance
                    .checked_add(deposit.amount)
                    .ok_or(Error::msg("Deposit would cause held balance overflow"))?;

                self.disputed_deposits
                    .insert(transaction_id, entry.remove());
                self.available_balance = available_balance;
                self.held_balance = held_balance;
            }
            Entry::Vacant(_) => (),
        };

        Ok(())
    }

    fn apply_resolve(&mut self, transaction_id: u32) -> Result<()> {
        match self.disputed_deposits.entry(transaction_id) {
            Entry::Occupied(entry) => {
                let deposit = entry.get();

                let available_balance = self
                    .available_balance
                    .checked_add(deposit.amount)
                    .ok_or(Error::msg("Resolve would cause available balance overflow"))?;

                let held_balance = self
                    .held_balance
                    .checked_sub(deposit.amount)
                    .ok_or(Error::msg("Resolve would cause held balance underflow"))?;

                self.applied_deposits.insert(transaction_id, entry.remove());
                self.available_balance = available_balance;
                self.held_balance = held_balance;
            }
            Entry::Vacant(_) => (),
        };

        Ok(())
    }

    fn apply_chargeback(&mut self, transaction_id: u32) -> Result<()> {
        match self.disputed_deposits.entry(transaction_id) {
            Entry::Occupied(entry) => {
                let deposit = entry.get();

                let held_balance = self
                    .held_balance
                    .checked_sub(deposit.amount)
                    .ok_or(Error::msg("Chargeback would cause held balance underflow"))?;

                let total_balance = self
                    .total_balance
                    .checked_sub(deposit.amount)
                    .ok_or(Error::msg("Chargeback would cause total balance underflow"))?;

                self.chargedback_deposits
                    .insert(transaction_id, entry.remove());
                self.held_balance = held_balance;
                self.total_balance = total_balance;
            }
            Entry::Vacant(_) => (),
        };

        Ok(())
    }
}
