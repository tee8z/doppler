use std::fs;

use doppler::{DopplerParser, Rule};
use pest::Parser;

fn main() {
    let contents = fs::read_to_string("./doppler_files/eclair_lnd_only_actions.doppler")
        .expect("file read error");
    print!("read file content");
    let parsed = DopplerParser::parse(Rule::page, &contents).expect("parse error");
    println!("{:#?}", parsed);
}
