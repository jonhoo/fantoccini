//! Alert tests
use crate::common::sample_page_url;
use fantoccini::{error, Client, Locator};
use serial_test::serial;

mod common;

async fn alert_accept(c: Client, port: u16) -> Result<(), error::CmdError> {
    let sample_url = sample_page_url(port);
    c.goto(&sample_url).await?;
    c.find(Locator::Id("button-alert")).await?.click().await?;
    assert_eq!(c.get_alert_text().await?, "This is an alert");
    c.accept_alert().await?;
    assert!(matches!(
        c.get_alert_text().await,
        Err(e) if e.is_no_such_alert()
    ));

    c.find(Locator::Id("button-confirm")).await?.click().await?;
    assert_eq!(c.get_alert_text().await?, "Press OK or Cancel");
    c.accept_alert().await?;
    assert!(matches!(
        c.get_alert_text().await,
        Err(e) if e.is_no_such_alert()
    ));
    assert_eq!(
        c.find(Locator::Id("alert-answer")).await?.text().await?,
        "OK"
    );

    Ok(())
}

async fn alert_dismiss(c: Client, port: u16) -> Result<(), error::CmdError> {
    let sample_url = sample_page_url(port);
    c.goto(&sample_url).await?;
    c.find(Locator::Id("button-alert")).await?.click().await?;
    assert_eq!(c.get_alert_text().await?, "This is an alert");
    c.dismiss_alert().await?;
    assert!(matches!(
        c.get_alert_text().await,
        Err(e) if e.is_no_such_alert()
    ));

    c.find(Locator::Id("button-confirm")).await?.click().await?;
    assert_eq!(c.get_alert_text().await?, "Press OK or Cancel");
    c.dismiss_alert().await?;
    assert!(matches!(
        c.get_alert_text().await,
        Err(e) if e.is_no_such_alert()
    ));
    assert_eq!(
        c.find(Locator::Id("alert-answer")).await?.text().await?,
        "Cancel"
    );

    Ok(())
}

async fn alert_text(c: Client, port: u16) -> Result<(), error::CmdError> {
    let sample_url = sample_page_url(port);
    c.goto(&sample_url).await?;
    c.find(Locator::Id("button-prompt")).await?.click().await?;
    assert_eq!(c.get_alert_text().await?, "What is your name?");
    c.send_alert_text("Fantoccini").await?;
    c.accept_alert().await?;
    assert!(matches!(
        c.get_alert_text().await,
        Err(e) if e.is_no_such_alert()
    ));
    assert_eq!(
        c.find(Locator::Id("alert-answer")).await?.text().await?,
        "Fantoccini"
    );

    Ok(())
}

mod firefox {
    use super::*;

    #[test]
    #[serial]
    fn alert_accept_test() {
        local_tester!(alert_accept, "firefox");
    }

    #[test]
    #[serial]
    fn alert_dismiss_test() {
        local_tester!(alert_dismiss, "firefox");
    }

    #[test]
    #[serial]
    fn alert_text_test() {
        local_tester!(alert_text, "firefox");
    }
}

mod chrome {
    use super::*;

    #[test]
    fn alert_accept_test() {
        local_tester!(alert_accept, "chrome");
    }

    #[test]
    fn alert_dismiss_test() {
        local_tester!(alert_dismiss, "chrome");
    }

    #[test]
    fn alert_text_test() {
        local_tester!(alert_text, "chrome");
    }
}
