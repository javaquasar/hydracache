use ntest::timeout;
use zbus::{Error, connection};

#[cfg(not(all(windows, feature = "tokio")))]
const UNIX_ADDRESS: &str = "unix:path=/this/path/does/not/exist";
const TCP_ADDRESS: &str = "tcp:host=localhost,port=4142,family=ipv4";
#[cfg(all(unix, feature = "unixexec"))]
const UNIXEXEC_ADDRESS: &str = "unixexec:path=/this/path/does/not/exist";

#[test]
#[timeout(15000)]
fn connection_error() {
    // Addresses issue [#1478](https://github.com/z-galaxy/zbus/issues/1478). The issue mentions
    // that connection error troubleshooting could be simplified by surfacing the connection
    // address to the user. This test ensures connection failures throw the error Error::Connection,
    // and that such error shows the address involved in the attempted connection.
    zbus::block_on(connection_error_async());
}

async fn connection_error_async() {
    #[allow(unused_mut)]
    let mut addresses = vec![TCP_ADDRESS];
    // A Tokio connection on Windows has nothing to reach a unix socket with, so it turns the
    // address down before any connection is attempted.
    #[cfg(not(all(windows, feature = "tokio")))]
    addresses.push(UNIX_ADDRESS);
    #[cfg(all(unix, feature = "unixexec"))]
    addresses.push(UNIXEXEC_ADDRESS);

    for addr in addresses {
        let res = connection::Builder::address(addr).build().await;

        let Err(error) = res else {
            panic!("expected a connection error, got {res:?}");
        };
        let Error::Connection(_, error_addr) = &error else {
            panic!("expected a connection error, got {error:?}");
        };

        assert_eq!(error_addr.to_string(), addr);
        // What follows the address is the I/O error, which differs between platforms.
        assert!(
            error
                .to_string()
                .starts_with(&format!("failed to connect to `{addr}`: ")),
            "unexpected message: {error}",
        );
    }
}
