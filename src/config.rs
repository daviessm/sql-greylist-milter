use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
pub struct Config {
    db_config: DbConfig,
    listen_address: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct DbConfig {
    db_type: String,
    db_user: String,
    db_pass: String,
    db_host: String,
    db_port: String,
    db_name: String,
}

impl Config {
    pub fn get_db_url(&self) -> String {
        format!(
            "{}://{}:{}@{}:{}/{}",
            self.db_config.db_type,
            self.db_config.db_user,
            self.db_config.db_pass,
            self.db_config.db_host,
            self.db_config.db_port,
            self.db_config.db_name,
        )
    }

    pub fn get_listen_address(&self) -> &String {
        &self.listen_address
    }
}
