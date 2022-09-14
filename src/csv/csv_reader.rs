use std::{fs::File, io::BufReader};

use anyhow::{Error, Result};
use csv::Reader;

pub fn open_csv_reader(path: &str) -> Result<Reader<BufReader<File>>> {
    let file = File::open(path)
        .map_err(|err| Error::msg(format!("Failed to open CSV at path {}: {}", path, err)))?;
    let buffered_reader = BufReader::new(file);
    Ok(Reader::from_reader(buffered_reader))
}
