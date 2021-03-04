//! ## Setup
//!
//! This example assumes you have geckodriver or chromedriver listening at port 4444.
//!
//! You can start the webdriver instance by:
//!
//! ### geckodriver
//!
//! ```text
//! geckodriver --port 4444
//! ```
//!
//! ### chromedriver
//!
//! ```text
//! chromedriver --port=4444
//! ```
//!
//! ## To Run
//!
//! ```
//! cargo run --example basic
//! ```

use fantoccini::{ClientBuilder, Locator};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to webdriver instance that is listening on port 4444
    let mut client = ClientBuilder::native()
        .connect("http://localhost:4444")
        .await?;

    // Go to the Rust website.
    client.goto("https://www.rust-lang.org/").await?;

    // This sleep is just used to make the browser's actions visible.
    sleep(Duration::from_millis(1000)).await;

    // Click the "Get Started" button.
    let button = client
        .find(Locator::Css(
            r#"a.button-download[href="/learn/get-started"]"#,
        ))
        .await?;
    button.click().await?;
    sleep(Duration::from_millis(1000)).await;

    // Click the "Try Rust Without Installing" button (using XPath this time).
    let button = r#"//a[@class="button button-secondary" and @href="https://play.rust-lang.org/"]"#;
    let button = client.find(Locator::XPath(button)).await?;
    button.click().await?;
    sleep(Duration::from_millis(1000)).await;

    // Find the big textarea.
    let mut code_area = client.find(Locator::Css(".ace_text-input")).await?;

    // And write in some code.
    code_area.send_keys("// Hello from Fantoccini\n").await?;
    sleep(Duration::from_millis(1000)).await;

    // Now, let's run it!
    let button = r#"//div[@class="segmented-button"]/button[1]"#;
    let button = client.find(Locator::XPath(button)).await?;
    button.click().await?;

    // Let the user marvel at what we achieved.
    sleep(Duration::from_millis(6000)).await;
    // Then close the browser window.
    client.close().await?;

    Ok(())
}
