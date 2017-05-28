extern crate fantoccini;

fn main() {
    use fantoccini::Client;
    let mut c = Client::new("http://localhost:4444").unwrap();
    c.goto("https://www.wikipedia.org/");
    println!("Starting out at: {}", c.current_url());
    println!("Initial search box says: {:?}",
             c.lookup_prop("#searchInput", "value"));
    {
        let mut f = c.form("#search-form").unwrap();
        f.set_by_name("search", "foobar");
    }
    println!("Search box now says: {:?}",
             c.lookup_prop("#searchInput", "value"));
    {
        let f = c.form("#search-form").unwrap();
        f.submit_direct();
    }
    let here = c.current_url();
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
    c.click(".mw-redirect");
    println!("After redirect, we're at: {}", c.current_url());
    c.goto(&format!("{}", here));
    c.follow_link_nojs(".mw-redirect");
    println!("After back and forth: {}", c.current_url());
}
