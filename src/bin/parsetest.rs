use std::fs;

use doppler::{DopplerParser, Rule};
use pest::Parser;

fn main() {
    let contents = fs::read_to_string("parsetest.doppler").expect("file read error");
    let parsed = DopplerParser::parse(Rule::page, &contents).expect("parse error");
    println!("{:#?}", parsed);
}
