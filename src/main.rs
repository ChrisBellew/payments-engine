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

    let client_accounts = process_csv(&csv_path)?;

    let mut writer = Writer::from_writer(stdout());

    writer.write_record(&["client", "available", "held", "total", "locked"])?;
    for account in client_accounts {
        writer.write_record(&[
            account.client_id.to_string(),
            format!("{:.4}", account.available_balance),
            format!("{:.4}", account.held_balance),
            format!("{:.4}", account.total_balance),
            account.locked.to_string(),
        ])?;
    }

    writer.flush()?;

    Ok(())
}

fn process_csv(csv_path: &str) -> Result<Vec<ClientAccount>> {
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

    Ok(client_accounts.into_values().collect())
}
#[cfg(test)]
mod tests {
    use std::{fs::File, io::BufWriter};

    use anyhow::Result;
    use csv::Writer;
    use rust_decimal_macros::dec;
    use stopwatch::Stopwatch;

    use crate::process_csv;

    #[test]
    #[ignore] // Comment this to test performance of a large file
    fn test_large_file() -> Result<()> {
        let csv_path = "/media/chris/x/large-file.csv";
        let mut writer = Writer::from_writer(BufWriter::new(File::create(csv_path)?));
        let num_events = 1_000_000_000;
        let num_deposits = num_events / 4;

        writer.write_record(&["type", "client", "tx", "amount"])?;
        for i in (0..num_deposits).step_by(2) {
            let amount = format!("{:.4}", dec!(123.45));
            writer.write_record(&["deposit", "1", &i.to_string(), &amount])?;
            writer.write_record(&["dispute", "1", &i.to_string(), &""])?;
            writer.write_record(&["resolve", "1", &i.to_string(), &""])?;
            writer.write_record(&["withdrawal", "1", &(i + 1).to_string(), &amount])?;
        }

        writer.flush()?;

        let stopwatch = Stopwatch::start_new();
        let client_accounts = process_csv(&csv_path)?;
        assert_eq!(1, client_accounts[0].client_id);
        assert_eq!(dec!(0), client_accounts[0].available_balance);
        assert_eq!(dec!(0), client_accounts[0].held_balance);
        assert_eq!(dec!(0), client_accounts[0].total_balance);
        println!(
            "Processed {} events in {} ms",
            num_events,
            stopwatch.elapsed_ms()
        );

        Ok(())
    }
}
