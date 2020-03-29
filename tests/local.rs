#[macro_use]
extern crate serial_test_derive;
extern crate fantoccini;
extern crate futures_util;

use fantoccini::{error, Client, Locator, Method};

use futures_util::future;
use futures_util::TryFutureExt;
use std::time::Duration;
use url::Url;

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

fn setup_server() -> u16 {
    let port = port_scanner::local_ports_available_range(8000..10_000).pop().expect("no available port");
    std::thread::spawn(move || {

        let mut rt = tokio::runtime::Builder::new().enable_all().basic_scheduler().build().unwrap();
        let server = start_server(port);
        rt.block_on(server);
    });
    std::thread::sleep(std::time::Duration::from_secs(1));
    port
}

use std::path::PathBuf;

#[rustfmt::skip]
use warp::{
    filters::BoxedFilter,
    fs::File,
    path::Peek,
    path,
    Filter, Reply,
};

async fn start_server(port: u16) {

    let localhost = [0, 0, 0, 0];
    let addr = (localhost, port);

    // You will need to change this if you use this as a template for your application.

    const ASSETS_DIR: &str = "tests/test_html";
    let assets_dir: PathBuf = PathBuf::from(ASSETS_DIR);
    let routes = static_files_handler(assets_dir);
    warp::serve(routes).run(addr).await
}


/// Expose filters that work with static files
fn static_files_handler(assets_dir: PathBuf) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
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
    Ok(())
}

async fn new_window(mut c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;
    c.new_window(false).await?;
    let handles = c.get_window_handles().await?;
    assert_eq!(handles.len(), 2);
    Ok(())
}


async fn new_tab(mut c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;
    c.new_window(true).await?;
    let handles = c.get_window_handles().await?;
    assert_eq!(handles.len(), 2);
    Ok(())
}

async fn close_window(mut c: Client, port: u16) -> Result<(), error::CmdError> {
    let url = sample_page_url(port);
    c.goto(&url).await?;
    c.new_window(true).await?;
    let handles = c.get_window_handles().await?;
    assert_eq!(handles.len(), 2);
    c.close_window().await?;
    let handles = c.get_window_handles().await?;
    assert_eq!(handles.len(), 1);
    Ok(())
}


#[test]
fn navigate_to_other_page() {
    tester!(goto, "firefox")
}

#[test]
fn new_window_test() {
    tester!(new_window, "firefox")
}

#[test]
fn new_tab_test() {
    tester!(new_tab, "firefox")
}

#[test]
fn close_window_test() {
    tester!(close_window, "firefox")
}

