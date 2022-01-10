//! Cookie-related functionality for WebDriver.
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use webdriver::command::{AddCookieParameters, WebDriverCommand};
use webdriver::common::Date;

use crate::client::Client;
use crate::error;

/// Type alias for a [cookie::Cookie]
pub type Cookie<'a> = cookie::Cookie<'a>;

/// Wrapper for serializing AddCookieParameters.
#[derive(Debug, Serialize)]
pub struct AddCookieParametersWrapper<'a> {
    /// The cookie to serialize.
    #[serde(with = "AddCookieParameters")]
    pub cookie: &'a AddCookieParameters,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) enum SameSite {
    Strict,
    Lax,
    None,
}

impl From<String> for SameSite {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_ref() {
            "strict" => SameSite::Strict,
            "lax" => SameSite::Lax,
            _ => SameSite::None,
        }
    }
}

impl From<SameSite> for String {
    fn from(s: SameSite) -> String {
        match s {
            SameSite::Strict => "Strict".to_string(),
            SameSite::Lax => "Lax".to_string(),
            SameSite::None => "None".to_string(),
        }
    }
}

/// Representation of a cookie as [defined by WebDriver](https://www.w3.org/TR/webdriver1/#cookies).
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct WebDriverCookie {
    name: String,
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    secure: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "httpOnly")]
    http_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expiry: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "sameSite")]
    same_site: Option<SameSite>,
}

impl From<AddCookieParameters> for WebDriverCookie {
    fn from(params: AddCookieParameters) -> Self {
        Self {
            name: params.name,
            value: params.value,
            path: params.path,
            domain: params.domain,
            secure: Some(params.secure),
            http_only: Some(params.httpOnly),
            expiry: params.expiry.map(|d| d.0),
            same_site: params.sameSite.map(|x| x.into()),
        }
    }
}

impl From<WebDriverCookie> for AddCookieParameters {
    fn from(cookie: WebDriverCookie) -> Self {
        Self {
            name: cookie.name,
            value: cookie.value,
            path: cookie.path,
            domain: cookie.domain,
            secure: cookie.secure.unwrap_or_default(),
            httpOnly: cookie.http_only.unwrap_or_default(),
            expiry: cookie.expiry.map(Date),
            sameSite: cookie.same_site.map(|x| x.into()),
        }
    }
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

        if let Some(same_site) = webdriver_cookie.same_site {
            use cookie::SameSite as CookieSameSite;
            cookie.set_same_site(match same_site {
                SameSite::Strict => CookieSameSite::Strict,
                SameSite::Lax => CookieSameSite::Lax,
                SameSite::None => CookieSameSite::None,
            });
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
        let same_site = cookie.same_site().map(|x| {
            use cookie::SameSite as CookieSameSite;
            match x {
                CookieSameSite::Strict => SameSite::Strict,
                CookieSameSite::Lax => SameSite::Lax,
                CookieSameSite::None => SameSite::None,
            }
        });

        Self {
            name,
            value,
            path,
            domain,
            secure,
            http_only,
            expiry,
            same_site,
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

    /// Add the specified cookie.
    ///
    /// See [16.3 Add Cookie](https://www.w3.org/TR/webdriver1/#add-cookie) of the
    /// WebDriver standard.
    pub async fn add_cookie(&mut self, cookie: Cookie<'static>) -> Result<(), error::CmdError> {
        let webdriver_cookie: WebDriverCookie = cookie.into();
        self.issue(WebDriverCommand::AddCookie(webdriver_cookie.into()))
            .await?;
        Ok(())
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
