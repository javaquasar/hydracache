//! Old blocking Proxy::new panics on nested runtime entry before bad-name parsing.
#[test]
fn nested_constructor_panics_before_bad_arguments() {
    let (left, right) = zbus::connection::socket::Channel::pair();
    let guid = zbus::Guid::try_from("07400000000000000000000000000000").unwrap();
    let (client, server) = zbus::block_on(async {
        let client = zbus::connection::Builder::authenticated_socket(left, guid.clone())
            .p2p()
            .build()
            .await
            .unwrap();
        let server = zbus::connection::Builder::authenticated_socket(right, guid)
            .p2p()
            .build()
            .await
            .unwrap();
        (crate::bridge::Connection::from_async(client), server)
    });
    let nested = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        zbus::block_on(async {
            assert!(
                crate::bridge::Proxy::new(&client, "bad name", "bad/path", "bad interface")
                    .is_err()
            );
        });
    }));
    // Close both synthetic endpoints before asserting the constructor contract.
    client.close().unwrap();
    zbus::block_on(server.close()).unwrap();
    assert!(
        nested.is_err(),
        "nested entry was bypassed by early argument refusal"
    );
}
