#[macro_use]
extern crate serial_test_derive;
extern crate fantoccini;
extern crate futures;

use fantoccini::{error, Client, Element, Locator, Method};
use futures::{
    future::{self, Future},
    Stream as _,
};

macro_rules! tester {
        ($f:ident, $endpoint:expr) => {{
            use std::sync::{Arc, Mutex};
            use std::thread;
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
                let mut rt = tokio::runtime::current_thread::Runtime::new().unwrap();
                let mut c = rt.block_on(c).expect("failed to construct test client");
                *sid.lock().unwrap() = rt.block_on(c.session_id()).unwrap();
                let x = rt.block_on($f(c));
                rt.run().unwrap();
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

fn works_inner(c: Client) -> impl Future<Item = (), Error = error::CmdError> {
    // go to the Wikipedia page for Foobar
    c.goto("https://en.wikipedia.org/wiki/Foobar")
        .and_then(|mut this| this.find(Locator::Id("History_and_etymology")))
        .and_then(|mut e| e.text().map(move |r| (e, r)))
        .and_then(|(e, text)| {
            assert_eq!(text, "History and etymology");
            let mut c = e.client();
            c.current_url().map(move |r| (c, r))
        })
        .and_then(|(mut c, url)| {
            assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");
            // click "Foo (disambiguation)"
            c.find(Locator::Css(".mw-disambig"))
        })
        .and_then(|e| e.click())
        .and_then(|mut c| {
            // click "Foo Lake"
            c.find(Locator::LinkText("Foo Lake"))
        })
        .and_then(|e| e.click())
        .and_then(|mut c| c.current_url())
        .and_then(|url| {
            assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foo_Lake");
            Ok(())
        })
}

fn clicks_inner_by_locator(c: Client) -> impl Future<Item = (), Error = error::CmdError> {
    // go to the Wikipedia frontpage this time
    c.goto("https://www.wikipedia.org/")
        .and_then(|mut c| {
            // find, fill out, and submit the search form
            c.form(Locator::Css("#search-form"))
        })
        .and_then(|mut f| f.set(Locator::Css("input[name='search']"), "foobar"))
        .and_then(|f| f.submit())
        .and_then(|mut c| c.current_url())
        .and_then(|url| {
            // we should now have ended up in the rigth place
            assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");
            Ok(())
        })
}

fn clicks_inner(c: Client) -> impl Future<Item = (), Error = error::CmdError> {
    // go to the Wikipedia frontpage this time
    c.goto("https://www.wikipedia.org/")
        .and_then(|mut c| {
            // find, fill out, and submit the search form
            c.form(Locator::Css("#search-form"))
        })
        .and_then(|mut f| f.set_by_name("search", "foobar"))
        .and_then(|f| f.submit())
        .and_then(|mut c| c.current_url())
        .and_then(|url| {
            // we should now have ended up in the rigth place
            assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");
            Ok(())
        })
}

fn send_keys_and_clear_input_inner(c: Client) -> impl Future<Item = (), Error = error::CmdError> {
    // go to the Wikipedia frontpage this time
    c.goto("https://www.wikipedia.org/")
        .and_then(|c: Client| {
            // find search input element
            c.wait_for_find(Locator::Id("searchInput"))
        })
        .and_then(|mut e| e.send_keys("foobar").map(|_| e))
        .and_then(|mut e: Element| {
            e.prop("value")
                .map(|o| (e, o.expect("input should have value prop")))
        })
        .and_then(|(mut e, v)| {
            eprintln!("{}", v);
            assert_eq!(v.as_str(), "foobar");
            e.clear().map(|_| e)
        })
        .and_then(|mut e| {
            e.prop("value")
                .map(move |o| o.expect("input should have value prop"))
        })
        .and_then(|v| {
            assert_eq!(v.as_str(), "");
            Ok(())
        })
}

fn raw_inner(c: Client) -> impl Future<Item = (), Error = error::CmdError> {
    // go back to the frontpage
    c.goto("https://www.wikipedia.org/")
        .and_then(|mut c| {
            // find the source for the Wikipedia globe
            c.find(Locator::Css("img.central-featured-logo"))
        })
        .and_then(|mut img| {
            img.attr("src")
                .map(move |src| (img, src.expect("image should have a src")))
        })
        .and_then(move |(img, src)| {
            // now build a raw HTTP client request (which also has all current cookies)
            img.client().raw_client_for(Method::GET, &src)
        })
        .and_then(|raw| {
            // we then read out the image bytes
            raw.into_body()
                .map_err(error::CmdError::from)
                .fold(Vec::new(), |mut pixels, chunk| {
                    pixels.extend(&*chunk);
                    future::ok::<Vec<u8>, error::CmdError>(pixels)
                })
        })
        .and_then(|pixels| {
            // and voilla, we now have the bytes for the Wikipedia logo!
            assert!(pixels.len() > 0);
            println!("Wikipedia logo is {}b", pixels.len());
            Ok(())
        })
}

fn window_size_inner(c: Client) -> impl Future<Item = (), Error = error::CmdError> {
    c.goto("https://www.wikipedia.org/")
        .and_then(|mut c| c.set_window_size(500, 400).map(move |_| c))
        .and_then(|mut c| c.get_window_size())
        .and_then(|(width, height)| {
            assert_eq!(width, 500);
            assert_eq!(height, 400);
            Ok(())
        })
}

fn window_position_inner(c: Client) -> impl Future<Item = (), Error = error::CmdError> {
    c.goto("https://www.wikipedia.org/")
        .and_then(|mut c| c.set_window_size(200, 100).map(move |_| c))
        .and_then(|mut c| c.set_window_position(0, 0).map(move |_| c))
        .and_then(|mut c| c.set_window_position(1, 2).map(move |_| c))
        .and_then(|mut c| c.get_window_position())
        .and_then(|(x, y)| {
            assert_eq!(x, 1);
            assert_eq!(y, 2);
            Ok(())
        })
}

fn window_rect_inner(c: Client) -> impl Future<Item = (), Error = error::CmdError> {
    c.goto("https://www.wikipedia.org/")
        .and_then(|mut c| c.set_window_rect(0, 0, 500, 400).map(move |_| c))
        .and_then(|mut c| c.get_window_position().map(move |r| (c, r)))
        .inspect(|&(_, (x, y))| {
            assert_eq!(x, 0);
            assert_eq!(y, 0);
        })
        .and_then(|(mut c, _)| c.get_window_size().map(move |r| (c, r)))
        .inspect(|&(_, (width, height))| {
            assert_eq!(width, 500);
            assert_eq!(height, 400);
        })
        .and_then(|(mut c, _)| c.set_window_rect(1, 2, 600, 300).map(move |_| c))
        .and_then(|mut c| c.get_window_position().map(move |r| (c, r)))
        .inspect(|&(_, (x, y))| {
            assert_eq!(x, 1);
            assert_eq!(y, 2);
        })
        .and_then(move |(mut c, _)| c.get_window_size())
        .inspect(|&(width, height)| {
            assert_eq!(width, 600);
            assert_eq!(height, 300);
        })
        .map(|_| ())
}

fn finds_all_inner(c: Client) -> impl Future<Item = (), Error = error::CmdError> {
    // go to the Wikipedia frontpage this time
    c.goto("https://en.wikipedia.org/")
        .and_then(|mut c| c.find_all(Locator::Css("#p-interaction li")))
        .and_then(|es| future::join_all(es.into_iter().take(4).map(|mut e| e.text())))
        .and_then(|texts| {
            assert_eq!(
                texts,
                [
                    "Help",
                    "About Wikipedia",
                    "Community portal",
                    "Recent changes"
                ]
            );
            Ok(())
        })
}

fn persist_inner(c: Client) -> impl Future<Item = (), Error = error::CmdError> {
    c.goto("https://en.wikipedia.org/")
        .and_then(|mut c| c.persist())
}

mod chrome {
    use super::*;

    #[test]
    fn it_works() {
        tester!(works_inner, "chrome")
    }

    #[test]
    fn it_clicks() {
        tester!(clicks_inner, "chrome")
    }

    #[test]
    fn it_clicks_by_locator() {
        tester!(clicks_inner_by_locator, "chrome")
    }

    #[test]
    fn it_sends_keys_and_clear_input() {
        tester!(send_keys_and_clear_input_inner, "chrome")
    }

    #[test]
    fn it_can_be_raw() {
        tester!(raw_inner, "chrome")
    }

    #[test]
    #[ignore]
    fn it_can_get_and_set_window_size() {
        tester!(window_size_inner, "chrome")
    }

    #[test]
    #[ignore]
    fn it_can_get_and_set_window_position() {
        tester!(window_position_inner, "chrome")
    }

    #[test]
    #[ignore]
    fn it_can_get_and_set_window_rect() {
        tester!(window_rect_inner, "chrome")
    }

    #[test]
    fn it_finds_all() {
        tester!(finds_all_inner, "chrome")
    }

    #[test]
    #[ignore]
    fn it_persists() {
        tester!(persist_inner, "chrome")
    }
}

mod firefox {
    use super::*;

    #[serial]
    #[test]
    fn it_works() {
        tester!(works_inner, "firefox")
    }

    #[serial]
    #[test]
    fn it_clicks() {
        tester!(clicks_inner, "firefox")
    }

    #[serial]
    #[test]
    fn it_clicks_by_locator() {
        tester!(clicks_inner_by_locator, "firefox")
    }

    #[serial]
    #[test]
    fn it_sends_keys_and_clear_input() {
        tester!(send_keys_and_clear_input_inner, "firefox")
    }

    #[serial]
    #[test]
    fn it_can_be_raw() {
        tester!(raw_inner, "firefox")
    }

    #[test]
    #[ignore]
    fn it_can_get_and_set_window_size() {
        tester!(window_size_inner, "firefox")
    }

    #[test]
    #[ignore]
    fn it_can_get_and_set_window_position() {
        tester!(window_position_inner, "firefox")
    }

    #[test]
    #[ignore]
    fn it_can_get_and_set_window_rect() {
        tester!(window_rect_inner, "firefox")
    }

    #[serial]
    #[test]
    fn it_finds_all() {
        tester!(finds_all_inner, "firefox")
    }

    #[test]
    #[ignore]
    fn it_persists() {
        tester!(persist_inner, "firefox")
    }
}
