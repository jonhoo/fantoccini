//! Tests that don't make use of external websites.
#[macro_use]
extern crate serial_test;
extern crate fantoccini;
extern crate futures_util;

use fantoccini::{error, Client, Locator};

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

async fn find_and_click_link(mut c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;
    c.find(Locator::Css("#other_page_id"))
        .await?
        .click()
        .await?;

    let new_url = c.current_url().await?;
    let expected_url = format!("http://localhost:{}/other_page.html", port);
    assert_eq!(new_url.as_str(), expected_url.as_str());

    c.close().await
}

async fn get_active_element(mut c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;
    c.find(Locator::Css("#select1")).await?.click().await?;

    let mut active = c.active_element().await?;
    assert_eq!(active.attr("id").await?, Some(String::from("select1")));

    c.close().await
}

async fn serialize_element(mut c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;
    let elem = c.find(Locator::Css("#other_page_id")).await?;

    // Check that webdriver understands it
    c.execute(
        "arguments[0].scrollIntoView(true);",
        vec![serde_json::to_value(elem)?],
    )
    .await?;

    // Check that it fails with an invalid serialization (from a previous run of the test)
    let json = r#"{"element-6066-11e4-a52e-4f735466cecf":"fbe5004d-ec8b-4c7b-ad08-642c55d84505"}"#;
    c.execute(
        "arguments[0].scrollIntoView(true);",
        vec![serde_json::from_str(json)?],
    )
    .await
    .expect_err("Failure expected with an invalid ID");

    c.close().await
}

async fn iframe_switch(mut c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;
    // Go to the page that holds the iframe
    c.find(Locator::Css("#iframe_page_id"))
        .await?
        .click()
        .await?;

    c.find(Locator::Id("iframe_button"))
        .await
        .expect_err("should not find the button in the iframe");
    c.find(Locator::Id("root_button")).await?; // Can find the button in the root context though.

    // find and switch into the iframe
    let iframe_element = c.find(Locator::Id("iframe")).await?;
    iframe_element.enter_frame().await?;

    // search for something in the iframe
    let button_in_iframe = c.find(Locator::Id("iframe_button")).await?;
    button_in_iframe.click().await?;
    c.find(Locator::Id("root_button"))
        .await
        .expect_err("Should not be able to access content in the root context");

    // switch back to the root context and access content there.
    let mut c = c.enter_parent_frame().await?;
    c.find(Locator::Id("root_button")).await?;

    c.close().await
}

async fn new_window(mut c: Client) -> Result<(), error::CmdError> {
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

async fn close_window(mut c: Client) -> Result<(), error::CmdError> {
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

async fn close_window_twice_errors(mut c: Client) -> Result<(), error::CmdError> {
    c.close_window().await?;
    c.close_window()
        .await
        .expect_err("Should get a no such window error");
    Ok(())
}

async fn set_by_name_textarea(mut c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;

    let mut form = c.form(Locator::Css("form")).await?;
    form.set_by_name("some_textarea", "a value!").await?;

    let value = c
        .find(Locator::Css("textarea"))
        .await?
        .prop("value")
        .await?
        .expect("textarea should contain a value");

    assert_eq!(value, "a value!");

    Ok(())
}

async fn stale_element(mut c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;
    let elem = c.find(Locator::Css("#other_page_id")).await?;

    // Remove the element from the DOM
    c.execute(
        "var elem = document.getElementById('other_page_id');
         elem.parentNode.removeChild(elem);",
        vec![],
    )
    .await?;

    match elem.click().await {
        Err(error::CmdError::NoSuchElement(_)) => Ok(()),
        _ => panic!("Expected a stale element reference error"),
    }
}

async fn select_by_index(mut c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;

    let mut select_element = c.find(Locator::Css("#select1")).await?;

    // Get first display text
    let initial_text = select_element.prop("value").await?;
    assert_eq!(Some("Select1-Option1".into()), initial_text);

    // Select second option
    select_element.clone().select_by_index(1).await?;

    // Get display text after selection
    let text_after_selecting = select_element.prop("value").await?;
    assert_eq!(Some("Select1-Option2".into()), text_after_selecting);

    // Check that the second select is not changed
    let select2_text = c
        .find(Locator::Css("#select2"))
        .await?
        .prop("value")
        .await?;
    assert_eq!(Some("Select2-Option1".into()), select2_text);

    // Show off that it selects only options and skip any other elements
    let mut select_element = c.find(Locator::Css("#select2")).await?;
    select_element.clone().select_by_index(1).await?;
    let text = select_element.prop("value").await?;
    assert_eq!(Some("Select2-Option2".into()), text);

    Ok(())
}

async fn select_by_label(mut c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;

    let mut select_element = c.find(Locator::Css("#select1")).await?;

    // Get first display text
    let initial_text = select_element.prop("value").await?;
    assert_eq!(Some("Select1-Option1".into()), initial_text);

    // Select second option
    select_element
        .clone()
        .select_by_label("Select1-Option2")
        .await?;

    // Get display text after selection
    let text_after_selecting = select_element.prop("value").await?;
    assert_eq!(Some("Select1-Option2".into()), text_after_selecting);

    // Check that the second select is not changed
    let select2_text = c
        .find(Locator::Css("#select2"))
        .await?
        .prop("value")
        .await?;
    assert_eq!(Some("Select2-Option1".into()), select2_text);

    Ok(())
}

async fn select_by(mut c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;

    let mut select_element = c.find(Locator::Css("#select3")).await?;

    // Get first display text
    let initial_text = select_element.prop("value").await?;
    assert_eq!(Some("Select3-Option1".into()), initial_text);

    // Select third option via css
    select_element
        .clone()
        .select_by(Locator::Css("#select3-option-3"))
        .await?;

    // Get display text after selection
    let text_after_selecting = select_element.prop("value").await?;
    assert_eq!(Some("Select3-Option3".into()), text_after_selecting);

    Ok(())
}

async fn resolve_execute_async_value(mut c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;

    let count: u64 = c
        .execute_async(
            "setTimeout(() => arguments[1](arguments[0] + 1))",
            vec![1_u32.into()],
        )
        .await?
        .as_u64()
        .expect("should be integer variant");

    assert_eq!(2, count);

    let count: u64 = c
        .execute_async("setTimeout(() => arguments[0](2))", vec![])
        .await?
        .as_u64()
        .expect("should be integer variant");

    assert_eq!(2, count);

    Ok(())
}

mod firefox {
    use super::*;
    #[test]
    #[serial]
    fn navigate_to_other_page() {
        local_tester!(goto, "firefox");
    }

    #[test]
    #[serial]
    fn find_and_click_link_test() {
        local_tester!(find_and_click_link, "firefox");
    }

    #[test]
    #[serial]
    fn get_active_element_test() {
        local_tester!(get_active_element, "firefox");
    }

    #[test]
    #[serial]
    fn serialize_element_test() {
        local_tester!(serialize_element, "firefox");
    }

    #[test]
    #[serial]
    fn iframe_test() {
        local_tester!(iframe_switch, "firefox");
    }

    #[test]
    #[serial]
    fn new_window_test() {
        tester!(new_window, "firefox");
    }

    #[test]
    #[serial]
    fn new_window_switch_test() {
        tester!(new_window_switch, "firefox");
    }

    #[test]
    #[serial]
    fn new_tab_switch_test() {
        tester!(new_tab_switch, "firefox");
    }

    #[test]
    #[serial]
    fn close_window_test() {
        tester!(close_window, "firefox");
    }

    #[test]
    #[serial]
    fn double_close_window_test() {
        tester!(close_window_twice_errors, "firefox");
    }

    #[test]
    #[serial]
    fn set_by_name_textarea_test() {
        local_tester!(set_by_name_textarea, "firefox");
    }

    #[test]
    #[serial]
    fn stale_element_test() {
        local_tester!(stale_element, "firefox");
    }

    #[test]
    #[serial]
    fn select_by_index_test() {
        local_tester!(select_by_index, "firefox");
    }

    #[test]
    #[serial]
    fn select_by_test() {
        local_tester!(select_by, "firefox")
    }

    #[test]
    #[serial]
    fn select_by_label_test() {
        local_tester!(select_by_label, "firefox");
    }

    #[test]
    #[serial]
    fn resolve_execute_async_value_test() {
        local_tester!(resolve_execute_async_value, "firefox");
    }
}

mod chrome {
    use super::*;
    #[test]
    fn navigate_to_other_page() {
        local_tester!(goto, "chrome");
    }

    #[test]
    fn find_and_click_link_test() {
        local_tester!(find_and_click_link, "chrome");
    }

    #[test]
    fn get_active_element_test() {
        local_tester!(get_active_element, "chrome");
    }

    #[test]
    fn serialize_element_test() {
        local_tester!(serialize_element, "chrome");
    }

    #[test]
    fn iframe_test() {
        local_tester!(iframe_switch, "chrome");
    }

    #[test]
    fn new_window_test() {
        tester!(new_window, "chrome");
    }

    #[test]
    fn new_window_switch_test() {
        tester!(new_window_switch, "chrome");
    }

    #[test]
    fn new_tab_test() {
        tester!(new_tab_switch, "chrome");
    }

    #[test]
    fn close_window_test() {
        tester!(close_window, "chrome");
    }

    #[test]
    fn double_close_window_test() {
        tester!(close_window_twice_errors, "chrome");
    }

    #[test]
    fn set_by_name_textarea_test() {
        local_tester!(set_by_name_textarea, "chrome");
    }

    #[test]
    #[serial]
    fn select_by_label_test() {
        local_tester!(select_by_label, "chrome");
    }

    #[test]
    fn select_by_index_label() {
        local_tester!(select_by_index, "chrome");
    }

    #[test]
    fn select_by_test() {
        local_tester!(select_by, "chrome")
    }
}
