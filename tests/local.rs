#[macro_use]
extern crate serial_test_derive;
extern crate fantoccini;
extern crate futures_util;

use fantoccini::{error, Client};
use warp::Filter;
use std::path::PathBuf;
use std::net::Ipv4Addr;

macro_rules! tester {
    // Ident should identify an async fn that takes a mut Client and a port.
    ($f:ident, $endpoint:expr) => {{
        use std::sync::{Arc, Mutex};
        use std::thread;

        let port = setup_server();

        let c = match $endpoint {
            "firefox" => {
                let mut caps = serde_json::map::Map::new();
                let opts = serde_json::json!({ "args": ["--headless"] });
                caps.insert("moz:firefoxOptions".to_string(), opts.clone());
                Client::with_capabilities("http://localhost:4444", caps)
            },
            "chrome" => {
                let mut caps = serde_json::map::Map::new();
                let opts = serde_json::json!({
                    "args": ["--headless", "--disable-gpu", "--no-sandbox", "--disable-dev-shm-usage"],
                    "binary":
                        if std::path::Path::new("/usr/bin/chromium-browser").exists() {
                            // on Ubuntu, it's called chromium-browser
                            "/usr/bin/chromium-browser"
                        } else if std::path::Path::new("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome").exists() {
                            // macOS
                            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
                        } else {
                            // elsewhere, it's just called chromium
                            "/usr/bin/chromium"
                        }
                });
                caps.insert("goog:chromeOptions".to_string(), opts.clone());

                Client::with_capabilities("http://localhost:9515", caps)
            },
            browser => unimplemented!("unsupported browser backend {}", browser),
        };

        // we'll need the session_id from the thread
        // NOTE: even if it panics, so can't just return it
        let session_id = Arc::new(Mutex::new(None));

        // run test in its own thread to catch panics
        let sid = session_id.clone();
        let success = match thread::spawn(move || {
            let mut rt = tokio::runtime::Builder::new().enable_all().basic_scheduler().build().unwrap();
            let mut c = rt.block_on(c).expect("failed to construct test client");
            *sid.lock().unwrap() = rt.block_on(c.session_id()).unwrap();
            let x = rt.block_on($f(c, port));
            drop(rt);
            x
        })
        .join()
        {
            Ok(Ok(_)) => true,
            Ok(Err(e)) => {
                eprintln!("test future failed to resolve: {:?}", e);
                false
            }
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
        };

        assert!(success);
    }};
}

lazy_static::lazy_static! {
    static ref PORT_COUNTER: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(8000);
}


fn setup_server() -> u16 {
    let port: u16;
    loop {
        let prospective_port = PORT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if port_scanner::local_port_available(prospective_port) {
            port = prospective_port;
            break
        }
    }

    std::thread::spawn(move || {

        let mut rt = tokio::runtime::Builder::new().enable_all().basic_scheduler().build().unwrap();
        let server = start_server(port);
        rt.block_on(server);
    });
    std::thread::sleep(std::time::Duration::from_secs(1));
    port
}



/// Starts the fileserver
async fn start_server(port: u16) {
    let localhost = Ipv4Addr::LOCALHOST;
    let addr = (localhost, port);

    const ASSETS_DIR: &str = "tests/test_html";
    let assets_dir: PathBuf = PathBuf::from(ASSETS_DIR);
    let routes = fileserver(assets_dir);
    warp::serve(routes).run(addr).await
}


/// Serves files under this directory.
fn fileserver(assets_dir: PathBuf) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::get()
        .and(warp::fs::dir(assets_dir))
        .and(warp::path::end())
}

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
    let handles = c.windows().await?;
    assert_eq!(handles.len(), 2);
    c.close().await
}

async fn new_window_switch(mut c: Client, _port: u16) -> Result<(), error::CmdError> {
    let handle_1 = c.window().await?;
    c.new_window(false).await?;
    let handle_2 = c.window().await?;
    assert_eq!(handle_1, handle_2, "After creating a new window, the session should not have switched to it");

    let all_handles = c.windows().await?;
    let new_window_handle = all_handles
        .into_iter()
        .find(|handle| handle != &handle_1)
        .expect("Should find a differing handle");

    c.switch_to_window(new_window_handle).await?;

    let handle_3 = c.window().await?;
    assert_ne!(handle_3, handle_2, "After switching to a new window, the handle should differ now.");

    c.close().await
}


async fn new_tab(mut c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;
    c.new_window(true).await?;
    let handles = c.windows().await?;
    assert_eq!(handles.len(), 2);
    c.close().await
}

async fn close_window(mut c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;
    let window_1 = c.window().await?;
    c.new_window(true).await?;
    let window_2 = c.window().await?;
    assert_eq!(window_1, window_2, "Creating a new window should not cause the client to switch to it.");

    let handles = c.windows().await?;
    assert_eq!(handles.len(), 2);

    c.close_window().await?;
    c.window().await.expect_err("After closing a window, the client can't find its currently selected window.");

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
    c.close_window().await.expect_err("Should get a no such window error");
    Ok(())
}


mod firefox {
    use super::*;
    #[test]
    #[serial]
    fn navigate_to_other_page() {
        tester!(goto, "firefox")
    }

    #[test]
    #[serial]
    fn new_window_test() {
        tester!(new_window, "firefox")
    }

    #[test]
    #[serial]
    fn new_window_switch_test() {
        tester!(new_window_switch, "firefox")
    }

    #[test]
    #[serial]
    fn new_tab_test() {
        tester!(new_tab, "firefox")
    }

    #[test]
    #[serial]
    fn close_window_test() {
        tester!(close_window, "firefox")
    }

    #[test]
    #[serial]
    fn double_close_window_test() {
        tester!(close_window_twice_errors, "firefox")
    }
}


mod chrome {
    use super::*;
    #[test]
    #[serial]
    fn navigate_to_other_page() {
        tester!(goto, "chrome")
    }

    #[test]
    #[serial]
    fn new_window_test() {
        tester!(new_window, "chrome")
    }

    #[test]
    #[serial]
    fn new_window_switch_test() {
        tester!(new_window_switch, "chrome")
    }

    #[test]
    #[serial]
    fn new_tab_test() {
        tester!(new_tab, "chrome")
    }

    #[test]
    #[serial]
    fn close_window_test() {
        tester!(close_window, "chrome")
    }


    #[test]
    #[serial]
    fn double_close_window_test() {
        tester!(close_window_twice_errors, "chrome")
    }
}


