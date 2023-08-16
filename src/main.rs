use std::ffi::CString;

use crate::entities::prelude::IncomingMail;
use config::Config;
use config_file::FromConfigFile;
use entities::email_status::EmailStatus;
use indymilter::{Callbacks, Context, SocketInfo, Status};
use sea_orm::prelude::DateTimeWithTimeZone;
use tokio::{net::TcpListener, signal};
use tracing::{debug, info, warn};

pub mod config;
pub mod entities;
#[cfg(test)]
pub mod tests;

struct Session {
    pub incoming_mail_id: Option<i32>,
    pub sender_local_part: Option<String>,
    pub sender_domain: Option<String>,
    pub recipients: Option<String>,
    pub message_id: Option<String>,
    pub sending_host_name: Option<Option<String>>,
    pub sending_ip: Option<String>,
    pub time_received: Option<DateTimeWithTimeZone>,
    pub time_accepted: Option<Option<DateTimeWithTimeZone>>,
    pub status: EmailStatus,
}

impl Session {
    fn new() -> Session {
        Session {
            incoming_mail_id: None,
            sender_local_part: None,
            sender_domain: None,
            recipients: None,
            message_id: None,
            sending_host_name: None,
            sending_ip: None,
            time_received: None,
            time_accepted: None,
            status: EmailStatus::New,
        }
    }
}

#[tokio::main]
async fn main() {
    // Set up logging
    tracing_subscriber::fmt::init();

    let config = Config::from_config_file(format!("/etc/{}.toml", env!("CARGO_PKG_NAME"))).expect(
        format!(
            "Unable to read configuration from /etc/{}.toml",
            env!("CARGO_PKG_NAME")
        )
        .as_str(),
    );

    info!(
        "Starting {} version {} on {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        config.get_listen_address()
    );
    let listener = TcpListener::bind(config.get_listen_address())
        .await
        .expect("Unable to open milter socket");

    let callbacks = get_callbacks();

    indymilter::run(listener, callbacks, Default::default(), signal::ctrl_c())
        .await
        .expect("milter execution failed");
}

fn get_callbacks() -> Callbacks<Session> {
    Callbacks::new()
        .on_connect(|context, hostname, socket_info| {
            Box::pin(handle_connect(context, hostname, socket_info))
        })
        .on_mail(|context, args| Box::pin(handle_mail(context, args)))
}

async fn handle_connect(
    session: &mut Context<Session>,
    hostname: CString,
    socket_info: SocketInfo,
) -> Status {
    let mut session_data = Session::new();

    if let SocketInfo::Inet(addr) = socket_info {
        debug!("Connect from {}", addr.ip());
        session_data.sending_ip = Some(addr.ip().to_string());
        if !hostname.is_empty() {
            session_data.sending_host_name = match hostname.into_string() {
                Ok(string) => Some(Some(string)),
                Err(err) => {
                    warn!("Unable to read host name: {}", err);
                    Some(None)
                }
            }
        } else {
            session_data.sending_host_name = Some(None);
        }
    }

    session.data = Some(session_data);

    Status::Continue
}

async fn handle_mail(context: &mut Context<Session>, args: Vec<CString>) -> Status {
    debug!("Mail {:?}", args);
    Status::Continue
}
