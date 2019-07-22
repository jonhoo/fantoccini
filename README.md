# fantoccini

[![Crates.io](https://img.shields.io/crates/v/fantoccini.svg)](https://crates.io/crates/fantoccini)
[![Documentation](https://docs.rs/fantoccini/badge.svg)](https://docs.rs/fantoccini/)
[![Build Status](https://travis-ci.com/jonhoo/fantoccini.svg?branch=master)](https://travis-ci.com/jonhoo/fantoccini)
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
A quick way to get one is to run [`geckodriver`] at the command line. The code also has
partial support for the legacy WebDriver protocol used by `chromedriver` and `ghostdriver`.

The examples will be using `panic!` or `unwrap` generously when errors occur (see `map_err`)
--- you should probably not do that in your code, and instead deal with errors when they occur.
This is particularly true for methods that you *expect* might fail, such as lookups by CSS
selector.

Let's start out clicking around on Wikipedia:

```rust
use fantoccini::{Client, Locator};
use futures::future::Future;
let c = Client::new("http://localhost:4444");

// let's set up the sequence of steps we want the browser to take
tokio::run(
    c
        .map_err(|e| {
            unimplemented!("failed to connect to WebDriver: {:?}", e)
        })
        .and_then(|c| {
            // first, go to the Wikipedia page for Foobar
            c.goto("https://en.wikipedia.org/wiki/Foobar")
        })
        .and_then(|mut c| c.current_url().map(move |url| (c, url)))
        .and_then(|(mut c, url)| {
            assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");
            // click "Foo (disambiguation)"
            c.find(Locator::Css(".mw-disambig"))
        })
        .and_then(|e| e.click())
        .and_then(|mut c| {
            // click "Foo Lake"
            c.find(Locator::LinkText("Foo Lake"))
        })
        .and_then(|e| e.click())
        .and_then(|mut c| c.current_url())
        .and_then(|url| {
            assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foo_Lake");
            Ok(())
        })
        .map_err(|e| {
            panic!("a WebDriver command failed: {:?}", e);
        })
);
```

How did we get to the Foobar page in the first place? We did a search!
Let's make the program do that for us instead:

```rust
// -- snip wrapper code --
// go to the Wikipedia frontpage this time
c.goto("https://www.wikipedia.org/")
    .and_then(|mut c| {
        // find the search form
        c.form(Locator::Css("#search-form"))
    })
    .and_then(|mut f| {
        // fill it out
        f.set_by_name("search", "foobar")
    })
    .and_then(|f| {
        // and submit it
        f.submit()
    })
    // we should now have ended up in the rigth place
    .and_then(|mut c| c.current_url())
    .and_then(|url| {
        assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");
        Ok(())
    })
// -- snip wrapper code --
```

What if we want to download a raw file? Fantoccini has you covered:

```rust
// -- snip wrapper code --
// go back to the frontpage
c.goto("https://www.wikipedia.org/")
    .and_then(|mut c| {
        // find the source for the Wikipedia globe
        c.find(Locator::Css("img.central-featured-logo"))
    })
    .and_then(|mut img| {
        img.attr("src")
            .map(move |src| (img, src.expect("image should have a src")))
    })
    .and_then(move |(img, src)| {
        // now build a raw HTTP client request (which also has all current cookies)
        img.client().raw_client_for(fantoccini::Method::GET, &src)
    })
    .and_then(|raw| {
        use futures::Stream;
        // we then read out the image bytes
        raw.into_body().map_err(fantoccini::error::CmdError::from).fold(
            Vec::new(),
            |mut pixels, chunk| {
                pixels.extend(&*chunk);
                futures::future::ok::<Vec<u8>, fantoccini::error::CmdError>(pixels)
            },
        )
    })
    .and_then(|pixels| {
        // and voilla, we now have the bytes for the Wikipedia logo!
        assert!(pixels.len() > 0);
        println!("Wikipedia logo is {}b", pixels.len());
        Ok(())
    })
// -- snip wrapper code --
```

For more examples, take a look at the `examples/` directory.

[WebDriver protocol]: https://www.w3.org/TR/webdriver/
[CSS selectors]: https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Selectors
[powerful]: https://developer.mozilla.org/en-US/docs/Web/CSS/Pseudo-classes
[operators]: https://developer.mozilla.org/en-US/docs/Web/CSS/Attribute_selectors
[WebDriver compatible]: https://github.com/Fyrd/caniuse/issues/2757#issuecomment-304529217
[`geckodriver`]: https://github.com/mozilla/geckodriver

## CI testing

Big thanks to [Sauce Labs][homepage] for providing us with free cross-browser
testing!

[![Sauce Test Status](https://saucelabs.com/browser-matrix/fantoccini.svg)](https://saucelabs.com/u/fantoccini)

[homepage]: https://saucelabs.com
