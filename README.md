# fantoccini

[![Crates.io](https://img.shields.io/crates/v/fantoccini.svg)](https://crates.io/crates/fantoccini)
[![Documentation](https://docs.rs/fantoccini/badge.svg)](https://docs.rs/fantoccini/)
[![codecov](https://codecov.io/gh/jonhoo/fantoccini/branch/master/graph/badge.svg?token=NteBJ0F7Ok)](https://codecov.io/gh/jonhoo/fantoccini)
[![Gitter chat](https://badges.gitter.im/Join%20Chat.svg)](https://gitter.im/fantoccini-rs/Lobby)

A high-level API for programmatically interacting with web pages through WebDriver.

This crate uses the [WebDriver protocol] to drive a conforming (potentially headless) browser
through relatively high-level operations such as "click this element", "submit this form", etc.

Most interactions are driven by using [CSS selectors]. With most WebDriver-compatible browser
being fairly recent, the more expressive levels of the CSS standard are also supported, giving
fairly [powerful] [operators].

Forms are managed by first calling `Client::form`, and then using the methods on `Form` to
manipulate the form's fields and eventually submitting it.

For low-level access to the page, `Client::source` can be used to fetch the full page HTML
source code, and `Client::raw_client_for` to build a raw HTTP request for a particular URL.

## Examples

These examples all assume that you have a [WebDriver compatible] process running on port 4444.
A quick way to get one is to run [`geckodriver`] at the command line.

Let's start out clicking around on Wikipedia:

```rust
use fantoccini::{ClientBuilder, Locator};

// let's set up the sequence of steps we want the browser to take
#[tokio::main]
async fn main() -> Result<(), fantoccini::error::CmdError> {
    let c = ClientBuilder::native().connect("http://localhost:4444").await.expect("failed to connect to WebDriver");

    // first, go to the Wikipedia page for Foobar
    c.goto("https://en.wikipedia.org/wiki/Foobar").await?;
    let url = c.current_url().await?;
    assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");

    // click "Foo (disambiguation)"
    c.find(Locator::Css(".mw-disambig")).await?.click().await?;

    // click "Foo Lake"
    c.find(Locator::LinkText("Foo Lake")).await?.click().await?;

    let url = c.current_url().await?;
    assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foo_Lake");

    c.close().await
}
```

How did we get to the Foobar page in the first place? We did a search!
Let's make the program do that for us instead:

```rust
// -- snip wrapper code --
// go to the Wikipedia frontpage this time
c.goto("https://www.wikipedia.org/").await?;
// find the search form, fill it out, and submit it
let f = c.form(Locator::Css("#search-form")).await?;
f.set_by_name("search", "foobar").await?
 .submit().await?;

// we should now have ended up in the right place
let url = c.current_url().await?;
assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");

// -- snip wrapper code --
```

What if we want to download a raw file? Fantoccini has you covered:

```rust
// -- snip wrapper code --
// go back to the frontpage
c.goto("https://www.wikipedia.org/").await?;
// find the source for the Wikipedia globe
let img = c.find(Locator::Css("img.central-featured-logo")).await?;
let src = img.attr("src").await?.expect("image should have a src");
// now build a raw HTTP client request (which also has all current cookies)
let raw = img.client().raw_client_for(fantoccini::Method::GET, &src).await?;

// we then read out the image bytes
use futures_util::TryStreamExt;
let pixels = raw
    .into_body()
    .try_fold(Vec::new(), |mut data, chunk| async move {
        data.extend_from_slice(&chunk);
        Ok(data)
    })
    .await
    .map_err(fantoccini::error::CmdError::from)?;
// and voilà, we now have the bytes for the Wikipedia logo!
assert!(pixels.len() > 0);
println!("Wikipedia logo is {}b", pixels.len());

// -- snip wrapper code --
```

For more examples, take a look at the `examples/` directory.

# Contributing to fantoccini

The following information applies only to developers interested in contributing
to this project. If you simply want to use it to automate web browsers you can
skip this section.

## How to run tests

The tests assume that you have [`chromedriver`] and [`geckodriver`] already running on your system.
You can download them using the links above. Then run them from separate tabs in your terminal.
They will stay running until terminated with Ctrl+C or until the terminal session is closed.

Then run `cargo test` from this project directory.

[WebDriver protocol]: https://www.w3.org/TR/webdriver/
[CSS selectors]: https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Selectors
[powerful]: https://developer.mozilla.org/en-US/docs/Web/CSS/Pseudo-classes
[operators]: https://developer.mozilla.org/en-US/docs/Web/CSS/Attribute_selectors
[WebDriver compatible]: https://github.com/Fyrd/caniuse/issues/2757#issuecomment-304529217
[`geckodriver`]: https://github.com/mozilla/geckodriver
[`chromedriver`]: https://chromedriver.chromium.org/downloads
