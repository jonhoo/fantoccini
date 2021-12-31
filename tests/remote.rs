//! Tests that make use of external websites.

use fantoccini::{error, Client, Locator};
use futures_util::TryFutureExt;
use hyper::Method;
use serial_test::serial;
use std::time::Duration;
use url::Url;

mod common;

async fn works_inner(mut c: Client) -> Result<(), error::CmdError> {
    // go to the Wikipedia page for Foobar
    c.goto("https://en.wikipedia.org/wiki/Foobar").await?;
    let mut e = c.find(Locator::Id("History_and_etymology")).await?;
    let text = e.text().await?;
    assert_eq!(text, "History and etymology");
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

async fn clicks_inner_by_locator(mut c: Client) -> Result<(), error::CmdError> {
    // go to the Wikipedia frontpage this time
    c.goto("https://www.wikipedia.org/").await?;

    // find, fill out, and submit the search form
    let mut f = c.form(Locator::Css("#search-form")).await?;
    let f = f
        .set(Locator::Css("input[name='search']"), "foobar")
        .await?;
    f.submit().await?;

    // we should now have ended up in the rigth place
    let url = c.current_url().await?;
    assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");

    c.close().await
}

async fn clicks_inner(mut c: Client) -> Result<(), error::CmdError> {
    // go to the Wikipedia frontpage this time
    c.goto("https://www.wikipedia.org/").await?;

    // find, fill out, and submit the search form
    let mut f = c.form(Locator::Css("#search-form")).await?;
    let f = f.set_by_name("search", "foobar").await?;
    f.submit().await?;

    // we should now have ended up in the rigth place
    let url = c.current_url().await?;
    assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");

    c.close().await
}

async fn send_keys_and_clear_input_inner(mut c: Client) -> Result<(), error::CmdError> {
    // go to the Wikipedia frontpage this time
    c.goto("https://www.wikipedia.org/").await?;

    // find search input element
    let mut e = c.wait().for_element(Locator::Id("searchInput")).await?;
    e.send_keys("foobar").await?;
    assert_eq!(
        e.prop("value")
            .await?
            .expect("input should have value prop")
            .as_str(),
        "foobar"
    );

    e.clear().await?;
    assert_eq!(
        e.prop("value")
            .await?
            .expect("input should have value prop")
            .as_str(),
        ""
    );

    let mut c = e.client();
    c.close().await
}

async fn raw_inner(mut c: Client) -> Result<(), error::CmdError> {
    // go back to the frontpage
    c.goto("https://www.wikipedia.org/").await?;

    // find the source for the Wikipedia globe
    let mut img = c.find(Locator::Css("img.central-featured-logo")).await?;
    let src = img.attr("src").await?.expect("image should have a src");

    // now build a raw HTTP client request (which also has all current cookies)
    let raw = img.client().raw_client_for(Method::GET, &src).await?;

    // we then read out the image bytes
    let pixels = hyper::body::to_bytes(raw.into_body())
        .map_err(error::CmdError::from)
        .await?;

    // and voilla, we now have the bytes for the Wikipedia logo!
    assert!(!pixels.is_empty());
    println!("Wikipedia logo is {}b", pixels.len());

    c.close().await
}

async fn window_size_inner(mut c: Client) -> Result<(), error::CmdError> {
    c.goto("https://www.wikipedia.org/").await?;
    c.set_window_size(500, 400).await?;
    let (width, height) = c.get_window_size().await?;
    assert_eq!(width, 500);
    assert_eq!(height, 400);

    c.close().await
}

async fn window_position_inner(mut c: Client) -> Result<(), error::CmdError> {
    c.goto("https://www.wikipedia.org/").await?;
    c.set_window_size(200, 100).await?;
    c.set_window_position(0, 0).await?;
    c.set_window_position(1, 2).await?;
    let (x, y) = c.get_window_position().await?;
    assert_eq!(x, 1);
    assert_eq!(y, 2);

    c.close().await
}

async fn window_rect_inner(mut c: Client) -> Result<(), error::CmdError> {
    c.goto("https://www.wikipedia.org/").await?;
    c.set_window_rect(0, 0, 500, 400).await?;
    let (x, y) = c.get_window_position().await?;
    assert_eq!(x, 0);
    assert_eq!(y, 0);
    let (width, height) = c.get_window_size().await?;
    assert_eq!(width, 500);
    assert_eq!(height, 400);
    c.set_window_rect(1, 2, 600, 300).await?;
    let (x, y) = c.get_window_position().await?;
    assert_eq!(x, 1);
    assert_eq!(y, 2);
    let (width, height) = c.get_window_size().await?;
    assert_eq!(width, 600);
    assert_eq!(height, 300);

    c.close().await
}

async fn finds_all_inner(mut c: Client) -> Result<(), error::CmdError> {
    // go to the Wikipedia frontpage this time
    c.goto("https://en.wikipedia.org/").await?;
    let es = c.find_all(Locator::Css("#p-interaction li")).await?;
    let texts = futures_util::future::try_join_all(
        es.into_iter()
            .take(4)
            .map(|mut e| async move { e.text().await }),
    )
    .await?;
    assert_eq!(
        texts,
        [
            "Help",
            "Learn to edit",
            "Community portal",
            "Recent changes"
        ]
    );

    c.close().await
}

async fn finds_sub_elements(mut c: Client) -> Result<(), error::CmdError> {
    // Go to the Wikipedia front page
    c.goto("https://en.wikipedia.org/").await?;
    // Get the main sidebar panel
    let mut panel = c.find(Locator::Css("div#mw-panel")).await?;
    // Get all the ul elements in the sidebar
    let mut portals = panel.find_all(Locator::Css("nav.portal")).await?;

    let portal_titles = &[
        // Because GetElementText (used by Element::text()) returns the text
        // *as rendered*, hidden elements return an empty String.
        "",
        "Contribute",
        "Tools",
        "Print/export",
        "In other projects",
        "Languages",
    ];
    // Unless something fundamentally changes, this should work
    assert_eq!(portals.len(), portal_titles.len());

    for (i, portal) in portals.iter_mut().enumerate() {
        // Each "portal" has an h3 element.
        let mut portal_title = portal.find(Locator::Css("h3")).await?;
        let portal_title = portal_title.text().await?;
        assert_eq!(portal_title, portal_titles[i]);
        // And also an <ul>.
        let list_entries = portal.find_all(Locator::Css("li")).await?;
        assert!(!list_entries.is_empty());
    }

    c.close().await
}

async fn persist_inner(mut c: Client) -> Result<(), error::CmdError> {
    c.goto("https://en.wikipedia.org/").await?;
    c.persist().await?;

    c.close().await
}

async fn simple_wait_test(mut c: Client) -> Result<(), error::CmdError> {
    #[allow(deprecated)]
    c.wait_for(move |_| {
        std::thread::sleep(Duration::from_secs(4));
        async move { Ok(true) }
    })
    .await?;

    c.close().await
}

async fn wait_for_navigation_test(mut c: Client) -> Result<(), error::CmdError> {
    let mut path = std::env::current_dir().unwrap();
    path.push("tests/redirect_test.html");

    let path_string = format!("file://{}", path.to_str().unwrap());
    let file_url_str = path_string.as_str();
    let mut url = Url::parse(file_url_str)?;

    c.goto(url.as_str()).await?;

    #[allow(deprecated)]
    loop {
        let wait_for = c.wait_for_navigation(Some(url)).await;
        assert!(wait_for.is_ok());
        url = c.current_url().await?;
        if url.as_str() == "about:blank" {
            // try again
            continue;
        }
        assert_eq!(url.as_str(), "https://www.wikipedia.org/");
        break;
    }

    c.close().await
}

// Verifies that basic cookie handling works
async fn handle_cookies_test(mut c: Client) -> Result<(), error::CmdError> {
    c.goto("https://www.wikipedia.org/").await?;

    let cookies = c.get_all_cookies().await?;
    assert!(!cookies.is_empty());

    let first_cookie = &cookies[0];
    assert_eq!(
        c.get_named_cookie(first_cookie.name()).await?.value(),
        first_cookie.value()
    );

    // Delete a cookie and make sure it's gone
    c.delete_cookie(first_cookie.name()).await?;
    assert!(c.get_named_cookie(first_cookie.name()).await.is_err());

    c.delete_all_cookies().await?;
    let cookies = c.get_all_cookies().await?;
    assert!(dbg!(cookies).is_empty());

    c.close().await
}

mod chrome {
    use super::*;

    #[test]
    fn it_works() {
        tester!(works_inner, "chrome");
    }

    #[test]
    fn it_clicks() {
        tester!(clicks_inner, "chrome");
    }

    #[test]
    fn it_clicks_by_locator() {
        tester!(clicks_inner_by_locator, "chrome");
    }

    #[test]
    fn it_sends_keys_and_clear_input() {
        tester!(send_keys_and_clear_input_inner, "chrome");
    }

    #[test]
    fn it_can_be_raw() {
        tester!(raw_inner, "chrome");
    }

    #[test]
    #[ignore]
    fn it_can_get_and_set_window_size() {
        tester!(window_size_inner, "chrome");
    }

    #[test]
    #[ignore]
    fn it_can_get_and_set_window_position() {
        tester!(window_position_inner, "chrome");
    }

    #[test]
    #[ignore]
    fn it_can_get_and_set_window_rect() {
        tester!(window_rect_inner, "chrome");
    }

    #[test]
    fn it_finds_all() {
        tester!(finds_all_inner, "chrome");
    }

    #[test]
    fn it_finds_sub_elements() {
        tester!(finds_sub_elements, "chrome");
    }

    #[test]
    #[ignore]
    fn it_persists() {
        tester!(persist_inner, "chrome");
    }

    #[serial]
    #[test]
    fn it_simple_waits() {
        tester!(simple_wait_test, "chrome");
    }

    #[serial]
    #[test]
    fn it_waits_for_navigation() {
        tester!(wait_for_navigation_test, "chrome");
    }

    #[serial]
    #[test]
    fn it_handles_cookies() {
        tester!(handle_cookies_test, "chrome");
    }
}

mod firefox {
    use super::*;

    #[serial]
    #[test]
    fn it_works() {
        tester!(works_inner, "firefox");
    }

    #[serial]
    #[test]
    fn it_clicks() {
        tester!(clicks_inner, "firefox");
    }

    #[serial]
    #[test]
    fn it_clicks_by_locator() {
        tester!(clicks_inner_by_locator, "firefox");
    }

    #[serial]
    #[test]
    fn it_sends_keys_and_clear_input() {
        tester!(send_keys_and_clear_input_inner, "firefox");
    }

    #[serial]
    #[test]
    fn it_can_be_raw() {
        tester!(raw_inner, "firefox");
    }

    #[test]
    #[ignore]
    fn it_can_get_and_set_window_size() {
        tester!(window_size_inner, "firefox");
    }

    #[test]
    #[ignore]
    fn it_can_get_and_set_window_position() {
        tester!(window_position_inner, "firefox");
    }

    #[test]
    #[ignore]
    fn it_can_get_and_set_window_rect() {
        tester!(window_rect_inner, "firefox");
    }

    #[serial]
    #[test]
    fn it_finds_all() {
        tester!(finds_all_inner, "firefox");
    }

    #[serial]
    #[test]
    fn it_finds_sub_elements() {
        tester!(finds_sub_elements, "firefox");
    }

    #[test]
    #[ignore]
    fn it_persists() {
        tester!(persist_inner, "firefox");
    }

    #[serial]
    #[test]
    fn it_simple_waits() {
        tester!(simple_wait_test, "firefox");
    }

    #[serial]
    #[test]
    fn it_waits_for_navigation() {
        tester!(wait_for_navigation_test, "firefox");
    }

    #[serial]
    #[test]
    fn it_handles_cookies() {
        tester!(handle_cookies_test, "firefox");
    }
}
