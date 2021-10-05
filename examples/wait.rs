use fantoccini::elements::Element;
use fantoccini::{ClientBuilder, Locator};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to webdriver instance that is listening on port 4444
    let mut client = ClientBuilder::native()
        .connect("http://localhost:4444")
        .await?;

    // Go to the Rust website.
    client.goto("https://www.rust-lang.org/").await?;

    // The explicit return types in following code is just to illustrate the type returned.
    // You can omit them in your code.

    // You can wait on anything that implements `WaitCondition`

    // Wait for a URL
    let _: () = client
        .wait()
        .for_url(url::Url::parse("https://www.rust-lang.org/")?)
        .await?;

    // Wait for a locator, and get back the element.
    let _: Element = client
        .wait()
        .for_element(Locator::Css(
            r#"a.button-download[href="/learn/get-started"]"#,
        ))
        .await?;

    // By default it will time-out after 30 seconds and check every 250 milliseconds.
    // However, you can change this.

    let _: Element = client
        .wait()
        .at_most(Duration::from_secs(5))
        .every(Duration::from_millis(100))
        .for_element(Locator::Css(
            r#"a.button-download[href="/learn/get-started"]"#,
        ))
        .await?;

    // Then close the browser window.
    client.close().await?;

    // done
    Ok(())
}
