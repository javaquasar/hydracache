//! Pure candidate requests and full-body differential; no File, clock or live owner.
use super::*;
use crate::diagnostic_worker_policy::local_context::{
    document_stack_controls as candidate, WorkerContextError,
};
use std::cell::{Cell, RefCell};

fn reference(
    expected: &str,
    length: usize,
    read: impl FnOnce(&mut [u8]) -> std::io::Result<usize>,
    mut metadata: impl FnMut() -> Result<bool, WorkerContextError>,
) -> Result<(), WorkerContextError> {
    if !metadata()? {
        return Err(WorkerContextError::Drift);
    }
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

#[test]
fn context_document_stack_candidate_requests() {
    for length in [0, 1, 4, 32, 33, 36, 37, 38, 63] {
        let (_, counts) = capture(|| {
            for _ in 0..256 {
                candidate::with_buffer(std::hint::black_box(length), |buffer| {
                    let bytes = std::hint::black_box(buffer);
                    assert_eq!(bytes.len(), length + 1);
                    assert!(bytes.iter().all(|b| *b == 0));
                });
            }
        });
        let allocations = if length < 38 { 0 } else { 256 };
        assert_eq!(counts.allocations, allocations);
        assert_eq!(counts.reallocations, 0);
        assert_eq!(counts.requested_bytes, allocations * (length as u64 + 1));
        println!(
            "HC074-CONTEXT-DOCUMENT-CANDIDATE {}",
            serde_json::json!({
                "length":length,"iterations":256,"counts":counts,
                "production_length":([33,37].contains(&length)),"rust_requests_only":true,
                "timing_claim":false,"profile":false,"server":false,"product":false
            })
        );
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Metadata,
    Read { length: usize, zero: bool },
}

fn run(
    new: bool,
    length: usize,
    payload: &[u8],
    count: usize,
    read_error: Option<i32>,
    fault: u8,
) -> (String, Vec<Event>) {
    let expected = "a".repeat(length.saturating_sub(1));
    let events = RefCell::new(Vec::new());
    let calls = Cell::new(0);
    let metadata = || {
        events.borrow_mut().push(Event::Metadata);
        calls.set(calls.get() + 1);
        let fail = match fault {
            1 | 2 => calls.get() == 1,
            3 | 4 => calls.get() == 2,
            _ => false,
        };
        if fail && fault.is_multiple_of(2) {
            Err(std::io::Error::from_raw_os_error(libc::EACCES).into())
        } else {
            Ok(!fail)
        }
    };
    let read = |buffer: &mut [u8]| {
        events.borrow_mut().push(Event::Read {
            length: buffer.len(),
            zero: buffer.iter().all(|b| *b == 0),
        });
        if let Some(code) = read_error {
            return Err(std::io::Error::from_raw_os_error(code));
        }
        let copied = payload.len().min(buffer.len());
        buffer[..copied].copy_from_slice(&payload[..copied]);
        Ok(count)
    };
    let result = if new {
        candidate::read_model(&expected, length, read, metadata)
    } else {
        reference(&expected, length, read, metadata)
    };
    let result = match result {
        Ok(()) => "ok".into(),
        Err(WorkerContextError::Drift) => "drift".into(),
        Err(WorkerContextError::Io(e)) => format!("io:{:?}", e.raw_os_error()),
        Err(e) => panic!("unexpected model refusal: {e}"),
    };
    (result, events.into_inner())
}

#[test]
fn context_document_stack_seeded_body_error_and_schedule_differential() {
    let mut seed = 0x074c_5a19_u64;
    for sample in 0..384 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let length = [0_usize, 1, 4, 32, 33, 36, 37, 38, 39, 63, 64, 127, 256][sample % 13];
        let mut payload = vec![b'a'; length];
        if let Some(last) = payload.last_mut() {
            *last = b'\n';
        }
        if (seed >> 32).is_multiple_of(3) && !payload.is_empty() {
            let i = seed as usize % payload.len();
            payload[i] = 0xff;
        }
        let count = [0, length.saturating_sub(1), length, length + 1, usize::MAX]
            [(seed >> 16) as usize % 5];
        let error =
            [None, Some(libc::EINTR), Some(libc::EIO), Some(libc::EBADF)][(seed >> 8) as usize % 4];
        let fault = (seed % 5) as u8;
        assert_eq!(
            run(false, length, &payload, count, error, fault),
            run(true, length, &payload, count, error, fault),
            "seed=0x074c5a19 sample={sample}"
        );
    }
    println!("HC074-CONTEXT-STACK-DIFFERENTIAL seed=0x074c5a19 samples=384 model_only=true");
}

#[test]
fn context_document_stack_boundary_owned_storage_and_healthy_models() {
    for length in [0_usize, 1, 4, 33, 37, 38, 39, 63, 256] {
        candidate::with_buffer(length, |outer| {
            assert_eq!(outer.len(), length + 1);
            outer[0] = 0xa5;
            candidate::with_buffer(length, |inner| {
                assert_eq!(inner.len(), length + 1);
                assert!(inner.iter().all(|b| *b == 0));
                inner[0] = 0x5a;
                assert_eq!(outer[0], 0xa5);
            });
        });
        if length > 0 {
            let mut payload = vec![b'a'; length];
            payload[length - 1] = b'\n';
            let old = run(false, length, &payload, length, None, 0);
            assert_eq!(old.0, "ok");
            assert_eq!(old, run(true, length, &payload, length, None, 0));
        }
    }
}

#[test]
fn context_document_stack_explicit_first_refusals_and_no_arithmetic_before_metadata() {
    for length in [4_usize, 33, 37, 38, 63] {
        for fault in 0..5 {
            for code in [None, Some(libc::EINTR), Some(libc::EIO), Some(libc::EBADF)] {
                assert_eq!(
                    run(false, length, b"bad!", 0, code, fault),
                    run(true, length, b"bad!", 0, code, fault)
                );
            }
        }
    }
    for new in [false, true] {
        let result = if new {
            candidate::read_model(
                "",
                usize::MAX,
                |_| panic!("read after refusal"),
                || Ok(false),
            )
        } else {
            reference(
                "",
                usize::MAX,
                |_| panic!("read after refusal"),
                || Ok(false),
            )
        };
        assert!(matches!(result, Err(WorkerContextError::Drift)));
    }
}
