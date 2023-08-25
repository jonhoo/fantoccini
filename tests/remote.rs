//! Tests that make use of external websites.

use cookie::SameSite;
use fantoccini::{error, Client};
use serial_test::serial;
use url::Url;

mod common;

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
    fn it_waits_for_navigation() {
        tester!(wait_for_navigation_test, "firefox");
    }

    #[serial]
    #[test]
    fn it_handles_cookies() {
        tester!(handle_cookies_test, "firefox");
    }
}
