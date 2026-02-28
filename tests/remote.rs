//! Tests that make use of external websites.

use cookie::SameSite;
use fantoccini::{error, Client};
use serial_test::serial;

mod common;

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
    cookie.set_secure(true);
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
    #[serial]
    fn it_handles_cookies() {
        tester!(handle_cookies_test, "chrome");
    }
}

mod firefox {
    use super::*;

    #[test]
    #[serial]
    fn it_handles_cookies() {
        tester!(handle_cookies_test, "firefox");
    }
}
