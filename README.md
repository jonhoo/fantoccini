# fantoccini

[![Crates.io](https://img.shields.io/crates/v/fantoccini.svg)](https://crates.io/crates/fantoccini)
[![Documentation](https://docs.rs/fantoccini/badge.svg)](https://docs.rs/fantoccini/)
[![Build Status](https://travis-ci.org/jonhoo/fantoccini.svg?branch=master)](https://travis-ci.org/jonhoo/fantoccini)

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

The examples will be using `unwrap` generously --- you should probably not do that in your
code, and instead deal with errors when they occur. This is particularly true for methods that
you *expect* might fail, such as lookups by CSS selector.

Let's start out clicking around on Wikipedia:

```rust,no_run
# use fantoccini::Client;
let mut c = Client::new("http://localhost:4444").unwrap();
// go to the Wikipedia page for Foobar
c.goto("https://en.wikipedia.org/wiki/Foobar").unwrap();
assert_eq!(c.current_url().unwrap().as_ref(), "https://en.wikipedia.org/wiki/Foobar");
// click "Foo (disambiguation)"
c.by_selector(".mw-disambig").unwrap().click().unwrap();
// click "Foo Lake"
c.by_link_text("Foo Lake").unwrap().click().unwrap();
assert_eq!(c.current_url().unwrap().as_ref(), "https://en.wikipedia.org/wiki/Foo_Lake");
```

How did we get to the Foobar page in the first place? We did a search!
Let's make the program do that for us instead:

```rust,no_run
# use fantoccini::Client;
# let mut c = Client::new("http://localhost:4444").unwrap();
// go to the Wikipedia frontpage this time
c.goto("https://www.wikipedia.org/").unwrap();
// find, fill out, and submit the search form
{
    let mut f = c.form("#search-form").unwrap();
    f.set_by_name("search", "foobar").unwrap();
    f.submit().unwrap();
}
// we should now have ended up in the rigth place
assert_eq!(c.current_url().unwrap().as_ref(), "https://en.wikipedia.org/wiki/Foobar");
```

What if we want to download a raw file? Fantoccini has you covered:

```rust,no_run
# use fantoccini::Client;
# let mut c = Client::new("http://localhost:4444").unwrap();
// go back to the frontpage
c.goto("https://www.wikipedia.org/").unwrap();
// find the source for the Wikipedia globe
let img = c.by_selector("img.central-featured-logo")
    .expect("image should be on page")
    .attr("src")
    .unwrap()
    .expect("image should have a src");
// now build a raw HTTP client request (which also has all current cookies)
let raw = c.raw_client_for(fantoccini::Method::Get, &img).unwrap();
// this is a RequestBuilder from hyper, so we could also add POST data here
// but for this we just send the request
let mut res = raw.send().unwrap();
// we then read out the image bytes
use std::io::prelude::*;
let mut pixels = Vec::new();
res.read_to_end(&mut pixels).unwrap();
// and voilla, we now have the bytes for the Wikipedia logo!
assert!(pixels.len() > 0);
println!("Wikipedia logo is {}b", pixels.len());
```

[WebDriver protocol]: https://www.w3.org/TR/webdriver/
[CSS selectors]: https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Selectors
[powerful]: https://developer.mozilla.org/en-US/docs/Web/CSS/Pseudo-classes
[operators]: https://developer.mozilla.org/en-US/docs/Web/CSS/Attribute_selectors
[WebDriver compatible]: https://github.com/Fyrd/caniuse/issues/2757#issuecomment-304529217
[`geckodriver`]: https://github.com/mozilla/geckodriver
