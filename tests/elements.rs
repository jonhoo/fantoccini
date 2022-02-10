//! Element tests
use crate::common::sample_page_url;
use fantoccini::key::Key;
use fantoccini::{error, Client, Locator};
use serial_test::serial;

mod common;

async fn element_is(c: Client, port: u16) -> Result<(), error::CmdError> {
    let sample_url = sample_page_url(port);
    c.goto(&sample_url).await?;
    let elem = c.find(Locator::Id("checkbox-option-1")).await?;
    assert!(elem.is_enabled().await?);
    assert!(elem.is_displayed().await?);
    assert!(!elem.is_selected().await?);
    elem.click().await?;
    let elem = c.find(Locator::Id("checkbox-option-1")).await?;
    assert!(elem.is_selected().await?);

    assert!(
        !c.find(Locator::Id("checkbox-disabled"))
            .await?
            .is_enabled()
            .await?
    );
    assert!(
        !c.find(Locator::Id("checkbox-hidden"))
            .await?
            .is_displayed()
            .await?
    );
    Ok(())
}

async fn element_attr(c: Client, port: u16) -> Result<(), error::CmdError> {
    let sample_url = sample_page_url(port);
    c.goto(&sample_url).await?;
    let elem = c.find(Locator::Id("checkbox-option-1")).await?;
    assert_eq!(elem.attr("id").await?.unwrap(), "checkbox-option-1");
    assert!(elem.attr("invalid-attribute").await?.is_none());
    Ok(())
}

async fn element_prop(c: Client, port: u16) -> Result<(), error::CmdError> {
    let sample_url = sample_page_url(port);
    c.goto(&sample_url).await?;
    let elem = c.find(Locator::Id("checkbox-option-1")).await?;
    assert_eq!(elem.prop("id").await?.unwrap(), "checkbox-option-1");
    assert_eq!(elem.prop("checked").await?.unwrap(), "false");
    assert!(elem.attr("invalid-property").await?.is_none());
    Ok(())
}

async fn element_css_value(c: Client, port: u16) -> Result<(), error::CmdError> {
    let sample_url = sample_page_url(port);
    c.goto(&sample_url).await?;
    let elem = c.find(Locator::Id("checkbox-hidden")).await?;
    assert_eq!(elem.css_value("display").await?, "none");
    assert_eq!(elem.css_value("invalid-css-value").await?, "");
    Ok(())
}

async fn element_tag_name(c: Client, port: u16) -> Result<(), error::CmdError> {
    let sample_url = sample_page_url(port);
    c.goto(&sample_url).await?;
    let elem = c.find(Locator::Id("checkbox-option-1")).await?;
    let tag_name = elem.tag_name().await?;
    assert!(
        tag_name.eq_ignore_ascii_case("input"),
        "{} != input",
        tag_name
    );
    Ok(())
}

async fn element_rect(c: Client, port: u16) -> Result<(), error::CmdError> {
    let sample_url = sample_page_url(port);
    c.goto(&sample_url).await?;
    let elem = c.find(Locator::Id("button-alert")).await?;
    let rect = elem.rectangle().await?;
    // Rather than try to verify the exact position and size of the element,
    // let's just verify that the returned values deserialized ok and
    // are within the expected range.
    assert!(rect.0 > 0.0);
    assert!(rect.0 < 100.0);
    assert!(rect.1 > 0.0);
    assert!(rect.1 < 1000.0);
    assert!(rect.2 > 0.0);
    assert!(rect.2 < 200.0);
    assert!(rect.3 > 0.0);
    assert!(rect.3 < 200.0);
    Ok(())
}

async fn element_send_keys(c: Client, port: u16) -> Result<(), error::CmdError> {
    let sample_url = sample_page_url(port);
    c.goto(&sample_url).await?;
    let elem = c.find(Locator::Id("text-input")).await?;
    assert_eq!(elem.prop("value").await?.unwrap(), "");
    elem.send_keys("fantoccini").await?;
    assert_eq!(elem.prop("value").await?.unwrap(), "fantoccini");
    let select_all = if cfg!(target_os = "macos") {
        Key::Command + "a"
    } else {
        Key::Control + "a"
    };
    let backspace = Key::Backspace.to_string();
    elem.send_keys(&select_all).await?;
    elem.send_keys(&backspace).await?;
    assert_eq!(elem.prop("value").await?.unwrap(), "");

    Ok(())
}

mod firefox {
    use super::*;

    #[test]
    #[serial]
    fn element_is_test() {
        local_tester!(element_is, "firefox");
    }

    #[test]
    #[serial]
    fn element_attr_test() {
        local_tester!(element_attr, "firefox");
    }

    #[test]
    #[serial]
    fn element_prop_test() {
        local_tester!(element_prop, "firefox");
    }

    #[test]
    #[serial]
    fn element_css_value_test() {
        local_tester!(element_css_value, "firefox");
    }

    #[test]
    #[serial]
    fn element_tag_name_test() {
        local_tester!(element_tag_name, "firefox");
    }

    #[test]
    #[serial]
    fn element_rect_test() {
        local_tester!(element_rect, "firefox");
    }

    #[test]
    #[serial]
    fn element_send_keys_test() {
        local_tester!(element_send_keys, "firefox");
    }
}

mod chrome {
    use super::*;

    #[test]
    fn element_is_test() {
        local_tester!(element_is, "chrome");
    }

    #[test]
    fn element_attr_test() {
        local_tester!(element_attr, "chrome");
    }

    #[test]
    fn element_prop_test() {
        local_tester!(element_prop, "chrome");
    }

    #[test]
    fn element_css_value_test() {
        local_tester!(element_css_value, "chrome");
    }

    #[test]
    fn element_tag_name_test() {
        local_tester!(element_tag_name, "chrome");
    }

    #[test]
    fn element_rect_test() {
        local_tester!(element_rect, "chrome");
    }

    #[test]
    fn element_send_keys_test() {
        local_tester!(element_send_keys, "chrome");
    }
}
