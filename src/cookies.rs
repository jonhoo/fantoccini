use serde_json::Value as Json;
use webdriver::command::WebDriverCommand;

use crate::client::Client;
use crate::error;

/// Key names for cookie fields used by WebDriver JSON.
const COOKIE_NAME: &str = "name";
const COOKIE_VALUE: &str = "value";
const COOKIE_PATH: &str = "path";
const COOKIE_DOMAIN: &str = "domain";
const COOKIE_SECURE: &str = "secure";
const COOKIE_HTTP_ONLY: &str = "httpOnly";
const COOKIE_EXPIRY: &str = "expiry";

/// Build a `cookie::Cookie` from raw JSON.
fn json_to_cookie(raw_cookie: &serde_json::Map<String, Json>) -> cookie::Cookie<'static> {
    // Required keys
    let name = raw_cookie.get(COOKIE_NAME).and_then(|v| v.as_str()).unwrap().to_string();
    let value = raw_cookie.get(COOKIE_VALUE).and_then(|v| v.as_str()).unwrap().to_string();

    let mut cookie = cookie::Cookie::new(name, value);

    // Optional keys
    let path = raw_cookie.get(COOKIE_PATH).and_then(|v| v.as_str()).map(String::from);
    let domain = raw_cookie.get(COOKIE_DOMAIN).and_then(|v| v.as_str()).map(String::from);
    let secure = raw_cookie.get(COOKIE_SECURE).and_then(|v| v.as_bool());
    let http_only = raw_cookie.get(COOKIE_HTTP_ONLY).and_then(|v| v.as_bool());
    let expiry = raw_cookie.get(COOKIE_EXPIRY).and_then(|v| v.as_u64());

    if let Some(path) = path {
        cookie.set_path(path);
    }

    if let Some(domain) = domain {
        cookie.set_domain(domain);
    }

    if let Some(secure) = secure {
        cookie.set_secure(secure);
    }

    if let Some(http_only) = http_only {
        cookie.set_http_only(http_only);
    }

    if let Some(_expiry) = expiry {
        todo!()
    }

    cookie
}

/// Serialize a `cookie::Cookie` to JSON.
#[allow(unused)]
fn cookie_to_json(cookie: &cookie::Cookie<'_>) -> Json {
    let mut json = serde_json::json!(
        { COOKIE_NAME: cookie.name(), COOKIE_VALUE: cookie.value() }
    );

    if let Some(path) = cookie.path() {
        json[COOKIE_PATH] = Json::String(path.to_string());
    }

    if let Some(domain) = cookie.domain() {
        json[COOKIE_DOMAIN] = Json::String(domain.to_string());
    }

    if let Some(secure) = cookie.secure() {
        json[COOKIE_SECURE] = Json::Bool(secure);
    }

    if let Some(http_only) = cookie.http_only() {
        json[COOKIE_HTTP_ONLY] = Json::Bool(http_only);
    }

    if let Some(_expiry) = cookie.expires() {
        todo!()
    }

    json
}

/// [Cookies](https://www.w3.org/TR/webdriver2/#cookies)
impl Client {
    /// Get all cookies associated with the current document.
    ///
    /// See [16.1 Get All Cookies](https://www.w3.org/TR/webdriver2/#get-all-cookies) of the
    /// WebDriver standard.
    pub async fn get_all_cookies(&mut self) -> Result<Vec<cookie::Cookie<'_>>, error::CmdError> {
        let resp = self.issue(WebDriverCommand::GetCookies).await?;

        let raw_cookies = resp.as_array();
        if raw_cookies.is_none() {
            let err =
                error::CmdError::UnexpectedJson("expected a JSON array of cookie objects".to_string());
            return Err(err);
        }

        let raw_cookies = raw_cookies.unwrap();
        let mut cookies = Vec::new();

        for raw_cookie in raw_cookies {
            let raw_cookie = raw_cookie.as_object();
            if raw_cookie.is_none() {
                let err =
                    error::CmdError::UnexpectedJson("expected a JSON object for cookie".to_string());
                return Err(err);
            }

            cookies.push(json_to_cookie(raw_cookie.unwrap()));
        }

        Ok(cookies)
    }

    /// Get a single named cookie associated with the current document.
    ///
    /// See [16.2 Get Named Cookie](https://www.w3.org/TR/webdriver2/#get-named-cookie) of the
    /// WebDriver standard.
    pub async fn get_named_cookie(&mut self, name: &str) -> Result<cookie::Cookie<'_>, error::CmdError> {
        self.issue(WebDriverCommand::GetNamedCookie(name.to_string())).await
            .and_then(|raw_cookie| {
                match raw_cookie.as_object() {
                    None => {
                        let err =
                            error::CmdError::UnexpectedJson("expected a JSON object".to_string());
                        Err(err)
                    }
                    Some(v) => Ok(json_to_cookie(v)),
                }
            })
    }

    /// Delete a single cookie from the current document.
    ///
    /// See [16.4 Delete Cookie](https://www.w3.org/TR/webdriver2/#delete-cookie) of the
    /// WebDriver standard.
    pub async fn delete_cookie(&mut self, name: &str) -> Result<Json, error::CmdError> {
        self.issue(WebDriverCommand::DeleteCookie(name.to_string())).await
    }

    /// Delete all cookies from the current document.
    ///
    /// See [16.5 Delete All Cookies](https://www.w3.org/TR/webdriver2/#delete-all-cookies) of the
    /// WebDriver standard.
    pub async fn delete_all_cookies(&mut self) -> Result<Json, error::CmdError> {
        self.issue(WebDriverCommand::DeleteCookies).await
    }
}
