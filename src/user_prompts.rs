//! User prompt related functionality for WebDriver.
//!
//! See [18. User Prompts](https://www.w3.org/TR/webdriver1/#user-prompts) of the WebDriver
//! standard.
use webdriver::command::WebDriverCommand;

use crate::error;
use crate::Client;

/// `UserPrompt` enumerates the different ways a `Client` can respond to a user prompt, or alert in
/// the browser window.
#[derive(Debug, Clone)]
pub enum UserPrompt {
    /// `Accept` is equivalent to a user clicking the `OK` button in the prompt.
    Accept,
    /// `Dismiss` is equivalent to a user clicking the `Cancel` or `OK` button in the prompt,
    /// whichever is present and appears first.
    Dismiss,
}

impl Client {
    /// Sends a response to the user prompt. For the different values you can provide, see
    /// [`UserPrompt`].
    ///
    /// See [18. User Prompts](https://www.w3.org/TR/webdriver1/#user-prompts) of the WebDriver
    /// standard.
    pub async fn handle_user_prompt(&mut self, prompt: &UserPrompt) -> Result<(), error::CmdError> {
        let cmd = match prompt {
            UserPrompt::Accept => WebDriverCommand::AcceptAlert,
            UserPrompt::Dismiss => WebDriverCommand::DismissAlert,
        };
        self.issue(cmd).await?;
        Ok(())
    }
}
