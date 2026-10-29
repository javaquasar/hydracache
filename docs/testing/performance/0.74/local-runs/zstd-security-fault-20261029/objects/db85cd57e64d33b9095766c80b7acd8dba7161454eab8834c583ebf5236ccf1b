use super::*;
use std::cell::Cell;

const BOOT: &[u8; 37] = b"e81671d6-bf23-4ebf-b8fe-5dad1f45866f\n";
fn boot_read(
    read: impl FnOnce(&mut [u8]) -> std::io::Result<usize>,
) -> Result<String, KernelError> {
    super::boot_value(super::boot_read(read)?)
}
fn reference(
    read: impl FnOnce(&mut [u8]) -> std::io::Result<usize>,
) -> Result<String, KernelError> {
    let mut raw = vec![0; DOC_BYTES + 1];
    let count = read(&mut raw)?;
    if count == 0 || count > DOC_BYTES {
        return Err(KernelError::Invalid);
    }
    raw.truncate(count);
    if raw.len() != 37
        || raw[36] != b'\n'
        || raw[..36].iter().enumerate().any(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                *c != b'-'
            } else {
                !c.is_ascii_digit() && !(b'a'..=b'f').contains(c)
            }
        })
    {
        return Err(KernelError::Invalid);
    }
    String::from_utf8(raw[..36].to_vec()).map_err(|_| KernelError::Invalid)
}
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Value(String),
    Invalid,
    Io(std::io::ErrorKind, Option<i32>),
}
fn outcome(value: Result<String, KernelError>) -> Outcome {
    match value {
        Ok(value) => Outcome::Value(value),
        Err(KernelError::Invalid) => Outcome::Invalid,
        Err(KernelError::Io(error)) => Outcome::Io(error.kind(), error.raw_os_error()),
        Err(error) => panic!("unexpected boot error: {error:?}"),
    }
}
fn pair(document: &[u8], limit: usize) -> Outcome {
    let calls = Cell::new(0);
    let read = |output: &mut [u8]| {
        calls.set(calls.get() + 1);
        let count = document.len().min(output.len()).min(limit);
        output[..count].copy_from_slice(&document[..count]);
        Ok(count)
    };
    let before = outcome(reference(read));
    assert_eq!(calls.get(), 1);
    let after = outcome(boot_read(read));
    assert_eq!(calls.get(), 2);
    assert_eq!(before, after, "length={} limit={limit}", document.len());
    after
}
#[test]
fn boot_buffer_canonical_empty_short_and_overlength_differential() {
    assert_eq!(
        pair(BOOT, usize::MAX),
        Outcome::Value(String::from_utf8(BOOT[..36].to_vec()).unwrap())
    );
    for length in 0..37 {
        assert_eq!(pair(&BOOT[..length], usize::MAX), Outcome::Invalid);
    }
    for length in [38, 39, 256, 65_536, 65_537, 131_074] {
        let mut document = vec![b'x'; length];
        document[..37].copy_from_slice(BOOT);
        assert_eq!(pair(&document, usize::MAX), Outcome::Invalid);
    }
}
#[test]
fn boot_buffer_every_single_byte_mutation_matches_old_decoder() {
    for index in 0..37 {
        for byte in 0..=255 {
            let mut document = *BOOT;
            document[index] = byte;
            pair(&document, usize::MAX);
        }
    }
}
#[test]
fn boot_buffer_seeded_documents_match_old_decoder() {
    let mut seed = 0x0740_b038_u64;
    for case in 0..1024 {
        let mut document = vec![0; case % 129];
        for byte in &mut document {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            *byte = seed as u8;
        }
        pair(&document, usize::MAX);
    }
    println!(
        "HC074-BOOT-DIFFERENTIAL seed=0x0740b038 generated=1024 mutations=9472 single_read=true"
    );
}
#[test]
fn boot_buffer_short_read_is_not_retried_and_does_not_claim_eof() {
    let mut document = BOOT.to_vec();
    document.push(b'x');
    for limit in 0..=38 {
        let result = pair(&document, limit);
        assert_eq!(matches!(result, Outcome::Value(_)), limit == 37);
    }
}
#[test]
fn boot_buffer_io_faults_are_preserved_without_retry() {
    for errno in [
        libc::EINTR,
        libc::EAGAIN,
        libc::EIO,
        libc::EACCES,
        libc::EBADF,
    ] {
        let calls = Cell::new(0);
        let read = |_: &mut [u8]| {
            calls.set(calls.get() + 1);
            Err(std::io::Error::from_raw_os_error(errno))
        };
        assert_eq!(outcome(reference(read)), outcome(boot_read(read)));
        assert_eq!(calls.get(), 2);
    }
}
#[test]
fn boot_buffer_empty_site_and_document_trace_are_preserved() {
    let (result, event) =
        crate::diagnostic_kernel_admission::refusal_trace::capture(|| boot_read(|_| Ok(0)));
    assert!(matches!(result, Err(KernelError::Invalid)));
    assert_eq!(event.unwrap().site, "proc-document-budget");
    use crate::diagnostic_ipc::validation_trace::kernel_walk::{capture, Kind};
    let (result, report) = capture(|| {
        boot_read(|bytes| {
            assert_eq!(bytes.len(), 38);
            bytes[..37].copy_from_slice(BOOT);
            Ok(37)
        })
    });
    result.unwrap();
    assert_eq!(report.nodes[Kind::Document as usize].calls, 1);
    assert_eq!(report.nodes.iter().map(|s| s.calls).sum::<u64>(), 1);
}
#[test]
fn boot_buffer_real_positional_reads_match_without_changing_cursor() {
    let mut file = File::open("/proc/sys/kernel/random/boot_id").unwrap();
    let read = |bytes: &mut [u8]| file.read_at(bytes, 0);
    assert_eq!(outcome(reference(read)), outcome(boot_read(read)));
    assert_eq!(boot_read(read).unwrap().len(), 36);
    assert_eq!(std::io::Seek::stream_position(&mut file).unwrap(), 0);
}
