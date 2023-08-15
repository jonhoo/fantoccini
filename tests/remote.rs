//! Tests that make use of external websites.

use cookie::SameSite;
use fantoccini::{error, Client, Locator};
use futures_util::TryFutureExt;
use hyper::Method;
use serial_test::serial;
use std::time::Duration;
use url::Url;

// To Be removed when tests are moved to local.rs
use crate::common::sample_page_url;

mod common;

async fn works_inner(c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;

    let e = c.find(Locator::Id("span_id")).await?;
    let text = e.text().await?;
    assert_eq!(text, "Span");

    let current_url = c.current_url().await?;
    assert_eq!(current_url.as_ref(), &url);

    // click "Other Page"
    c.find(Locator::Css(".other_page")).await?.click().await?;

    // click "iframe inner"
    c.find(Locator::LinkText("iframe inner"))
        .await?
        .click()
        .await?;

    let current_url = c.current_url().await?;
    assert_eq!(
        current_url.as_ref(),
        format!("http://localhost:{}/iframe_inner.html", port)
    );

    c.close().await
}

async fn clicks_inner_by_locator(c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;

    // find, fill out, and submit the search form
    let f = c.form(Locator::Css("#search-form")).await?;
    let f = f
        .set(Locator::Css("input[name='search']"), "foobar")
        .await?;
    f.submit().await?;

    // we should now have ended up in the rigth place
    let current_url = c.current_url().await?;
    assert_eq!(current_url.as_ref(), format!("{}?search=foobar", url));

    c.close().await
}

async fn clicks_inner(c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;

    // find, fill out, and submit the search form
    let f = c.form(Locator::Css("#search-form")).await?;
    let f = f.set_by_name("search", "foobar").await?;
    f.submit().await?;

    // we should now have ended up in the rigth place
    let current_url = c.current_url().await?;
    // This is not a 1to1 match with previous test ('foobar' vs ?search=foobar),
    // but I believe it has the same result
    assert_eq!(current_url.as_ref(), format!("{}?search=foobar", url));

    c.close().await
}

async fn send_keys_and_clear_input_inner(c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;

    // find search input element
    let e = c.wait().for_element(Locator::Id("text-input")).await?;
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

    let c = e.client();
    c.close().await
}

async fn raw_inner(c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;

    // find the source for the globe
    let img = c.find(Locator::Css("img.globe")).await?;
    let src = img.attr("src").await?.expect("image should have a src");

    // now build a raw HTTP client request (which also has all current cookies)
    let raw = img.client().raw_client_for(Method::GET, &src).await?;

    // we then read out the image bytes
    let pixels = hyper::body::to_bytes(raw.into_body())
        .map_err(error::CmdError::from)
        .await?;

    // and voilla, we now have the bytes for the globe!
    assert!(!pixels.is_empty());
    println!("The logo is {}b", pixels.len());

    c.close().await
}

async fn window_size_inner(c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;
    c.set_window_size(500, 400).await?;
    let (width, height) = c.get_window_size().await?;
    assert_eq!(width, 500);
    assert_eq!(height, 400);

    c.close().await
}

async fn window_position_inner(c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;
    c.set_window_size(200, 100).await?;
    c.set_window_position(0, 0).await?;
    c.set_window_position(1, 2).await?;
    let (x, y) = c.get_window_position().await?;
    assert_eq!(x, 1);
    assert_eq!(y, 2);

    c.close().await
}

async fn window_rect_inner(c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;
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

async fn finds_all_inner(c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;

    // Find all the footer links
    let es = c.find_all(Locator::Css("#footer li")).await?;
    let mut texts =
        futures_util::future::try_join_all(es.into_iter().map(|e| async move { e.text().await }))
            .await?;
    texts.retain(|t| !t.is_empty());
    assert_eq!(texts, ["Footer Element", "Another Footer Element",]);

    c.close().await
}

async fn finds_sub_elements(c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;

    // Get the main footer
    let footer = c.find(Locator::Css("#footer")).await?;
    // Get all the li place elements in the footer
    let mut places = footer.find_all(Locator::Css("li")).await?;

    let place_titles = &["Footer Element", "Another Footer Element"];

    for (i, place) in places.iter_mut().enumerate() {
        // Each "place" has a link element.
        let place_title = place.find(Locator::Css("a")).await?;
        let place_title = place_title.text().await?;
        if place_title.is_empty() {
            assert!(i >= place_titles.len(), "{} >= {}", i, place_titles.len());
        } else {
            assert_eq!(place_title, place_titles[i]);
        }
    }

    c.close().await
}

async fn persist_inner(c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;
    c.persist().await?;

    c.close().await
}

async fn simple_wait_test(c: Client, _port: u16) -> Result<(), error::CmdError> {
    #[allow(deprecated)]
    c.wait_for(move |_| {
        std::thread::sleep(Duration::from_secs(4));
        async move { Ok(true) }
    })
    .await?;

    c.close().await
}

async fn wait_for_navigation_test(c: Client) -> Result<(), error::CmdError> {
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
async fn handle_cookies_test(c: Client) -> Result<(), error::CmdError> {
    c.goto("https://www.wikipedia.org/").await?;

    let cookies = c.get_all_cookies().await?;
    assert!(!cookies.is_empty());

    // Add a new cookie.
    use fantoccini::cookies::Cookie;
    let mut cookie = Cookie::new("cookietest", "fantoccini");
    cookie.set_domain(".wikipedia.org");
    cookie.set_path("/");
    cookie.set_same_site(Some(SameSite::Lax));
    c.add_cookie(cookie.clone()).await?;

    // Verify that the cookie exists.
    assert_eq!(
        c.get_named_cookie(cookie.name()).await?.value(),
        cookie.value()
    );

    // Delete the cookie and make sure it's gone
    c.delete_cookie(cookie.name()).await?;
    assert!(c.get_named_cookie(cookie.name()).await.is_err());

    // Verify same_site None corner-case is correctly parsed
    cookie.set_same_site(None);
    c.add_cookie(cookie.clone()).await?;
    assert_eq!(
        c.get_named_cookie(cookie.name()).await?.same_site(),
        Some(SameSite::None)
    );

    c.delete_all_cookies().await?;
    let cookies = c.get_all_cookies().await?;
    assert!(dbg!(cookies).is_empty());

    c.close().await
}

mod chrome {
    use super::*;

    #[test]
    fn it_works() {
        local_tester!(works_inner, "chrome");
    }

    #[test]
    fn it_clicks() {
        local_tester!(clicks_inner, "chrome");
    }

    #[test]
    fn it_clicks_by_locator() {
        local_tester!(clicks_inner_by_locator, "chrome");
    }

    #[test]
    fn it_sends_keys_and_clear_input() {
        local_tester!(send_keys_and_clear_input_inner, "chrome");
    }

    #[test]
    fn it_can_be_raw() {
        local_tester!(raw_inner, "chrome");
    }

    #[test]
    #[ignore]
    fn it_can_get_and_set_window_size() {
        local_tester!(window_size_inner, "chrome");
    }

    #[test]
    #[ignore]
    fn it_can_get_and_set_window_position() {
        local_tester!(window_position_inner, "chrome");
    }

    #[test]
    #[ignore]
    fn it_can_get_and_set_window_rect() {
        local_tester!(window_rect_inner, "chrome");
    }

    #[test]
    fn it_finds_all() {
        local_tester!(finds_all_inner, "chrome");
    }

    #[test]
    fn it_finds_sub_elements() {
        local_tester!(finds_sub_elements, "chrome");
    }

    #[test]
    #[ignore]
    fn it_persists() {
        local_tester!(persist_inner, "chrome");
    }

    #[serial]
    #[test]
    fn it_simple_waits() {
        local_tester!(simple_wait_test, "chrome");
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
        local_tester!(works_inner, "firefox");
    }

    #[serial]
    #[test]
    fn it_clicks() {
        local_tester!(clicks_inner, "firefox");
    }

    #[serial]
    #[test]
    fn it_clicks_by_locator() {
        local_tester!(clicks_inner_by_locator, "firefox");
    }

    #[serial]
    #[test]
    fn it_sends_keys_and_clear_input() {
        local_tester!(send_keys_and_clear_input_inner, "firefox");
    }

    #[serial]
    #[test]
    fn it_can_be_raw() {
        local_tester!(raw_inner, "firefox");
    }

    #[test]
    #[ignore]
    fn it_can_get_and_set_window_size() {
        local_tester!(window_size_inner, "firefox");
    }

    #[test]
    #[ignore]
    fn it_can_get_and_set_window_position() {
        local_tester!(window_position_inner, "firefox");
    }

    #[test]
    #[ignore]
    fn it_can_get_and_set_window_rect() {
        local_tester!(window_rect_inner, "firefox");
    }

    #[serial]
    #[test]
    fn it_finds_all() {
        local_tester!(finds_all_inner, "firefox");
    }

    #[serial]
    #[test]
    fn it_finds_sub_elements() {
        local_tester!(finds_sub_elements, "firefox");
    }

    #[test]
    #[ignore]
    fn it_persists() {
        local_tester!(persist_inner, "firefox");
    }

    #[serial]
    #[test]
    fn it_simple_waits() {
        local_tester!(simple_wait_test, "firefox");
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
