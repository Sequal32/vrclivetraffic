use attohttpc;
use serde_json::Value;
use std::env;

const PROGRAM_RELEASE_URL: &str =
    "https://api.github.com/repos/sequal32/vrclivetraffic/releases/latest";

const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:53.0) Gecko/20100101 Firefox/53.0";

pub struct Updater {}

impl Updater {
    fn get_url(url: &str) -> Result<attohttpc::Response, attohttpc::Error> {
        attohttpc::get(url).header("User-Agent", USER_AGENT).send()
    }

    fn get_json_from_url(url: &str) -> Result<Value, ()> {
        // Download installer release info
        let response = match Self::get_url(url) {
            Ok(response) => response,
            Err(_) => return Err(()),
        };

        match response.json() {
            Ok(data) => Ok(data),
            Err(_) => return Err(()),
        }
    }

    fn get_latest_version_info() -> Result<String, ()> {
        let json = Self::get_json_from_url(PROGRAM_RELEASE_URL)?;

        return match json["tag_name"].as_str() {
            Some(v) => {
                // Cache
                Ok(v.to_string())
            }
            None => Err(()),
        };
    }

    pub fn get_latest_version() -> Result<String, ()> {
        Self::get_latest_version_info()
    }

    pub fn get_version() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
}
