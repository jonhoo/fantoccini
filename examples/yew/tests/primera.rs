use fantoccini::{error::CmdError, Client, Locator};

#[fantoccini::test(chrome, firefox)]
async fn scaffolding(_client: Client) -> Result<(), CmdError> {
    Ok(())
}

#[fantoccini::test(chrome)]
async fn hello_world(client: Client) -> Result<(), CmdError> {
    // go to the Wikipedia page for Foobar
    client.goto("http://localhost:8080").await?;
    let h1 = client.find(Locator::XPath("//h1")).await?;
    let text = h1.text().await?;
    assert_eq!(text, "Hello, World!");
    let url = client.current_url().await?;
    assert_eq!(url.as_ref(), "http://localhost:8080/");

    Ok(())
}
