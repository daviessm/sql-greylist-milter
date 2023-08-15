use crate::entities::prelude::IncomingMail;
use config::Config;
use config_file::FromConfigFile;
use indymilter::{Callbacks, Context, SocketInfo, Status};
use tokio::{net::UnixListener, signal};

pub mod config;
pub mod entities;

#[tokio::main]
async fn main() {
    let config = Config::from_config_file("/etc/sql-greylist-milter.toml")
        .expect("Unable to read configuration from /etc/sql-greylist-milter.toml");
    let listener =
        UnixListener::bind(config.get_listen_address()).expect("Unable to open milter socket");

    let callbacks = Callbacks::new()
        .on_connect(|context, _, socket_info| Box::pin(handle_connect(context, socket_info)));

    indymilter::run(listener, callbacks, Default::default(), signal::ctrl_c())
        .await
        .expect("milter execution failed");
}

async fn handle_connect(_: &mut Context<()>, socket_info: SocketInfo) -> Status {
    if let SocketInfo::Inet(addr) = socket_info {
        println!("connect from {}", addr.ip());
    }

    Status::Continue
}
