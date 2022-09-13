use std::fs::File;

use anyhow::{Error, Result};
use csv::Reader;

pub fn open_csv_reader(path: &str) -> Result<Reader<File>> {
    let file = File::open(path)
        .map_err(|err| Error::msg(format!("Failed to open CSV at path {}: {}", path, err)))?;
    Ok(Reader::from_reader(file))
}
