use fantoccini::{Client, Locator};
use std::time::Duration;
use tokio::time::delay_for;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // expects WebDriver instance to be listening at port 4444
    let mut client = Client::new("http://localhost:4444")
        .await
        .expect("failed to connect to WebDriver");

    client.goto("https://www.rust-lang.org/").await?;
    // delay_for is an artificial delay only used to see the browsers actions and not necessary
    // in your own code
    delay_for(Duration::from_millis(3000)).await;

    let get_started_button =
        r#"//a[@class="button button-download ph4 mt0 w-100" and @href="/learn/get-started"]"#;
    let element = client.find(Locator::XPath(get_started_button)).await?;
    element.click().await?;
    delay_for(Duration::from_millis(3000)).await;

    let try_without_installing_button =
        r#"//a[@class="button button-secondary" and @href="https://play.rust-lang.org/"]"#;
    let element = client
        .find(Locator::XPath(try_without_installing_button))
        .await?;
    element.click().await?;
    delay_for(Duration::from_millis(3000)).await;

    let play_rust_lang_run_button = r#"//div[@class="segmented-button"]/button[1]"#;
    let element = client
        .find(Locator::XPath(play_rust_lang_run_button))
        .await?;
    element.click().await?;
    delay_for(Duration::from_millis(6000)).await;

    Ok(())
}
