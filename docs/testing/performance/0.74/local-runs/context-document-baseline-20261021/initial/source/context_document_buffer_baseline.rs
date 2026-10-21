//! Pure baseline and frozen-body model. No File, proc read, clock or live owner.
use super::*;
use crate::diagnostic_worker_policy::local_context::WorkerContextError;
use std::cell::RefCell;

const LENGTHS: [usize; 9] = [0, 1, 4, 32, 33, 36, 37, 38, 63];
const ITERATIONS: u64 = 256;

fn frozen_read_with(
    expected: &str,
    length: usize,
    read: impl FnOnce(&mut [u8]) -> std::io::Result<usize>,
    mut metadata: impl FnMut() -> Result<bool, WorkerContextError>,
) -> Result<(), WorkerContextError> {
    if !metadata()? {
        return Err(WorkerContextError::Drift);
    }
    // One bounded positional read. Short/EINTR reads refuse; no retry or IO deadline claim.
    let mut buffer = vec![0; length + 1];
    let count = read(&mut buffer)?;
    if !metadata()? {
        return Err(WorkerContextError::Drift);
    }
    if count != length
        || expected.len() + 1 != length
        || buffer[count - 1] != b'\n'
        || &buffer[..count - 1] != expected.as_bytes()
    {
        return Err(WorkerContextError::Drift);
    }
    Ok(())
}

fn class(value: Result<(), WorkerContextError>) -> String {
    match value {
        Ok(()) => "ok".into(),
        Err(WorkerContextError::Io(e)) => format!("io:{:?}", e.raw_os_error()),
        Err(WorkerContextError::Drift) => "drift".into(),
        Err(e) => panic!("unexpected frozen-body error: {e}"),
    }
}

#[test]
fn context_document_buffer_baseline_reference_requests() {
    for length in LENGTHS {
        let (_, counts) = capture(|| {
            for _ in 0..ITERATIONS {
                let buffer = vec![0_u8; std::hint::black_box(length) + 1];
                let bytes = std::hint::black_box(buffer.as_slice());
                assert_eq!(bytes.len(), length + 1);
                assert!(bytes.iter().all(|b| *b == 0));
            }
        });
        assert_eq!(counts.allocations, ITERATIONS);
        assert_eq!(counts.reallocations, 0);
        assert_eq!(counts.requested_bytes, (length as u64 + 1) * ITERATIONS);
        println!(
            "HC074-CONTEXT-DOCUMENT-BASELINE {}",
            serde_json::json!({
                "length":length,"iterations":ITERATIONS,"counts":counts,
                "production_length":[33,37].contains(&length),"rust_requests_only":true,
                "timing_claim":false,"profile":false,"server":false,"product":false
            })
        );
    }
}

#[test]
fn context_document_buffer_baseline_exact_zeroed_slice_and_seeded_success() {
    let mut seed = 0x074c_ba51_u64;
    for length in [1, 4, 33, 37, 38, 63] {
        for _ in 0..64 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let expected = char::from(b'a' + (seed % 26) as u8)
                .to_string()
                .repeat(length - 1);
            let events = RefCell::new(Vec::new());
            frozen_read_with(
                &expected,
                length,
                |bytes| {
                    events.borrow_mut().push("read");
                    assert_eq!(bytes.len(), length + 1);
                    assert!(bytes.iter().all(|b| *b == 0));
                    bytes[..length - 1].copy_from_slice(expected.as_bytes());
                    bytes[length - 1] = b'\n';
                    Ok(length)
                },
                || {
                    events.borrow_mut().push("metadata");
                    Ok(true)
                },
            )
            .unwrap();
            assert_eq!(*events.borrow(), ["metadata", "read", "metadata"]);
        }
    }
}

#[test]
fn context_document_buffer_baseline_read_errors_do_not_retry_or_reach_closing_metadata() {
    for code in [libc::EINTR, libc::EIO, libc::EBADF] {
        let events = RefCell::new(Vec::new());
        let result = frozen_read_with(
            "abc",
            4,
            |_| {
                events.borrow_mut().push("read");
                Err(std::io::Error::from_raw_os_error(code))
            },
            || {
                events.borrow_mut().push("metadata");
                Ok(true)
            },
        );
        assert_eq!(class(result), format!("io:{:?}", Some(code)));
        assert_eq!(*events.borrow(), ["metadata", "read"]);
    }
}

#[test]
fn context_document_buffer_baseline_metadata_error_order_and_no_read_after_first_refusal() {
    for first in [true, false] {
        for io in [true, false] {
            let events = RefCell::new(Vec::new());
            let mut calls = 0;
            let result = frozen_read_with(
                "abc",
                4,
                |_| {
                    events.borrow_mut().push("read");
                    Ok(0) // Later content refusal must not outrank closing metadata IO.
                },
                || {
                    calls += 1;
                    events.borrow_mut().push("metadata");
                    if (calls == 1) == first {
                        if io {
                            Err(std::io::Error::from_raw_os_error(libc::EACCES).into())
                        } else {
                            Ok(false)
                        }
                    } else {
                        Ok(true)
                    }
                },
            );
            assert_eq!(
                class(result),
                if io {
                    format!("io:{:?}", Some(libc::EACCES))
                } else {
                    "drift".into()
                }
            );
            assert_eq!(
                *events.borrow(),
                if first {
                    vec!["metadata"]
                } else {
                    vec!["metadata", "read", "metadata"]
                }
            );
        }
    }
    // Preserve the original first check before even length+1 allocation arithmetic.
    assert_eq!(
        class(frozen_read_with(
            "",
            usize::MAX,
            |_| panic!("read after refusal"),
            || Ok(false)
        )),
        "drift"
    );
}

#[test]
fn context_document_buffer_baseline_short_overrun_content_and_zero_shapes_refuse() {
    for length in [0, 1, 4, 33, 37, 63] {
        let expected = "a".repeat(length.saturating_sub(1));
        for count in [0, length.saturating_sub(1), length + 1, usize::MAX] {
            if count == length {
                continue;
            }
            let calls = std::cell::Cell::new(0);
            let result = frozen_read_with(
                &expected,
                length,
                |bytes| {
                    assert_eq!(bytes.len(), length + 1);
                    calls.set(calls.get() + 1);
                    Ok(count)
                },
                || Ok(true),
            );
            assert_eq!(class(result), "drift");
            assert_eq!(calls.get(), 1);
        }
    }
    for (expected, payload) in [
        ("abc", b"abc!".as_slice()),
        ("abc", b"abd\n"),
        ("abc", b"\xffbc\n"),
        ("ab", b"abc\n"),
        ("", b"abc\n"),
    ] {
        let result = frozen_read_with(
            expected,
            4,
            |bytes| {
                bytes[..4].copy_from_slice(payload);
                Ok(4)
            },
            || Ok(true),
        );
        assert_eq!(class(result), "drift");
    }
    assert_eq!(
        class(frozen_read_with("", 0, |_| Ok(0), || Ok(true))),
        "drift"
    );
}
