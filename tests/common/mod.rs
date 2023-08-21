use std::time::Duration;

use indymilter::Actions;
use indymilter_test::TestConnection;
use sql_greylist_milter;
use tokio::{
    sync::oneshot::{self, Receiver},
    time::sleep,
};
use tracing::{warn, Level};

async fn shutdown_handler(rx: Receiver<()>) -> std::io::Result<()> {
    rx.await.unwrap();
    Ok(())
}

pub async fn setup() -> (TestConnection, oneshot::Sender<()>) {
    // Set up logging
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .init();

    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        sql_greylist_milter::real_main(
            "tests/common/config.toml".to_string(),
            shutdown_handler(rx),
        )
        .await;
    });

    let mut maybe_conn = TestConnection::configure()
        .read_timeout(Duration::from_secs(10))
        .write_timeout(Duration::from_secs(10))
        .available_actions(Actions::ADD_RCPT | Actions::DELETE_RCPT)
        .open_tcp("[::1]:9876")
        .await;

    loop {
        if let Err(e) = maybe_conn {
            warn!("Waiting for milter to start up: {}", e);
            sleep(Duration::from_millis(200)).await;

            maybe_conn = TestConnection::configure()
                .read_timeout(Duration::from_secs(10))
                .write_timeout(Duration::from_secs(10))
                .available_actions(Actions::ADD_RCPT | Actions::DELETE_RCPT)
                .open_tcp("[::1]:9876")
                .await;
        } else {
            break;
        }
    }

    (maybe_conn.unwrap(), tx)
}

pub fn shutdown(tx: oneshot::Sender<()>) {
    tx.send(()).unwrap();
}
