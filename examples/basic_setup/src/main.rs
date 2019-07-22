use fantoccini::error::CmdError;
use fantoccini::{Client, Locator};
use futures::future::Future;
use tokio;

fn main() {
    // expects WebDriver instance to be listening at port 4444
    let client = Client::new("http://localhost:9515");
    let get_started_button =
        r#"//a[@class="button button-download ph4 mt0 w-100" and @href="/learn/get-started"]"#;
    let try_without_installing_button =
        r#"//a[@class="button button-secondary" and @href="https://play.rust-lang.org/"]"#;
    let play_rust_lang_run_button = r#"//div[@class="segmented-button"]/button[1]"#;

    let rust_lang = client
        .map_err(|error| unimplemented!("failed to connect to WebDriver: {:?}", error))
        .and_then(|client| client.goto("https://www.rust-lang.org/"))
        .and_then(|client| client_wait(client, 3000))
        .and_then(move |client| client.wait_for_find(Locator::XPath(get_started_button)))
        .and_then(|element| element.click())
        .and_then(|client| client_wait(client, 3000))
        .and_then(move |client| client.wait_for_find(Locator::XPath(try_without_installing_button)))
        .and_then(|element| element.click())
        .and_then(|client| client_wait(client, 3000))
        .and_then(move |client| {
            client.wait_for_find(Locator::XPath(play_rust_lang_run_button))
        })
        .and_then(|element| element.click())
        .and_then(|client| client_wait(client, 6000))
        .map(|_| ())
        .map_err(|error| panic!("a WebDriver command failed: {:?}", error));

    tokio::run(rust_lang);
}

// helper function to delay the client
// so that the example doesn't execute too quickly
fn client_wait(client: Client, delay: u64) -> impl Future<Item = Client, Error = CmdError> {
    use std::time::{Duration, Instant};
    use tokio::timer::Delay;

    Delay::new(Instant::now() + Duration::from_millis(delay))
        .and_then(|_| Ok(client))
        .map_err(|error| panic!("client failed to wait with error: {:?}", error))
}
