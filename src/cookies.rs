//! Cookie-related functionality for WebDriver.
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use webdriver::command::WebDriverCommand;

use crate::client::Client;
use crate::error;

/// Type alias for a `cookie::Cookie`
pub type Cookie<'a> = cookie::Cookie<'a>;

/// JSON representation of a cookie as [defined by WebDriver](https://www.w3.org/TR/webdriver1/#cookies).
#[derive(Debug, Deserialize, Serialize)]
struct JsonCookie {
    name: String,
    value: String,
    path: Option<String>,
    domain: Option<String>,
    secure: Option<bool>,
    #[serde(rename = "httpOnly")]
    http_only: Option<bool>,
    expiry: Option<u64>,
}

impl Into<Cookie<'static>> for JsonCookie {
    fn into(self) -> Cookie<'static> {
        let mut cookie = cookie::Cookie::new(self.name, self.value);

        if let Some(path) = self.path {
            cookie.set_path(path);
        }

        if let Some(domain) = self.domain {
            cookie.set_domain(domain);
        }

        if let Some(secure) = self.secure {
            cookie.set_secure(secure);
        }

        if let Some(http_only) = self.http_only {
            cookie.set_http_only(http_only);
        }

        if let Some(expiry) = self.expiry {
            let dt = OffsetDateTime::from_unix_timestamp(expiry as i64);
            cookie.set_expires(dt);
        }

        cookie
    }
}

impl From<Cookie<'static>> for JsonCookie {
    fn from(cookie: Cookie<'static>) -> Self {
        let name = cookie.name().to_string();
        let value = cookie.value().to_string();
        let path = cookie.path().map(String::from);
        let domain = cookie.domain().map(String::from);
        let secure = cookie.secure();
        let http_only = cookie.http_only();
        let expiry = cookie.expires().map(|dt| dt.unix_timestamp() as u64);

        Self {
            name,
            value,
            path,
            domain,
            secure,
            http_only,
            expiry,
        }
    }
}

/// [Cookies](https://www.w3.org/TR/webdriver1/#cookies)
impl Client {
    /// Get all cookies associated with the current document.
    ///
    /// See [16.1 Get All Cookies](https://www.w3.org/TR/webdriver1/#get-all-cookies) of the
    /// WebDriver standard.
    pub async fn get_all_cookies(&mut self) -> Result<Vec<Cookie<'static>>, error::CmdError> {
        let resp = self.issue(WebDriverCommand::GetCookies).await?;

        let json_cookies: Vec<JsonCookie> = serde_json::from_value(resp)?;
        let cookies: Vec<Cookie<'static>> = json_cookies
            .into_iter()
            .map(|raw_cookie| raw_cookie.into())
            .collect();

        Ok(cookies)
    }

    /// Get a single named cookie associated with the current document.
    ///
    /// See [16.2 Get Named Cookie](https://www.w3.org/TR/webdriver1/#get-named-cookie) of the
    /// WebDriver standard.
    pub async fn get_named_cookie(&mut self, name: &str) -> Result<Cookie<'static>, error::CmdError> {
        let resp = self.issue(WebDriverCommand::GetNamedCookie(name.to_string())).await?;
        let json_cookie: JsonCookie = serde_json::from_value(resp)?;
        Ok(json_cookie.into())
    }

    /// Delete a single cookie from the current document.
    ///
    /// See [16.4 Delete Cookie](https://www.w3.org/TR/webdriver1/#delete-cookie) of the
    /// WebDriver standard.
    pub async fn delete_cookie(&mut self, name: &str) -> Result<(), error::CmdError> {
        self.issue(WebDriverCommand::DeleteCookie(name.to_string()))
            .await
            .map(|_| ())
    }

    /// Delete all cookies from the current document.
    ///
    /// See [16.5 Delete All Cookies](https://www.w3.org/TR/webdriver1/#delete-all-cookies) of the
    /// WebDriver standard.
    pub async fn delete_all_cookies(&mut self) -> Result<(), error::CmdError> {
        self.issue(WebDriverCommand::DeleteCookies)
            .await
            .map(|_| ())
    }
}
