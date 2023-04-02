//! Previous methods common to the test suite

use serde_json::map;

use crate::error;

/// makes capabilities for the given browser
pub fn make_capabilities(browser: &str) -> map::Map<String, serde_json::Value> {
    match browser {
        "firefox" => {
            let mut caps = serde_json::map::Map::new();
            let opts = serde_json::json!({ "args": ["--headless"] });
            caps.insert("moz:firefoxOptions".to_string(), opts);
            caps
        }
        "chrome" => {
            let mut caps = serde_json::map::Map::new();
            let opts = serde_json::json!({
                "args": ["--headless", "--disable-gpu", "--no-sandbox", "--disable-dev-shm-usage"],
            });
            caps.insert("goog:chromeOptions".to_string(), opts);
            caps
        }
        browser => unimplemented!("unsupported browser backend {}", browser),
    }
}

/// generates a wedriver url for the given browser
pub fn make_url(browser: &str) -> &'static str {
    match browser {
        "firefox" => "http://localhost:4444",
        "chrome" => "http://localhost:9515",
        browser => unimplemented!("unsupported browser backend {}", browser),
    }
}

/// handle test errors
pub fn handle_test_error(
    res: Result<Result<(), error::CmdError>, Box<dyn std::any::Any + Send>>,
) -> bool {
    match res {
        Ok(Ok(_)) => true,
        Ok(Err(e)) => match e {
            error::CmdError::Standard(error) => {
                eprintln!("{:?}: {:?}", error.error, error.message);
                false
            }
            _ => {
                eprintln!("test future failed to resolve: {:?}", e);
                false
            }
        },
        Err(e) => {
            if let Some(e) = e.downcast_ref::<error::CmdError>() {
                eprintln!("test future panicked: {:?}", e);
            } else if let Some(e) = e.downcast_ref::<error::NewSessionError>() {
                eprintln!("test future panicked: {:?}", e);
            } else {
                eprintln!("test future panicked; an assertion probably failed");
            }
            false
        }
    }
}
