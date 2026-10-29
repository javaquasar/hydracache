//! In-process Channel only. All names/credentials are synthetic assertions.
use super::*;
use std::{future::poll_fn, pin::Pin, time::Duration};
use zbus::{export::futures_core::Stream, message::Message};

const METHOD: Duration = Duration::from_millis(250);

fn pair() -> (Connection, zbus::Connection) {
    zbus::block_on(async {
        let (a, b) = zbus::connection::socket::Channel::pair();
        let guid = zbus::Guid::try_from("07400000000000000000000000000000").unwrap();
        let client = zbus::connection::Builder::authenticated_socket(a, guid.clone())
            .p2p()
            .method_timeout(METHOD)
            .build()
            .await
            .unwrap();
        let server = zbus::connection::Builder::authenticated_socket(b, guid)
            .p2p()
            .method_timeout(METHOD)
            .build()
            .await
            .unwrap();
        (Connection::from_async(client), server)
    })
}

enum Answer {
    Number,
    WrongType,
    WrongSender,
    NoSuchUnit,
    ForeignError,
}

fn answer(server: &zbus::Connection, answers: Vec<Answer>) -> tokio::task::JoinHandle<()> {
    let connection = server.clone();
    zbus::block_on(async move {
        let mut stream = zbus::MessageStream::from(&connection);
        tokio::spawn(async move {
            for answer in answers {
                let request =
                    tokio::time::timeout(METHOD, poll_fn(|cx| Pin::new(&mut stream).poll_next(cx)))
                        .await
                        .unwrap()
                        .unwrap()
                        .unwrap();
                assert_eq!(request.header().member().unwrap().as_str(), "GetUnit");
                assert_eq!(request.header().destination().unwrap().as_str(), ":1.1");
                assert_eq!(
                    request.body().deserialize::<String>().unwrap(),
                    "fixture.service"
                );
                let header = request.header();
                let response = match answer {
                    Answer::Number => Message::method_return(&header).sender(":1.1").build(&74u32),
                    Answer::WrongType => Message::method_return(&header)
                        .sender(":1.1")
                        .build(&"wrong-type"),
                    Answer::WrongSender => {
                        Message::method_return(&header).sender(":1.2").build(&74u32)
                    }
                    Answer::NoSuchUnit => {
                        Message::error(&header, "org.freedesktop.systemd1.NoSuchUnit")
                            .sender(":1.1")
                            .build(&"missing")
                    }
                    Answer::ForeignError => {
                        Message::error(&header, "org.freedesktop.systemd1.NoSuchUnit")
                            .sender(":1.2")
                            .build(&"missing")
                    }
                }
                .unwrap();
                connection.send(&response).await.unwrap();
            }
            zbus::AsyncDrop::async_drop(stream).await;
        })
    })
}

fn finish(client: Connection, server: zbus::Connection, task: tokio::task::JoinHandle<()>) {
    zbus::block_on(async {
        tokio::time::timeout(METHOD, task).await.unwrap().unwrap();
    });
    client.close().unwrap();
    zbus::block_on(server.close()).unwrap();
}

#[test]
fn setters_fail_immediately_in_original_order() {
    let (client, server) = pair();
    assert!(
        matches!(proxy(&client, "bad name", "/valid", "org.example.Valid"), Err(reason) if reason == "proxy destination")
    );
    assert!(
        matches!(proxy(&client, ":1.1", "bad/path", "org.example.Valid"), Err(reason) if reason == "proxy path")
    );
    assert!(
        matches!(proxy(&client, ":1.1", "/valid", "bad interface"), Err(reason) if reason == "proxy interface")
    );
    client.close().unwrap();
    zbus::block_on(server.close()).unwrap();
}

#[test]
fn checked_call_preserves_success_signature_and_sender() {
    let (client, server) = pair();
    let task = answer(
        &server,
        vec![Answer::Number, Answer::WrongType, Answer::WrongSender],
    );
    let proxy = proxy(
        &client,
        ":1.1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
    )
    .unwrap();
    let first: Result<u32, ReplyError> =
        checked_call(&proxy, ":1.1", "GetUnit", &("fixture.service",));
    assert_eq!(first, Ok(74));
    let wrong: Result<u32, ReplyError> =
        checked_call(&proxy, ":1.1", "GetUnit", &("fixture.service",));
    assert_eq!(wrong, Err(ReplyError::Refused));
    let foreign: Result<u32, ReplyError> =
        checked_call(&proxy, ":1.1", "GetUnit", &("fixture.service",));
    assert_eq!(foreign, Err(ReplyError::Refused));
    drop(proxy);
    finish(client, server, task);
}

#[test]
fn method_error_keeps_original_sender() {
    let (client, server) = pair();
    let task = answer(&server, vec![Answer::NoSuchUnit, Answer::ForeignError]);
    let proxy = proxy(
        &client,
        ":1.1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
    )
    .unwrap();
    let original: Result<u32, ReplyError> =
        checked_call(&proxy, ":1.1", "GetUnit", &("fixture.service",));
    assert_eq!(original, Err(ReplyError::NoSuchUnit));
    let foreign: Result<u32, ReplyError> =
        checked_call(&proxy, ":1.1", "GetUnit", &("fixture.service",));
    assert_eq!(foreign, Err(ReplyError::Refused));
    drop(proxy);
    finish(client, server, task);
}

#[test]
fn proxy_clone_keeps_connection_after_original_drop() {
    let (client, server) = pair();
    let task = answer(&server, vec![Answer::Number, Answer::Number]);
    let proxy = proxy(
        &client,
        ":1.1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
    )
    .unwrap();
    let alias = proxy.clone();
    drop(client);
    assert_eq!(
        proxy
            .call::<_, _, u32>("GetUnit", &("fixture.service",))
            .unwrap(),
        74
    );
    drop(proxy);
    assert_eq!(
        alias
            .call::<_, _, u32>("GetUnit", &("fixture.service",))
            .unwrap(),
        74
    );
    let held = alias.connection().clone();
    drop(alias);
    finish(held, server, task);
}

#[test]
fn no_reply_preserves_method_timeout_refusal() {
    let (client, server) = pair();
    let connection = server.clone();
    let (release, wait) = tokio::sync::oneshot::channel();
    let (seen, observed) = std::sync::mpsc::sync_channel(1);
    let task = zbus::block_on(async move {
        let mut stream = zbus::MessageStream::from(&connection);
        tokio::spawn(async move {
            let request =
                tokio::time::timeout(METHOD, poll_fn(|cx| Pin::new(&mut stream).poll_next(cx)))
                    .await
                    .unwrap()
                    .unwrap()
                    .unwrap();
            assert_eq!(request.header().member().unwrap().as_str(), "GetUnit");
            seen.send(()).unwrap();
            wait.await.unwrap();
            zbus::AsyncDrop::async_drop(stream).await;
        })
    });
    let proxy = proxy(
        &client,
        ":1.1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
    )
    .unwrap();
    let result: Result<u32, ReplyError> =
        checked_call(&proxy, ":1.1", "GetUnit", &("fixture.service",));
    assert_eq!(result, Err(ReplyError::Refused));
    observed.try_recv().unwrap();
    release.send(()).unwrap();
    drop(proxy);
    finish(client, server, task);
}
