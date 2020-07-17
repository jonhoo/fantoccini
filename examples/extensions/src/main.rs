/// This example is only work with geckodriver
extern crate fantoccini;
extern crate tokio;
extern crate serde;
extern crate serde_json;

use fantoccini::{Client, ExtensionCommand, Method, WebDriverExtensionCommand};
use serde::Serialize;
use serde_json::Value;
use std::io::Error;
use std::thread::sleep;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AddonInstallParameters {
    pub path: String,
    pub temporary: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AddonUninstallParameters {
    pub id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GeckoExtensionCommand {
    InstallAddon(AddonInstallParameters),
    UninstallAddon(AddonUninstallParameters)
}

impl ExtensionCommand for GeckoExtensionCommand {
    fn method(&self) -> Method {
        Method::POST
    }

    fn endpoint(&self) -> &str {
        match self {
            Self::InstallAddon(_)=>"/moz/addon/install",
            Self::UninstallAddon(_)=>"/moz/addon/uninstall"
        }
    }
}

impl WebDriverExtensionCommand for GeckoExtensionCommand {
    fn parameters_json(&self) -> Option<Value> {
        Some(match self {
            Self::InstallAddon(param)=>serde_json::to_value(param).unwrap(),
            Self::UninstallAddon(param)=>serde_json::to_value(param).unwrap()
        })
    }
}

#[tokio::main]
async fn main()-> Result<(), Error> {
    let mut client = Client::new("http://localhost:4444").await.unwrap();

    let install_command = GeckoExtensionCommand::InstallAddon(AddonInstallParameters {
        path: String::from("/path/to/addon.xpi"),
        temporary: Some(true)
    });
    let ins_res = client.extension_command(install_command).await.expect("Can not install the addon");

    println!("Install Response: {:#?}", ins_res);

    let addon_id = ins_res.as_str().unwrap();

    sleep(Duration::from_secs(5));

    let uninstall_command = GeckoExtensionCommand::UninstallAddon(AddonUninstallParameters{
        id: String::from(addon_id)
    });

    let uns_res = client.extension_command(uninstall_command).await.expect("Can not uninstall the addon");

    println!("Uninstall Reponse: {:#?}", uns_res);

    Ok(())
}
