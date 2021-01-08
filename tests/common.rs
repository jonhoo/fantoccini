#![allow(dead_code)]

extern crate fantoccini;
extern crate futures_util;

use fantoccini::{error, Client};

use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::convert::Infallible;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Server, Request, Response, Body, StatusCode};
use tokio::fs::read_to_string;

const ASSETS_DIR: &str = "tests/test_html";

pub async fn select_client_type(s: &str) -> Result<Client, error::NewSessionError> {
    match s {
        "firefox" => {
            let mut caps = serde_json::map::Map::new();
            let opts = serde_json::json!({ "args": ["--headless"] });
            caps.insert("moz:firefoxOptions".to_string(), opts.clone());
            Client::with_capabilities("http://localhost:4444", caps).await
        }
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
        }
        browser => unimplemented!("unsupported browser backend {}", browser),
    }
}

pub fn handle_test_error(
    res: Result<Result<(), fantoccini::error::CmdError>, Box<dyn std::any::Any + Send>>,
) -> bool {
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

#[macro_export]
macro_rules! tester {
    ($f:ident, $endpoint:expr) => {{
        use std::sync::{Arc, Mutex};
        use std::thread;

        let c = common::select_client_type($endpoint);

        // we'll need the session_id from the thread
        // NOTE: even if it panics, so can't just return it
        let session_id = Arc::new(Mutex::new(None));

        // run test in its own thread to catch panics
        let sid = session_id.clone();
        let res = thread::spawn(move || {
            let mut rt = tokio::runtime::Builder::new()
                .enable_all()
                .basic_scheduler()
                .build()
                .unwrap();
            let mut c = rt.block_on(c).expect("failed to construct test client");
            *sid.lock().unwrap() = rt.block_on(c.session_id()).unwrap();
            // make sure we close, even if an assertion fails
            let x = rt.block_on(async move {
                let r = tokio::spawn($f(c.clone())).await;
                let _ = c.close().await;
                r
            });
            drop(rt);
            x.expect("test panicked")
        })
        .join();
        let success = common::handle_test_error(res);
        assert!(success);
    }};
}

#[macro_export]
macro_rules! local_tester {
    ($f:ident, $endpoint:expr) => {{
        let port: u16 = common::setup_server();
        let f = move |c: Client| async move { $f(c, port).await };
        tester!(f, $endpoint)
    }};
}

/// Sets up the server and returns the port it bound to.
pub fn setup_server() -> u16 {
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let mut rt = tokio::runtime::Builder::new()
            .enable_all()
            .basic_scheduler()
            .build()
            .unwrap();
        let _ = rt.block_on(async {
            let (socket_addr, server) = start_server();
            tx.send(socket_addr.port())
                .expect("To be able to send port");
            server.await.expect("To start the server")
        });
    });

    rx.recv().expect("To get the bound port.")
}

/// Configures and starts the server
fn start_server() -> (SocketAddr, impl Future<Output = hyper::Result<()>> + 'static) {
    let socket_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);

    let server = Server::bind(&socket_addr)
        .serve(make_service_fn(move |_| async {
             Ok::<_, Infallible>(service_fn(handle_file_request))
        }));

    let addr = server.local_addr();
    (addr, server)
}

/// Tries to return the requested html file
async fn handle_file_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let uri_path = req.uri().path().trim_matches(&['/', '\\'][..]);

    // tests only contain html files
    // needed because the content-type: text/html is returned
    if !uri_path.ends_with(".html") {
        return Ok(file_not_found())
    }

    // this does not protect against a directory traversal attack
    // but in this case it's not a risk
    let asset_file = Path::new(ASSETS_DIR).join(uri_path);

    let ctn = match read_to_string(asset_file).await {
        Ok(ctn) => ctn,
        Err(_) => return Ok(file_not_found())
    };

    let res = Response::builder()
        .header("content-type", "text/html")
        .header("content-length", ctn.len())
        .body(ctn.into())
        .unwrap();

    Ok(res)
}

/// Response returned when a file is not found or could not be read
fn file_not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::empty())
        .unwrap()
}
