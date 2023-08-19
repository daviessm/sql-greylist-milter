use std::str::FromStr;

use ipnet::IpNet;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
pub struct Config {
    db_config: DbConfig,
    listen_address: String,
    allow_from_ranges: Option<Vec<String>>,
    greylist_time_seconds: i64,
    blocked_senders: Option<Vec<String>>,
    spam_message: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct DbConfig {
    db_type: String,
    user: String,
    pass: String,
    host: String,
    port: u16,
    db_name: String,
}

impl Config {
    pub fn get_db_url(&self) -> String {
        format!(
            "{}://{}:{}@{}:{}/{}",
            self.db_config.db_type,
            self.db_config.user,
            self.db_config.pass,
            self.db_config.host,
            self.db_config.port,
            self.db_config.db_name,
        )
    }

    pub fn get_listen_address(&self) -> &String {
        &self.listen_address
    }

    pub fn get_allow_from_networks(&self) -> Vec<IpNet> {
        if let Some(allow_from_ranges) = &self.allow_from_ranges {
            allow_from_ranges
                .iter()
                .map(|net| IpNet::from_str(net.as_str()).expect("Unable to parse network"))
                .collect()
        } else {
            vec![]
        }
    }

    pub fn get_greylist_time_seconds(&self) -> i64 {
        self.greylist_time_seconds
    }

    pub fn get_blocked_senders(&self) -> Option<Vec<String>> {
        self.blocked_senders.clone()
    }

    pub fn get_spam_message(&self) -> Option<String> {
        self.spam_message.clone()
    }
}
