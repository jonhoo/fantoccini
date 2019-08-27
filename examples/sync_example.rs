use fantoccini::error::CmdError;
use fantoccini::sync::{Client};
use std::error::Error;

fn main() {
    let mut client = Client::new("http://localhost:4444").unwrap();

    client.goto("https://www.rust-lang.org/").unwrap();

}