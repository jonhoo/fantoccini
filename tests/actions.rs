//! Actions tests
use crate::common::sample_page_url;
use fantoccini::actions::{
    Actions, InputSource, KeyAction, KeyActions, MouseActions, NullActions, PointerAction,
    MOUSE_BUTTON_LEFT,
};
use fantoccini::key::Key;
use fantoccini::{error, Client, Locator};
use serial_test::serial;
use std::time::Duration;
use time::Instant;

mod common;

async fn actions_null(c: Client, port: u16) -> Result<(), error::CmdError> {
    let sample_url = sample_page_url(port);
    c.goto(&sample_url).await?;
    let null_actions = NullActions::new("null".to_string()).pause(Duration::from_secs(1));
    let now = Instant::now();
    c.perform_actions(null_actions).await?;
    assert!(now.elapsed().as_seconds_f64() >= 1.0);
    Ok(())
}

async fn actions_key(c: Client, port: u16) -> Result<(), error::CmdError> {
    let sample_url = sample_page_url(port);
    c.goto(&sample_url).await?;

    // Test pause.
    let key_pause = KeyActions::new("key".to_string()).pause(Duration::from_secs(1));
    let now = Instant::now();
    c.perform_actions(key_pause).await?;
    assert!(now.elapsed().as_seconds_f64() >= 1.0);

    // Test key down/up.
    let elem = c.find(Locator::Id("text-input")).await?;
    elem.send_keys("a").await?;
    assert_eq!(elem.prop("value").await?.unwrap(), "a");

    let key_actions = KeyActions::new("key".to_string())
        .then(KeyAction::Down {
            value: Key::Backspace.into(),
        })
        .then(KeyAction::Up {
            value: Key::Backspace.into(),
        });
    elem.click().await?;
    c.perform_actions(key_actions).await?;
    let elem = c.find(Locator::Id("text-input")).await?;
    assert_eq!(elem.prop("value").await?.unwrap(), "");
    Ok(())
}

async fn actions_mouse(c: Client, port: u16) -> Result<(), error::CmdError> {
    let sample_url = sample_page_url(port);
    c.goto(&sample_url).await?;

    // Test pause.
    let mouse_pause = MouseActions::new("mouse".to_string()).pause(Duration::from_secs(1));
    let now = Instant::now();
    c.perform_actions(mouse_pause).await?;
    assert!(now.elapsed().as_seconds_f64() >= 1.0);

    let elem = c.find(Locator::Id("button-alert")).await?;

    // Test mouse down/up.
    let mouse_actions = MouseActions::new("mouse".to_string())
        .then(PointerAction::MoveToElement {
            element: elem,
            duration: None,
            x: 0,
            y: 0,
        })
        .then(PointerAction::Down {
            button: MOUSE_BUTTON_LEFT,
        })
        .then(PointerAction::Up {
            button: MOUSE_BUTTON_LEFT,
        });

    c.perform_actions(mouse_actions).await?;
    assert_eq!(c.get_alert_text().await?, "This is an alert");
    c.dismiss_alert().await?;
    Ok(())
}

async fn actions_mouse_move(c: Client, port: u16) -> Result<(), error::CmdError> {
    // Set window size to avoid moving the cursor out-of-bounds during actions.
    c.set_window_rect(0, 0, 800, 800).await?;

    let sample_url = sample_page_url(port);
    c.goto(&sample_url).await?;

    let elem = c.find(Locator::Id("button-alert")).await?;
    let rect = elem.rectangle().await?;
    let elem_center_x = rect.0 + (rect.2 / 2.0);
    let elem_center_y = rect.1 + (rect.3 / 2.0);

    // Test mouse MoveBy.
    let mouse_actions = MouseActions::new("mouse".to_string())
        // Move to a position at a known offset from the button.
        .then(PointerAction::MoveTo {
            duration: None,
            x: 0,
            y: elem_center_y as i64 - 100,
        })
        // Now move by relative offset so that the cursor is now over the button.
        .then(PointerAction::MoveBy {
            duration: None,
            x: elem_center_x as i64,
            y: 100,
        })
        // Press left mouse button down.
        .then(PointerAction::Down {
            button: MOUSE_BUTTON_LEFT,
        })
        // Release left mouse button.
        .then(PointerAction::Up {
            button: MOUSE_BUTTON_LEFT,
        });

    // Sanity check - ensure no alerts are displayed prior to actions.
    assert!(matches!(
        c.get_alert_text().await,
        Err(e) if e.is_no_such_alert()
    ));

    let actions = Actions::from(mouse_actions);
    c.perform_actions(actions).await?;
    assert_eq!(c.get_alert_text().await?, "This is an alert");
    c.accept_alert().await?;

    Ok(())
}

async fn actions_release(c: Client, port: u16) -> Result<(), error::CmdError> {
    let sample_url = sample_page_url(port);
    c.goto(&sample_url).await?;

    // Focus the input element.
    let elem = c.find(Locator::Id("text-input")).await?;
    elem.click().await?;

    // Add initial text.
    let elem = c.find(Locator::Id("text-input")).await?;
    assert_eq!(elem.prop("value").await?.unwrap(), "");

    // Press CONTROL key down and hold it.
    let key_actions = KeyActions::new("key".to_string()).then(KeyAction::Down {
        value: Key::Control.into(),
    });
    c.perform_actions(key_actions).await?;

    // Now release all actions. This should release the control key.
    c.release_actions().await?;

    // Now press the 'a' key again.
    //
    // If the Control key was not released, this would do `Ctrl+a` (i.e. select all)
    // but there is no text so it would do nothing.
    //
    // However if the Control key was released (as expected)
    // then this will type 'a' into the text element.
    let key_actions = KeyActions::new("key".to_string()).then(KeyAction::Down { value: 'a' });
    c.perform_actions(key_actions).await?;
    assert_eq!(elem.prop("value").await?.unwrap(), "a");
    Ok(())
}

mod firefox {
    use super::*;

    #[test]
    #[serial]
    fn actions_null_test() {
        local_tester!(actions_null, "firefox");
    }

    #[test]
    #[serial]
    fn actions_key_test() {
        local_tester!(actions_key, "firefox");
    }

    #[test]
    #[serial]
    fn actions_mouse_test() {
        local_tester!(actions_mouse, "firefox");
    }

    #[test]
    #[serial]
    fn actions_mouse_move_test() {
        local_tester!(actions_mouse_move, "firefox");
    }

    #[test]
    #[serial]
    fn actions_release_test() {
        local_tester!(actions_release, "firefox");
    }
}

mod chrome {
    use super::*;

    #[test]
    fn actions_null_test() {
        local_tester!(actions_null, "chrome");
    }

    #[test]
    fn actions_key_test() {
        local_tester!(actions_key, "chrome");
    }

    #[test]
    fn actions_mouse_test() {
        local_tester!(actions_mouse, "chrome");
    }

    #[test]
    fn actions_mouse_move_test() {
        local_tester!(actions_mouse_move, "chrome");
    }

    #[test]
    fn actions_release_test() {
        local_tester!(actions_release, "chrome");
    }
}
