use proc_macro2::TokenTree;
use std::vec::IntoIter;

use proc_macro2::{Ident, Span};
use quote::quote;

#[proc_macro_attribute]
pub fn test(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    core(attr.into(), input.into()).into()
}

fn core(
    attr: proc_macro2::TokenStream,
    input: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let original_test_code: syn::ItemFn = syn::parse2(input).unwrap();
    let attrs = get_raw_args(attr).into_iter();

    let fn_name = &original_test_code.sig.ident;

    // if you don't clone the attrs
    // the find will alter / mutate the iterator such that the second search won't find anything
    // cloning isn't ideal but works for now
    let code_to_call_chrome_variant_of_original_test_code =
        generate_test_fn(fn_name, "chrome", &mut attrs.clone());
    let code_to_call_firefox_variant_of_original_test_code =
        generate_test_fn(fn_name, "firefox", &mut attrs.clone());

    let expanded = quote! {
        #[cfg(test)]
        pub mod #fn_name {
            use super::*;
            use fantoccini::common::{make_capabilities, make_url, handle_test_error};
            use fantoccini::{ClientBuilder, Client};

            #original_test_code

            #code_to_call_chrome_variant_of_original_test_code
            #code_to_call_firefox_variant_of_original_test_code
        }
    };

    expanded
}

fn generate_test_fn(
    func: &Ident,
    browser: &str,
    attrs: &mut IntoIter<String>,
) -> proc_macro2::TokenStream {
    let test_name = syn::Ident::new(&browser, Span::call_site());

    let code_to_run_test_in_seperate_thread =
        generate_code_to_run_test_in_seperate_thread(browser, func.clone());

    match attrs.find(|attr| attr == browser) {
        Some(_) => {
            let stream = quote! {
                #[tokio::test]
                #[serial_test::serial(#browser)]
                async fn #test_name(){
                    #code_to_run_test_in_seperate_thread
                }
            };
            stream
        }
        None => quote! {},
    }
}

fn generate_code_to_run_test_in_seperate_thread(
    browser: &str,
    func: syn::Ident,
) -> proc_macro2::TokenStream {
    quote! {
        use std::thread;

        let url = make_url(#browser);
        let caps = make_capabilities(#browser);

        // what was the session_id for?
        // the session id was never used for my test case

        let response = thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            let client = runtime.block_on(async move {
                ClientBuilder::native()
                    .capabilities(caps)
                    .connect(url)
                    .await
            }).expect("failed to construct test client");

            // make sure we close, even if an assertion fails
            let result = runtime.block_on(async move {
                let result = tokio::spawn(#func(client.clone())).await;
                let _ = client.close().await;
                result
            });
            drop(runtime);
            result.expect("test panicked")
        })
        .join();
        let success = handle_test_error(response);
        assert!(success);
    }
}

#[cfg(test)]
mod gen_test_fn {
    use proc_macro2::Span;

    #[test]
    fn gen_test_fn_firefox() {
        let fname = syn::Ident::new("test", Span::call_site());

        let a: proc_macro2::TokenStream = super::gen_test_fn(
            &fname,
            "firefox",
            &mut vec!["firefox".to_string()].into_iter(),
        );

        assert_eq!(
            a.to_string(),
            "# [tokio :: test] # [serial_test :: serial (\"firefox\")] async fn firefox () { let url = make_url (\"firefox\") ; let caps = make_capabilities (\"firefox\") ; use std :: thread ; let res = thread :: spawn (move || { let rt = tokio :: runtime :: Builder :: new_current_thread () . enable_all () . build () . unwrap () ; let client = rt . block_on (async move { ClientBuilder :: native () . capabilities (caps) . connect (url) . await }) . expect (\"failed to construct test client\") ; let x = rt . block_on (async move { let r = tokio :: spawn (test (client . clone ())) . await ; let _ = client . close () . await ; r }) ; drop (rt) ; x . expect (\"test panicked\") }) . join () ; let success = handle_test_error (res) ; assert ! (success) ; }"
        )
    }

    #[test]
    fn gen_test_fn_chrome() {
        let fname = syn::Ident::new("test", Span::call_site());

        let a: proc_macro2::TokenStream = super::gen_test_fn(
            &fname,
            "chrome",
            &mut vec!["chrome".to_string()].into_iter(),
        );

        assert_eq!(
            a.to_string(),
            "# [tokio :: test] # [serial_test :: serial (\"chrome\")] async fn chrome () { let url = make_url (\"chrome\") ; let caps = make_capabilities (\"chrome\") ; use std :: thread ; let res = thread :: spawn (move || { let rt = tokio :: runtime :: Builder :: new_current_thread () . enable_all () . build () . unwrap () ; let client = rt . block_on (async move { ClientBuilder :: native () . capabilities (caps) . connect (url) . await }) . expect (\"failed to construct test client\") ; let x = rt . block_on (async move { let r = tokio :: spawn (test (client . clone ())) . await ; let _ = client . close () . await ; r }) ; drop (rt) ; x . expect (\"test panicked\") }) . join () ; let success = handle_test_error (res) ; assert ! (success) ; }"
        )
    }
}

fn get_raw_args(attr: proc_macro2::TokenStream) -> Vec<String> {
    let mut attrs = attr.into_iter().collect::<Vec<TokenTree>>();
    let mut raw_args: Vec<String> = Vec::new();
    while !attrs.is_empty() {
        match attrs.remove(0) {
            TokenTree::Ident(id) => {
                let name = id.to_string();
                raw_args.push(name);
            }
            TokenTree::Literal(literal) => {
                let string_literal = literal.to_string();
                if !string_literal.starts_with('\"') || !string_literal.ends_with('\"') {
                    panic!("Expected a string literal, got '{}'", string_literal);
                }
                // Hacky way of getting a string without the enclosing quotes
                raw_args.push(string_literal[1..string_literal.len() - 1].to_string());
            }
            x => {
                panic!("Expected either strings or literals as args, not {}", x);
            }
        }
        if !attrs.is_empty() {
            match attrs.remove(0) {
                TokenTree::Punct(p) if p.as_char() == ',' => {}
                x => {
                    panic!("Expected , between args, not {}", x);
                }
            }
        }
    }
    raw_args
}

#[cfg(test)]
mod get_raw_args {
    use super::get_raw_args;
    use quote::quote;
    #[test]
    fn test_get_raw_args_chrome_literal() {
        let attr = proc_macro2::TokenStream::from(quote! {"chrome"});
        let raw_args = get_raw_args(attr);
        assert_eq!(raw_args, vec!["chrome".to_string()]);
    }

    #[test]
    fn test_get_raw_args_firefox_literal() {
        let attr = proc_macro2::TokenStream::from(quote! {"firefox"});
        let raw_args = get_raw_args(attr);
        assert_eq!(raw_args, vec!["firefox".to_string()]);
    }

    #[test]
    fn test_get_raw_args__chrome_literal_firefox_literal() {
        let attr = proc_macro2::TokenStream::from(quote! {"chrome", "firefox"});
        let raw_args = get_raw_args(attr);
        assert_eq!(raw_args, vec!["chrome".to_string(), "firefox".to_string()]);
    }

    #[test]
    fn test_get_raw_args_chrome_ident_firefox_literal() {
        let attr = proc_macro2::TokenStream::from(quote! {chrome, "firefox"});
        let raw_args = get_raw_args(attr);
        assert_eq!(raw_args, vec!["chrome".to_string(), "firefox".to_string()]);
    }

    #[test]
    fn test_get_raw_args_chrome_ident_firefox_ident() {
        let attr = proc_macro2::TokenStream::from(quote! {chrome, firefox});
        let raw_args = get_raw_args(attr);
        assert_eq!(raw_args, vec!["chrome".to_string(), "firefox".to_string()]);
    }

    #[test]
    fn test_get_raw_args_chrome_ident() {
        let attr = proc_macro2::TokenStream::from(quote! {chrome});
        let raw_args = get_raw_args(attr);
        assert_eq!(raw_args, vec!["chrome".to_string(),]);
    }
}
