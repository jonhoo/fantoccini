use fantoccini::error::CmdError;
use fantoccini::sync::{Client};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut client = Client::new("http://localhost:4444")?;
    client.goto("https://www.rust-lang.org/")?;

    Ok(())
}