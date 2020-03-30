#[macro_use]
extern crate serial_test_derive;
extern crate fantoccini;
extern crate futures_util;

use fantoccini::{error, Client};

mod common;


fn sample_page_url(port: u16) -> String {
    format!("http://localhost:{}/sample_page.html", port)
}

async fn goto(mut c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;
    let current_url = c.current_url().await?;
    assert_eq!(url.as_str(), current_url.as_str());
    c.close().await
}

async fn new_window(mut c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;
    c.new_window(false).await?;
    let windows = c.windows().await?;
    assert_eq!(windows.len(), 2);
    c.close().await
}

async fn new_window_switch(mut c: Client) -> Result<(), error::CmdError> {
    let window_1 = c.window().await?;
    c.new_window(false).await?;
    let window_2 = c.window().await?;
    assert_eq!(
        window_1, window_2,
        "After creating a new window, the session should not have switched to it"
    );

    let all_windows = c.windows().await?;
    assert_eq!(all_windows.len(), 2);
    let new_window = all_windows
        .into_iter()
        .find(|handle| handle != &window_1)
        .expect("Should find a differing window handle");

    c.switch_to_window(new_window).await?;

    let window_3 = c.window().await?;
    assert_ne!(
        window_3, window_2,
        "After switching to a new window, the window handle returned from window() should differ now."
    );

    c.close().await
}


async fn new_tab_switch(mut c: Client) -> Result<(), error::CmdError> {
    let window_1 = c.window().await?;
    c.new_window(true).await?;
    let window_2 = c.window().await?;
    assert_eq!(
        window_1, window_2,
        "After creating a new window, the session should not have switched to it"
    );

    let all_windows = c.windows().await?;
    assert_eq!(all_windows.len(), 2);
    let new_window = all_windows
        .into_iter()
        .find(|handle| handle != &window_1)
        .expect("Should find a differing window handle");

    c.switch_to_window(new_window).await?;

    let window_3 = c.window().await?;
    assert_ne!(
        window_3, window_2,
        "After switching to a new window, the window handle returned from window() should differ now."
    );

    c.close().await
}

async fn close_window(mut c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;
    let window_1 = c.window().await?;
    c.new_window(true).await?;
    let window_2 = c.window().await?;
    assert_eq!(
        window_1, window_2,
        "Creating a new window should not cause the client to switch to it."
    );

    let handles = c.windows().await?;
    assert_eq!(handles.len(), 2);

    c.close_window().await?;
    c.window()
        .await
        .expect_err("After closing a window, the client can't find its currently selected window.");

    let other_window = handles
        .into_iter()
        .find(|handle| handle != &window_2)
        .expect("Should find a differing handle");
    c.switch_to_window(other_window).await?;

    // Close the session by closing the remaining window
    c.close_window().await?;

    c.windows().await.expect_err("Session should be closed.");
    Ok(())
}

async fn close_window_twice_errors(mut c: Client, _port: u16) -> Result<(), error::CmdError> {
    c.close_window().await?;
    c.close_window()
        .await
        .expect_err("Should get a no such window error");
    Ok(())
}

mod firefox {
    use super::*;
    #[test]
    #[serial]
    fn navigate_to_other_page() {
        local_tester!(goto, "firefox")
    }

    #[test]
    #[serial]
    fn new_window_test() {
        local_tester!(new_window, "firefox")
    }

    #[test]
    #[serial]
    fn new_window_switch_test() {
        tester!(new_window_switch, "firefox")
    }

    #[test]
    #[serial]
    fn new_tab_switch_test() {
        tester!(new_tab_switch, "firefox")
    }

    #[test]
    #[serial]
    fn close_window_test() {
        local_tester!(close_window, "firefox")
    }

    #[test]
    #[serial]
    fn double_close_window_test() {
        local_tester!(close_window_twice_errors, "firefox")
    }
}

mod chrome {
    use super::*;
    #[test]
    #[serial]
    fn navigate_to_other_page() {
        local_tester!(goto, "chrome")
    }

    #[test]
    #[serial]
    fn new_window_test() {
        local_tester!(new_window, "chrome")
    }

    #[test]
    #[serial]
    fn new_window_switch_test() {
        tester!(new_window_switch, "chrome")
    }

    #[test]
    #[serial]
    fn new_tab_test() {
        tester!(new_tab_switch, "chrome")
    }

    #[test]
    #[serial]
    fn close_window_test() {
        local_tester!(close_window, "chrome")
    }

    #[test]
    #[serial]
    fn double_close_window_test() {
        local_tester!(close_window_twice_errors, "chrome")
    }
}
