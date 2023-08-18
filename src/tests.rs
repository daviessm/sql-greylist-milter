use byte_strings::c_str;
use indymilter::{Actions, MacroStage};
use indymilter_test::*;
use std::{net::Ipv4Addr, time::Duration};
use tokio::{net::TcpListener, sync::oneshot};

use crate::get_callbacks;

const LOCALHOST: (Ipv4Addr, u16) = (Ipv4Addr::LOCALHOST, 0);

#[tokio::test]
async fn basic() {
    tracing_subscriber::fmt::init();

    let mut conn = TestConnection::configure()
        .read_timeout(Duration::from_secs(10))
        .write_timeout(Duration::from_secs(10))
        .available_actions(Actions::ADD_RCPT)
        .open_tcp("[::1]:9876")
        .await
        .unwrap();

//    assert_eq!(conn.negotiated_actions(), Actions::ADD_RCPT);

    let status = conn
        .connect("client.example.org", [123, 123, 123, 123])
        .await
        .unwrap();
    assert_eq!(status, Status::Continue);

    conn.macros(MacroStage::Mail, [("{auth_authen}", "from@example.org")])
        .await
        .unwrap();

    let status = conn.mail(["<from@example.org>"]).await.unwrap();
    assert_eq!(status, Status::Continue);

    let (actions, status) = conn.eom().await.unwrap();

    assert_eq!(
        status,
        Status::Continue
    );

    conn.close().await.unwrap();
}