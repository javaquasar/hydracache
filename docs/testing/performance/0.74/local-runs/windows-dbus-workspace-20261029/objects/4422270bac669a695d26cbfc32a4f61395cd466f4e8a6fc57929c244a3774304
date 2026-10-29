use super::*;
use std::io::{Read, Write};

fn pair() -> Pair {
    let mut pair = Pair::open().unwrap();
    pair.initialize().unwrap();
    pair
}
#[test]
fn original_pipe_kernel_identity_flags_and_distinct_streams() {
    let a = pair();
    let b = pair();
    a.inspect().unwrap();
    b.inspect().unwrap();
    assert!(a.identity != b.identity);
    assert!(status_valid(flags(&a.reader, libc::F_GETFL).unwrap(), true));
    assert!(status_valid(
        flags(&a.writer, libc::F_GETFL).unwrap(),
        false
    ));
}
#[test]
fn original_pipe_kernel_binary_bytes_and_eof_require_last_writer() {
    let Pair {
        mut reader,
        mut writer,
        ..
    } = pair();
    let mut duplicate = writer.try_clone().unwrap();
    let mut bytes = [0; 32];
    assert_eq!(
        reader.read(&mut bytes).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    writer.write_all(b"\0binary\xff\n").unwrap();
    let count = reader.read(&mut bytes).unwrap();
    assert_eq!(&bytes[..count], b"\0binary\xff\n");
    let mut read = [0; 32];
    drop(writer);
    duplicate.write_all(b"late").unwrap();
    assert_eq!(reader.read(&mut read).unwrap(), 4);
    assert_eq!(&read[..4], b"late");
    assert_eq!(
        reader.read(&mut read).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    drop(duplicate);
    assert_eq!(reader.read(&mut read).unwrap(), 0);
}
#[test]
fn original_pipe_kernel_status_seeded_exact_flag_language() {
    let mut seed = 0x0740_7011_u32;
    for _ in 0..4096 {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let status = seed as i32;
        assert_eq!(status_valid(status, true), status == libc::O_NONBLOCK);
        assert_eq!(status_valid(status, false), status == libc::O_WRONLY);
    }
    for status in [
        libc::O_RDONLY,
        libc::O_NONBLOCK,
        libc::O_WRONLY,
        libc::O_WRONLY | libc::O_NONBLOCK,
        libc::O_RDWR,
        -1,
    ] {
        assert_eq!(status_valid(status, true), status == libc::O_NONBLOCK);
        assert_eq!(status_valid(status, false), status == libc::O_WRONLY);
    }
    println!("HC074-PIPE-FLAGS seed=0x07407011 randomized=4096 edges=6 exact=true");
}
#[test]
fn original_pipe_kernel_full_queue_preserves_blocking_backpressure() {
    let mut p = pair();
    set_flags(&p.writer, libc::F_SETFL, libc::O_NONBLOCK).unwrap();
    let mut total = 0;
    let block = [0x75; 4096];
    loop {
        match p.writer.write(&block) {
            Ok(n) => {
                assert!(n > 0);
                total += n;
                assert!(total <= 16 * 1024 * 1024);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) => panic!("{e}"),
        }
    }
    set_flags(&p.writer, libc::F_SETFL, 0).unwrap();
    p.inspect().unwrap();
    let mut poll = libc::pollfd {
        fd: p.writer.as_raw_fd(),
        events: libc::POLLOUT,
        revents: 0,
    };
    assert_eq!(unsafe { libc::poll(&mut poll, 1, 0) }, 0);
    assert_eq!(poll.revents, 0);
    assert!(total >= block.len());
    let mut bytes = [0; 4096];
    assert_eq!(p.reader.read(&mut bytes).unwrap(), block.len());
    assert_eq!(bytes, block);
    assert_eq!(unsafe { libc::poll(&mut poll, 1, 0) }, 1);
    assert_eq!(poll.revents, libc::POLLOUT);
    p.inspect().unwrap();
}
