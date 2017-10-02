# fantoccini

[![Crates.io](https://img.shields.io/crates/v/fantoccini.svg)](https://crates.io/crates/fantoccini)
[![Documentation](https://docs.rs/fantoccini/badge.svg)](https://docs.rs/fantoccini/)
[![Build Status](https://travis-ci.org/jonhoo/fantoccini.svg?branch=master)](https://travis-ci.org/jonhoo/fantoccini)

A high-level API for programmatically interacting with web pages through WebDriver.

This crate uses the [WebDriver protocol] to drive a conforming (potentially headless) browser
through relatively high-level operations such as "click this element", "submit this form", etc.
It is currently nightly-only, but this will change once
[`conservative_impl_trait`](https://github.com/rust-lang/rust/issues/34511) lands in stable.

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

The examples will be using `unwrap` generously --- you should probably not do that in your
code, and instead deal with errors when they occur. This is particularly true for methods that
you *expect* might fail, such as lookups by CSS selector.

Let's start out clicking around on Wikipedia:

```rust
let mut core = tokio_core::reactor::Core::new().unwrap();
let (c, fin) = Client::new("http://localhost:4444", &core.handle());
let c = core.run(c).unwrap();

{
    // we want to have a reference to c so we can use it in the and_thens below
    let c = &c;

    // now let's set up the sequence of steps we want the browser to take
    // first, go to the Wikipedia page for Foobar
    let f = c.goto("https://en.wikipedia.org/wiki/Foobar")
        .and_then(move |_| c.current_url())
        .and_then(move |url| {
            assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");
            // click "Foo (disambiguation)"
            c.by_selector(".mw-disambig")
        })
        .and_then(|e| e.click())
        .and_then(move |_| {
            // click "Foo Lake"
            c.by_link_text("Foo Lake")
        })
        .and_then(|e| e.click())
        .and_then(move |_| c.current_url())
        .and_then(|url| {
            assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foo_Lake");
            Ok(())
        });

    // and set the browser off to do those things
    core.run(f).unwrap();
}

// drop the client to delete the browser session
drop(c);
// and wait for cleanup to finish
core.run(fin).unwrap();
```

How did we get to the Foobar page in the first place? We did a search!
Let's make the program do that for us instead:

```rust
// -- snip wrapper code --
// go to the Wikipedia frontpage this time
c.goto("https://www.wikipedia.org/")
    .and_then(move |_| {
        // find the search form
        c.form("#search-form")
    })
    .and_then(|f| {
        // fill it out
        f.set_by_name("search", "foobar")
    })
    .and_then(|f| {
        // and submit it
        f.submit()
    })
    // we should now have ended up in the rigth place
    .and_then(move |_| c.current_url())
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
    .and_then(move |_| {
        // find the source for the Wikipedia globe
        c.by_selector("img.central-featured-logo")
    })
    .and_then(|img| {
        img.attr("src")
            .map(|src| src.expect("image should have a src"))
    })
    .and_then(move |src| {
        // now build a raw HTTP client request (which also has all current cookies)
        c.raw_client_for(fantoccini::Method::Get, &src)
    })
    .and_then(|raw| {
        use futures::Stream;
        // we then read out the image bytes
        raw.body().map_err(fantoccini::error::CmdError::from).fold(
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

[WebDriver protocol]: https://www.w3.org/TR/webdriver/
[CSS selectors]: https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Selectors
[powerful]: https://developer.mozilla.org/en-US/docs/Web/CSS/Pseudo-classes
[operators]: https://developer.mozilla.org/en-US/docs/Web/CSS/Attribute_selectors
[WebDriver compatible]: https://github.com/Fyrd/caniuse/issues/2757#issuecomment-304529217
[`geckodriver`]: https://github.com/mozilla/geckodriver
