use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
pub struct Config {
    db_config: DbConfig,
    listen_address: String,
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
}
