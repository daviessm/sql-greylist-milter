mod common;

use indymilter::{Actions, MacroStage};
use indymilter_test::*;

#[tokio::test]
async fn greylist() {
    let (mut conn, shutdown_sender) = common::setup().await;

    assert_eq!(
        conn.negotiated_actions(),
        Actions::ADD_RCPT | Actions::DELETE_RCPT
    );

    let status = conn
        .connect("client.test.example", [123, 123, 123, 123])
        .await
        .unwrap();
    assert_eq!(status, Status::Continue);

    let status = conn.mail(["<from@test.example>"]).await.unwrap();
    assert_eq!(status, Status::Continue);

    let status = conn.rcpt(["<to@test.example>"]).await.unwrap();
    assert_eq!(status, Status::Continue);

    let status = conn
        .header("Message-Id", "<test_greylist@example.org>")
        .await
        .unwrap();
    assert_eq!(status, Status::Continue);

    let status = conn.eoh().await.unwrap();
    assert_eq!(status, Status::Tempfail { message: None });

    conn.close().await.unwrap();

    common::shutdown(shutdown_sender);
}

#[tokio::test]
async fn ip_accept() {
    let (mut conn, shutdown_sender) = common::setup().await;

    assert_eq!(
        conn.negotiated_actions(),
        Actions::ADD_RCPT | Actions::DELETE_RCPT
    );

    let status = conn
        .connect("client.test.example", [10, 255, 2, 123])
        .await
        .unwrap();
    assert_eq!(status, Status::Continue);

    let status = conn.mail(["<from@test.example>"]).await.unwrap();
    assert_eq!(status, Status::Continue);

    let status = conn.rcpt(["<to@test.example>"]).await.unwrap();
    assert_eq!(status, Status::Continue);

    let status = conn
        .header("Message-Id", "<test_ip_accept@example.org>")
        .await
        .unwrap();
    assert_eq!(status, Status::Continue);

    let status = conn.eoh().await.unwrap();
    assert_eq!(status, Status::Continue);

    let (_actions, status) = conn.eom().await.unwrap();
    assert_eq!(status, Status::Continue);

    conn.close().await.unwrap();

    common::shutdown(shutdown_sender);
}

#[tokio::test]
async fn auth_accept() {
    let (mut conn, shutdown_sender) = common::setup().await;

    assert_eq!(
        conn.negotiated_actions(),
        Actions::ADD_RCPT | Actions::DELETE_RCPT
    );

    let status = conn
        .connect("client.test.example", [123, 123, 123, 123])
        .await
        .unwrap();
    assert_eq!(status, Status::Continue);

    let status = conn.mail(["<from@test.example>"]).await.unwrap();
    assert_eq!(status, Status::Continue);

    let status = conn.rcpt(["<to@test.example>"]).await.unwrap();
    assert_eq!(status, Status::Continue);

    let status = conn
        .header("Message-Id", "<test_auth_accept@example.org>")
        .await
        .unwrap();
    assert_eq!(status, Status::Continue);

    conn.macros(MacroStage::Eoh, [("{auth_type}", "sasl")])
        .await
        .unwrap();

    let status = conn.eoh().await.unwrap();
    assert_eq!(status, Status::Continue);

    let (_actions, status) = conn.eom().await.unwrap();
    assert_eq!(status, Status::Continue);

    conn.close().await.unwrap();

    common::shutdown(shutdown_sender);
}
