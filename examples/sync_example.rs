use fantoccini::error::CmdError;
use fantoccini::sync::{Client};
use std::error::Error;

// fn navigate_rustlang<T>() -> Result<T, Box<dyn Error>> {
//     let client = Client::new("http://localhost:4444")?;

//     client.goto("https://www.rust-lang.org/")
// }
// fn main() -> Result<T, Box<dyn Error>> {
fn main() {
    // navigate_rustlang().unwrap();
    let mut client = Client::new("http://localhost:4444").unwrap();

    client.goto("https://www.rust-lang.org/").unwrap();

}