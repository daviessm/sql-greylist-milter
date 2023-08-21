use std::io;

use sql_greylist_milter::real_main;
use tokio::signal::unix::SignalKind;

#[tokio::main]
async fn main() {
    // Set up logging
    tracing_subscriber::fmt::init();

    real_main(
        format!("/etc/{}.toml", env!("CARGO_PKG_NAME")),
        await_sigint(),
    )
    .await
}

async fn await_sigint() -> io::Result<()> {
    tokio::signal::unix::signal(SignalKind::terminate())?
        .recv()
        .await;
    Ok(())
}
