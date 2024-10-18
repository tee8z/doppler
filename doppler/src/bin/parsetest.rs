use std::{fs, io::Error, path::PathBuf};

use clap::{arg, Parser as CliParser};
use doppler::{get_absolute_path, DopplerParser, Rule};
use pest::Parser;

#[derive(CliParser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Sets path to doppler file
    #[arg(short, long, value_name = "FILE")]
    file: PathBuf,
}

fn main() -> Result<(), Error> {
    let cli: Cli = Cli::parse();
    let filepath = get_doppler_file_path(&cli)?;
    let contents = fs::read_to_string(filepath).expect("file read error");
    print!("read file content");
    let parsed = DopplerParser::parse(Rule::page, &contents).expect("parse error");
    println!("{:#?}", parsed);
    println!("Successfully parsed file with grammar");
    Ok(())
}

pub fn get_doppler_file_path(cli: &Cli) -> Result<String, Error> {
    let file_path = cli.file.to_string_lossy();
    let full_path = get_absolute_path(&file_path).unwrap();
    Ok(full_path.to_string_lossy().to_string())
}
