use std::{str::FromStr, env};

use config::{Config, ConfigError, File};
use ipnet::IpNet;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    milter: Milter,
    database: Database,
    greylist: Option<Greylist>,
    spam: Option<Spam>,
    recipient_rewriting: Option<RecipientRewriting>,
}

#[derive(Debug, Deserialize)]
pub struct Milter {
    listen_address: String,
}

#[derive(Debug, Deserialize)]
struct Database {
    r#type: String,
    user: String,
    pass: String,
    host: String,
    port: u16,
    db_name: String,
}

#[derive(Debug, Deserialize)]
struct Greylist {
    allow_from_ranges: Vec<String>,
    greylist_time_seconds: i64,
}

#[derive(Debug, Deserialize)]
struct Spam {
    reject_message: String,
    recipients: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RecipientRewriting {
    rewrites: Vec<Rewrite>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Rewrite {
    pub old_to: String,
    pub action: ChangeRecipientAction,
    pub new_to: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub enum ChangeRecipientAction {
    Add,
    Replace,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::with_name(&format!("/etc/{}.toml", env!("CARGO_PKG_NAME"))))
            .build()?;

        s.try_deserialize()
    }

    pub fn get_db_url(&self) -> String {
        format!(
            "{}://{}:{}@{}:{}/{}",
            self.database.r#type,
            self.database.user,
            self.database.pass,
            self.database.host,
            self.database.port,
            self.database.db_name,
        )
    }

    pub fn get_listen_address(&self) -> &String {
        &self.milter.listen_address
    }

    pub fn get_allow_from_networks(&self) -> Vec<IpNet> {
        if let Some(greylist) = &self.greylist {
            greylist.allow_from_ranges
                .iter()
                .map(|net| IpNet::from_str(net.as_str()).expect("Unable to parse network"))
                .collect()
        } else {
            vec![]
        }
    }

    pub fn get_greylist_time_seconds(&self) -> i64 {
        match &self.greylist {
            Some(greylist) => greylist.greylist_time_seconds,
            None => 0,
        }
    }

    pub fn get_blocked_senders(&self) -> Vec<String> {
        match &self.spam {
            Some(spam) => spam.recipients.clone(),
            None => vec![],
        }
    }

    pub fn get_spam_message(&self) -> Option<String> {
        match &self.spam {
            Some(spam) => Some(spam.reject_message.clone()),
            None => None,
        }
    }

    pub fn get_rewrites(&self) -> Vec<Rewrite> {
        match &self.recipient_rewriting {
            Some(rewrites) => rewrites.rewrites.clone(),
            None => vec![],
        }
    }
}
