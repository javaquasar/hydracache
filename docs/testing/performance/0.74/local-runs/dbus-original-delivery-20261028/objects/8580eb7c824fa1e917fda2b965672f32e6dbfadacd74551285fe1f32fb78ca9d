#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "support/root_process_guard.rs"]
mod root_process_guard;

use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::process::{Command, Stdio};
use std::sync::Mutex;

static SPAWN: Mutex<()> = Mutex::new(());

fn cat() -> (Command, UnixStream) {
    let (input, mut writer) = UnixStream::pair().unwrap();
    writer.write_all(b"public root guard fixture\n").unwrap();
    let input: OwnedFd = input.into();
    let mut command = Command::new("/bin/cat");
    command
        .env_clear()
        .stdin(Stdio::from(input))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    (command, writer)
}

#[test]
fn successful_exit_keeps_exact_streams_and_status() {
    let (mut command, writer) = cat();
    drop(writer);
    let result = root_process_guard::output(&mut command, &SPAWN);
    assert!(result.status.success());
    assert_eq!(result.stdout, b"public root guard fixture\n");
    assert!(result.stderr.is_empty());
}

#[test]
fn nonzero_exit_is_returned_not_promoted_or_retried() {
    let mut command = Command::new("/bin/cat");
    command
        .arg("--hc074-guard-invalid-option")
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let result = root_process_guard::output(&mut command, &SPAWN);
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    assert!(!result.stderr.is_empty());
}

#[test]
fn timeout_reports_original_kill_reap_and_public_output() {
    let (mut command, writer) = cat();
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        root_process_guard::output(&mut command, &SPAWN)
    }))
    .expect_err("open input must reach the unchanged timeout");
    drop(writer);
    let message = failure
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| failure.downcast_ref::<&str>().copied())
        .unwrap();
    assert!(message.contains("normal root preparation stalled"));
    assert!(message.contains("signal=Some(9)"), "{message}");
    assert!(message.contains("original_wait_ok=true"), "{message}");
    assert!(
        message.contains("public root guard fixture\\n"),
        "{message}"
    );
}

#[test]
fn a_live_writer_is_would_block_not_eof_or_an_unbounded_read() {
    let (reader, mut writer) = UnixStream::pair().unwrap();
    let writer_flags = unsafe { libc::fcntl(writer.as_raw_fd(), libc::F_GETFL) };
    let empty = root_process_guard::capture(Some(reader.try_clone().unwrap()));
    assert!(empty.bytes.is_empty());
    assert_eq!(empty.end, root_process_guard::End::WouldBlock);
    writer.write_all(b"public prefix").unwrap();
    let prefix = root_process_guard::capture(Some(reader));
    assert_eq!(prefix.bytes, b"public prefix");
    assert_eq!(prefix.end, root_process_guard::End::WouldBlock);
    assert_eq!(
        unsafe { libc::fcntl(writer.as_raw_fd(), libc::F_GETFL) },
        writer_flags
    );
}

#[test]
fn eof_and_byte_cap_are_distinct() {
    let (reader, mut writer) = UnixStream::pair().unwrap();
    writer.write_all(b"public EOF").unwrap();
    drop(writer);
    let result = root_process_guard::capture(Some(reader));
    assert_eq!(result.bytes, b"public EOF");
    assert_eq!(result.end, root_process_guard::End::Eof);

    let payload = vec![b'x'; root_process_guard::PREFIX_BYTES + 1];
    let result = root_process_guard::read_bounded(&mut io::Cursor::new(payload));
    assert_eq!(result.bytes.len(), root_process_guard::PREFIX_BYTES);
    assert_eq!(result.end, root_process_guard::End::ByteLimit);
}

struct Interrupted(usize);
impl Read for Interrupted {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        self.0 += 1;
        Err(io::ErrorKind::Interrupted.into())
    }
}

#[test]
fn repeated_eintr_consumes_the_fixed_read_budget() {
    let mut reader = Interrupted(0);
    let result = root_process_guard::read_bounded(&mut reader);
    assert!(result.bytes.is_empty());
    assert_eq!(result.end, root_process_guard::End::ReadBudget);
    assert_eq!(reader.0, root_process_guard::READ_ATTEMPTS);
}

struct Broken(usize);
impl Read for Broken {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.0 += 1;
        if self.0 == 1 {
            buffer[..4].copy_from_slice(b"diag");
            Ok(4)
        } else {
            Err(io::Error::from_raw_os_error(libc::EIO))
        }
    }
}

#[test]
fn an_io_error_preserves_only_the_preceding_prefix() {
    let mut reader = Broken(0);
    let result = root_process_guard::read_bounded(&mut reader);
    assert_eq!(result.bytes, b"diag");
    assert_eq!(
        result.end,
        root_process_guard::End::ReadError(Some(libc::EIO))
    );
    assert_eq!(reader.0, 2);
}

struct Invalid;
impl AsRawFd for Invalid {
    fn as_raw_fd(&self) -> i32 {
        -1
    }
}
impl Read for Invalid {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        panic!("setup refusal must not read");
    }
}

#[test]
fn missing_and_invalid_handles_refuse_capture_without_reading() {
    let missing = root_process_guard::capture::<UnixStream>(None);
    assert!(missing.bytes.is_empty());
    assert_eq!(missing.end, root_process_guard::End::Missing);
    let invalid = root_process_guard::capture(Some(Invalid));
    assert!(invalid.bytes.is_empty());
    assert_eq!(
        invalid.end,
        root_process_guard::End::SetupError(Some(libc::EBADF))
    );
}

#[test]
fn seeded_binary_prefixes_never_overread_or_claim_unobserved_eof() {
    let mut seed = 0x0740_c091_u64;
    for len in [0, 1, 511, 512, 513, 3584, 3585, 4095, 4096, 4097, 8192] {
        let payload = (0..len)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                seed as u8
            })
            .collect::<Vec<_>>();
        let mut reader = io::Cursor::new(&payload);
        let result = root_process_guard::read_bounded(&mut reader);
        assert!(result.bytes.len() <= root_process_guard::PREFIX_BYTES);
        assert_eq!(result.bytes, payload[..result.bytes.len()]);
        assert_eq!(reader.position(), result.bytes.len() as u64);
        match result.end {
            root_process_guard::End::Eof => assert_eq!(result.bytes.len(), len),
            root_process_guard::End::ByteLimit => {
                assert_eq!(result.bytes.len(), root_process_guard::PREFIX_BYTES);
            }
            root_process_guard::End::ReadBudget => {
                assert!(result.bytes.len() < root_process_guard::PREFIX_BYTES);
            }
            _ => panic!("cursor cannot report a pipe or IO state"),
        }
    }
    println!("root_guard_prefix seed=0x0740c091 lengths=11 admission=false");
}
