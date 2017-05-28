extern crate fantoccini;

fn main() {
    use fantoccini::Client;
    let mut c = Client::new("http://localhost:4444").unwrap();
    c.goto("https://www.wikipedia.org/").unwrap();
    println!("Starting out at: {}", c.current_url().unwrap());
    println!("Initial search box says: {:?}",
             c.lookup_prop("#searchInput", "value"));
    {
        let mut f = c.form("#search-form").unwrap();
        f.set_by_name("search", "foobar").unwrap();
    }
    println!("Search box now says: {:?}",
             c.lookup_prop("#searchInput", "value"));
    {
        let f = c.form("#search-form").unwrap();
        f.submit_direct().unwrap();
    }
    let here = c.current_url().unwrap();
    println!("After submitting, we're at: {}", here);
    println!("First paragraph is:\n\t{:?}",
             c.lookup_text("#mw-content-text p:first-of-type"));
    println!("Some stuff:");
    println!("---------------------------------------------");
    println!("{:?}",
             c.lookup_html("#mw-content-text .hatnote:first-child", false));
    println!("---------------------------------------------");
    println!("{:?}",
             c.lookup_html("#mw-content-text .hatnote:first-child", true));
    println!("---------------------------------------------");
    c.click(".mw-redirect").unwrap();
    println!("After redirect, we're at: {}", c.current_url().unwrap());
    c.goto(&format!("{}", here)).unwrap();
    c.follow_link_nojs(".mw-redirect").unwrap();
    println!("After back and forth: {}", c.current_url().unwrap());

    println!("\nAnd now for something completely different");
    drop(c);
    let mut c = Client::new("http://localhost:4444").unwrap();
    c.goto("https://www.wikipedia.org/").unwrap();
    let img = c.lookup_attr("img.central-featured-logo", "src")
        .unwrap()
        .unwrap();
    let raw = c.raw_client_for(fantoccini::Method::Get, &img).unwrap();
    let mut res = raw.send().unwrap();

    use std::io::prelude::*;
    let mut pixels = Vec::new();
    res.read_to_end(&mut pixels).unwrap();
    println!("Wikipedia logo is {}b", pixels.len());
}
