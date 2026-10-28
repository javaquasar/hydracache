#[cfg(unix)]
#[test]
fn fd_value() {
    use std::os::fd::AsFd;
    use zbus::{Basic, Fd, wire::LE};

    let stdout = std::io::stdout();
    let fd = stdout.as_fd();
    fd_value_test!(LE, Fd::from(fd), 4, 4, 8);
}
