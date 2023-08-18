use std::{ffi::CString, sync::Arc, time::Duration};

use chrono::Utc;
use config::Config;
use config_file::FromConfigFile;
use entities::incoming_mail::ActiveModel as IncomingMailActive;
use indymilter::{Callbacks, Context, SocketInfo, Status};
use sea_orm::{ConnectOptions, Database, Set};
use tokio::{net::TcpListener, signal};
use tracing::{debug, info, warn};

pub mod config;
pub mod entities;
#[cfg(test)]
pub mod tests;

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

    let mut db_options = ConnectOptions::new(config.get_db_url());
    db_options
        .max_connections(100)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(2))
        .idle_timeout(Duration::from_secs(5));
    let orm_pool = Arc::new(
        Database::connect(db_options)
            .await
            .expect("Unable to connect to database"),
    );
    let callbacks = get_callbacks();

    indymilter::run(listener, callbacks, Default::default(), signal::ctrl_c())
        .await
        .expect("milter execution failed");
}

fn get_callbacks() -> Callbacks<IncomingMailActive> {
    Callbacks::new()
        .on_connect(|context, hostname, socket_info| {
            Box::pin(handle_connect(context, hostname, socket_info))
        })
        .on_mail(|context, args| Box::pin(handle_mail(context, args)))
}

async fn handle_connect(
    session: &mut Context<IncomingMailActive>,
    hostname: CString,
    socket_info: SocketInfo,
) -> Status {
    let mut session_data = IncomingMailActive {
        time_received: Set(Utc::now().into()),
        ..Default::default()
    };

    if let SocketInfo::Inet(addr) = socket_info {
        debug!("Connect from {}", addr.ip());
        session_data.sending_ip = Set(addr.ip().to_string());
        if !hostname.is_empty() {
            session_data.sending_host_name = match hostname.into_string() {
                Ok(string) => Set(Some(string)),
                Err(err) => {
                    warn!("Unable to read host name: {}", err);
                    Set(None)
                }
            }
        } else {
            session_data.sending_host_name = Set(None);
        }
    }

    session.data = Some(session_data);

    Status::Continue
}

async fn handle_mail(session: &mut Context<IncomingMailActive>, args: Vec<CString>) -> Status {
    debug!("Mail {:?}", args);
    let session_data = session.data.as_mut().expect("No session?");

    if args.len() >= 1 {
        if let Ok(sender) = args[0].clone().into_string() {
            if sender.len() > 2 {
                // Assume the first and last characters are < and >
                let sender = &sender[1..sender.len() - 1];
                let mut sender_parts = sender.split('@');
                if let Some(sender_local_part) = sender_parts.next() {
                    session_data.sender_local_part = Set(sender_local_part.to_string());
                } else {
                    warn!("No sender_local_part? (args from MAIL FROM: {:?})", args);
                    return Status::Reject;
                }
                if let Some(sender_domain) = sender_parts.next() {
                    session_data.sender_domain = Set(sender_domain.to_string());
                    Status::Continue
                } else {
                    warn!("No sender_domain? (args from MAIL FROM: {:?})", args);
                    return Status::Reject;
                }
            } else {
                warn!("Sender length is < 2? (args from MAIL FROM: {:?})", args);
                return Status::Reject;
            }
        } else {
            warn!("Sender is not a valid String? (args from MAIL FROM: {:?})", args);
            return Status::Reject;
        }
    } else {
        warn!("Null sender? (args from MAIL FROM: {:?})", args);
        return Status::Reject;
    }
}
