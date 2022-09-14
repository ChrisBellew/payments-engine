mod assert_err;
mod csv;
mod domain;

use crate::csv::csv_reader::open_csv_reader;
use crate::csv::csv_transaction::CsvTransaction;
use ::csv::Writer;
use anyhow::{Error, Result};
use domain::client_account::{ClientAccount, ClientId};
use std::{collections::HashMap, env, io::stdout};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let csv_path = args.get(1).ok_or(Error::msg(
        "Missing CSV path argument. Example: cargo run -- transactions.csv",
    ))?;

    let mut reader = open_csv_reader(csv_path)?;

    let mut client_accounts: HashMap<ClientId, ClientAccount> = HashMap::new();

    for csv_record in reader.records() {
        let record = csv_record.expect("Failed to parse CSV line");
        let csv_transaction = CsvTransaction::from_string_record(record)?;
        let transaction = csv_transaction.to_transaction()?;

        let client_account = client_accounts
            .entry(transaction.client_id)
            .or_insert(ClientAccount::new(transaction.client_id));

        client_account.apply_transaction(transaction)?;
    }

    let mut writer = Writer::from_writer(stdout());

    for account in client_accounts.into_values() {
        writer.serialize(account)?;
    }
    writer.flush()?;

    Ok(())
}
