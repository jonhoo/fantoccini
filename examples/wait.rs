use fantoccini::elements::Element;
use fantoccini::wait::{Condition, Predicate};
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

    // You can wait on anything that implements `WaitCondition`

    // Wait for a URL
    let _: () = client
        .wait()
        .on(url::Url::parse("https://www.rust-lang.org/")?)
        .await?;

    // Wait for a locator, and get back the element.
    let _: Element = client
        .wait()
        .on(Locator::Css(
            r#"a.button-download[href="/learn/get-started"]"#,
        ))
        .await?;

    // By default it will time-out after 30 seconds and check every 250 milliseconds.
    // However, you can change this.

    let _: Element = client
        .wait()
        .at_most(Duration::from_secs(5))
        .every(Duration::from_millis(100))
        .on(Locator::Css(
            r#"a.button-download[href="/learn/get-started"]"#,
        ))
        .await?;

    // You can also use closures for more custom checks. However, in order to deal with
    // async traits and lifetimes, they need to be wrapped in a newtype in order to implement
    // the WaitCondition trait.

    // Wait for a condition (returning a value)
    let _: String = client
        .wait()
        .on(Condition(|client| {
            Box::pin(async move { Ok(client.get_ua().await?) })
        }))
        .await?;

    // Wait for a condition, using a dedicated method to avoid using the newtype.
    let _: String = client
        .wait()
        .on_condition(|client| Box::pin(async move { Ok(client.get_ua().await?) }))
        .await?;

    // Instead of a condition, returning a value, you can also wait for a boolean outcome.

    // Wait for a predicate (true or false)
    let _: () = client
        .wait()
        .on(Predicate(|client| {
            Box::pin(async move { Ok(client.source().await?.contains("Rust")) })
        }))
        .await?;

    // Wait for a predicate (true or false), using a dedicated method.
    let _: () = client
        .wait()
        .on_predicate(|client| Box::pin(async move { Ok(client.source().await?.contains("Rust")) }))
        .await?;

    // Then close the browser window.
    client.close().await?;

    // done
    Ok(())
}
