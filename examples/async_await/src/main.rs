use fantoccini::{Client, Locator};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to webdriver instance that is listening on port 9515
    let mut c = Client::new("http://localhost:4444").await?;

    // Go to wikipedia page
    c.goto("https://wikipedia.org").await?;

    // Find search form and it's input by their ids
    let search_form = c.form(Locator::Id("search-form")).await?;
    let mut search_input = c.find(Locator::Id("searchInput")).await?;

    // Type into search input field
    search_input.send_keys("Rust programming language").await?;
    // Submit form
    search_form.submit().await?;

    Ok(())
}
