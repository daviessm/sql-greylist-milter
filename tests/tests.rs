mod common;

use indymilter::{Actions, MacroStage};
use indymilter_test::*;

#[tokio::test]
async fn basic() {
    let (mut conn, shutdown_sender) = common::setup().await;

    assert_eq!(conn.negotiated_actions(), Actions::ADD_RCPT | Actions::DELETE_RCPT);

    let status = conn
        .connect("client.example.org", [123, 123, 123, 123])
        .await
        .unwrap();
    assert_eq!(status, Status::Continue);

    conn.macros(MacroStage::Eoh, [("{auth_type}", "sasl")])
        .await
        .unwrap();

    let status = conn.mail(["<from@example.org>"]).await.unwrap();
    assert_eq!(status, Status::Continue);

    let status = conn.rcpt(["<test@example.org>"]).await.unwrap();
    assert_eq!(status, Status::Continue);

    // Second recipient for the same message
    let status = conn.rcpt(["<test2@example.org>"]).await.unwrap();
    assert_eq!(status, Status::Continue);

    //let status = conn.data().await.unwrap();
    //assert_eq!(status, Status::Continue);

    let status = conn
        .header("From", "Test Testerson <test@example.org>")
        .await
        .unwrap();
    assert_eq!(status, Status::Continue);

    let status = conn
        .header("To", "Test Testerson <test@example.org>")
        .await
        .unwrap();
    assert_eq!(status, Status::Continue);

    let status = conn
        .header("CC", "Test Testerson <test@example.org>")
        .await
        .unwrap();
    assert_eq!(status, Status::Continue);

    let status = conn
        .header(
            "Message-Id",
            "<12345678900987654321.1234567890@example.org>",
        )
        .await
        .unwrap();
    assert_eq!(status, Status::Continue);

    let status = conn.eoh().await.unwrap();
    assert_eq!(status, Status::Tempfail { message: None });

    let (_actions, status) = conn.eom().await.unwrap();
    assert_eq!(status, Status::Continue);

    conn.close().await.unwrap();

    common::shutdown(shutdown_sender);
}
