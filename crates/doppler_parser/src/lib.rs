
use std::{io::Error, path::PathBuf};

use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "src/grammar.pest"]
pub struct DopplerParser;

pub fn get_doppler_file_path(file_path: &str) -> Result<String, Error> {
    let full_path = get_absolute_path(file_path).unwrap();
    Ok(full_path.to_string_lossy().to_string())
}

pub fn get_absolute_path(relative_path: &str) -> Result<PathBuf, Error> {
    let current_dir = std::env::current_dir()?;
    let absolute_path = current_dir.join(relative_path);

    Ok(absolute_path)
}