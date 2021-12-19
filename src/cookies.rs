//! Cookie-related functionality for WebDriver.
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use webdriver::command::WebDriverCommand;

use crate::client::Client;
use crate::error;

/// Type alias for a [cookie::Cookie]
pub type Cookie<'a> = cookie::Cookie<'a>;

/// Representation of a cookie as [defined by WebDriver](https://www.w3.org/TR/webdriver1/#cookies).
#[derive(Debug, Deserialize, Serialize)]
struct WebDriverCookie {
    name: String,
    value: String,
    path: Option<String>,
    domain: Option<String>,
    secure: Option<bool>,
    #[serde(rename = "httpOnly")]
    http_only: Option<bool>,
    expiry: Option<u64>,
}

impl From<WebDriverCookie> for Cookie<'static> {
    fn from(webdriver_cookie: WebDriverCookie) -> Self {
        let mut cookie = cookie::Cookie::new(webdriver_cookie.name, webdriver_cookie.value);

        if let Some(path) = webdriver_cookie.path {
            cookie.set_path(path);
        }

        if let Some(domain) = webdriver_cookie.domain {
            cookie.set_domain(domain);
        }

        if let Some(secure) = webdriver_cookie.secure {
            cookie.set_secure(secure);
        }

        if let Some(http_only) = webdriver_cookie.http_only {
            cookie.set_http_only(http_only);
        }

        if let Some(expiry) = webdriver_cookie.expiry {
            let dt = OffsetDateTime::from_unix_timestamp(expiry as i64).ok();
            cookie.set_expires(dt);
        }

        cookie
    }
}

impl<'a> From<Cookie<'a>> for WebDriverCookie {
    fn from(cookie: Cookie<'a>) -> Self {
        let name = cookie.name().to_string();
        let value = cookie.value().to_string();
        let path = cookie.path().map(String::from);
        let domain = cookie.domain().map(String::from);
        let secure = cookie.secure();
        let http_only = cookie.http_only();
        let expiry = cookie
            .expires()
            .and_then(|e| e.datetime().map(|dt| dt.unix_timestamp() as u64));

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

        let webdriver_cookies: Vec<WebDriverCookie> = serde_json::from_value(resp)?;
        let cookies: Vec<Cookie<'static>> = webdriver_cookies
            .into_iter()
            .map(|raw_cookie| raw_cookie.into())
            .collect();

        Ok(cookies)
    }

    /// Get a single named cookie associated with the current document.
    ///
    /// See [16.2 Get Named Cookie](https://www.w3.org/TR/webdriver1/#get-named-cookie) of the
    /// WebDriver standard.
    pub async fn get_named_cookie(
        &mut self,
        name: &str,
    ) -> Result<Cookie<'static>, error::CmdError> {
        let resp = self
            .issue(WebDriverCommand::GetNamedCookie(name.to_string()))
            .await?;
        let webdriver_cookie: WebDriverCookie = serde_json::from_value(resp)?;
        Ok(webdriver_cookie.into())
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
