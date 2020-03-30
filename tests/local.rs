#[macro_use]
extern crate serial_test_derive;
extern crate fantoccini;
extern crate futures_util;

use fantoccini::{error, Client};
use std::net::{Ipv4Addr, TcpListener, SocketAddr};
use std::path::PathBuf;
use warp::Filter;
use futures_core::Future;
use fantoccini::error::CmdError;
use warp::test::request;


async fn select_client_type(s: &str) -> Result<Client, error::NewSessionError> {
    match s {
        "firefox" => {
            let mut caps = serde_json::map::Map::new();
            let opts = serde_json::json!({ "args": ["--headless"] });
            caps.insert("moz:firefoxOptions".to_string(), opts.clone());
            Client::with_capabilities("http://localhost:4444", caps).await
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

            Client::with_capabilities("http://localhost:9515", caps).await
        },
        browser => unimplemented!("unsupported browser backend {}", browser),
    }
}

fn handle_test_error(res: Result<Result<(), CmdError>, Box<dyn std::any::Any + Send>>) -> bool{
    match res {
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
    }
}

macro_rules! local_tester {
    ($f:ident, $endpoint:expr) => {{
        use std::sync::{Arc, Mutex};
        use std::thread;

        let port: u16 = setup_server();
        let c = select_client_type($endpoint);

        // we'll need the session_id from the thread
        // NOTE: even if it panics, so can't just return it
        let session_id = Arc::new(Mutex::new(None));

        // run test in its own thread to catch panics
        let sid = session_id.clone();
        let res = thread::spawn(move || {
        let mut rt = tokio::runtime::Builder::new().enable_all().basic_scheduler().build().unwrap();
            let mut c = rt.block_on(c).expect("failed to construct test client");
            *sid.lock().unwrap() = rt.block_on(c.session_id()).unwrap();
            let x = rt.block_on($f(c, port));
            drop(rt);
            x
        })
        .join();
        let success = handle_test_error(res);
        assert!(success);
    }}
}


macro_rules! tester {
    // Ident should identify an async fn that takes just a Client.
    ($f:ident, $endpoint:expr) => {{
        use std::sync::{Arc, Mutex};
        use std::thread;

        let c = select_client_type($endpoint);

        // we'll need the session_id from the thread
        // NOTE: even if it panics, so can't just return it
        let session_id = Arc::new(Mutex::new(None));

        // run test in its own thread to catch panics
        let sid = session_id.clone();
        let res = thread::spawn(move || {
            let mut rt = tokio::runtime::Builder::new().enable_all().basic_scheduler().build().unwrap();
            let mut c = rt.block_on(c).expect("failed to construct test client");
            *sid.lock().unwrap() = rt.block_on(c.session_id()).unwrap();
            let x = rt.block_on($f(c));
            drop(rt);
            x
        });
        let success = handle_test_error(res);
        assert!(success);
    }};

}

fn setup_server() -> u16 {
    let (tx, rx) = std::sync::mpsc::channel();

     std::thread::spawn(move || {
         let mut rt = tokio::runtime::Builder::new().enable_all().basic_scheduler().build().unwrap();
         let _ = rt.block_on(async {
             let socket_addr = TcpListener::bind("127.0.0.1:0")
                .unwrap()
                .local_addr()
                .unwrap();
             let (socket_addr, server) = start_server(socket_addr);
             tx.send(socket_addr.port()).expect("To be able to send");
             server
        });
    });

    let port = rx.recv().expect("to get port");
    println!("got port: {}", port);

    let url = format!("http://localhost:{}/sample_page.html", port);
    let mut count = 0;
    loop {
        match reqwest::blocking::get(&url) {
            Ok(..) => break,
            e => {
                println!("error: {:?}", e);
                count += 1;
                if count > 100 {
                    break;
                }

                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
    port
}

/// Starts the fileserver
fn start_server(socket_addr: SocketAddr) -> (SocketAddr, impl Future<Output = ()> + 'static) {
    const ASSETS_DIR: &str = "tests/test_html";
    let assets_dir: PathBuf = PathBuf::from(ASSETS_DIR);
    let routes = fileserver(assets_dir);
    warp::serve(routes).bind_ephemeral(socket_addr)
}

/// Serves files under this directory.
fn fileserver(
    assets_dir: PathBuf,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
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
    assert_eq!(
        handle_1, handle_2,
        "After creating a new window, the session should not have switched to it"
    );

    let all_handles = c.windows().await?;
    let new_window_handle = all_handles
        .into_iter()
        .find(|handle| handle != &handle_1)
        .expect("Should find a differing handle");

    c.switch_to_window(new_window_handle).await?;

    let handle_3 = c.window().await?;
    assert_ne!(
        handle_3, handle_2,
        "After switching to a new window, the handle should differ now."
    );

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
        local_tester!(new_window_switch, "firefox")
    }

    #[test]
    #[serial]
    fn new_tab_test() {
        local_tester!(new_tab, "firefox")
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
        local_tester!(new_window_switch, "chrome")
    }

    #[test]
    #[serial]
    fn new_tab_test() {
        local_tester!(new_tab, "chrome")
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
