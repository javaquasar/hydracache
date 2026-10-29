//! Synthetic reads only. No pathname, syscall, clock or actual sender is used.
use super::*;
use crate::counter::{capture, Counts};
use crate::diagnostic_ipc::validation_trace::kernel_walk;
use std::cell::Cell;

const LENGTHS: [usize; 4] = [16, 37, 4096, 128];
fn four_reads(scratch: &mut DocumentBuffer) {
    let original = scratch.bytes.as_ptr();
    for (index, length) in LENGTHS.into_iter().enumerate() {
        let calls = Cell::new(0);
        let raw = scratch
            .read_with(|bytes| {
                calls.set(calls.get() + 1);
                assert_eq!(bytes.len(), DOC_BYTES + 1);
                assert_eq!(bytes.as_ptr(), original);
                bytes[..length].fill(index as u8 + 1);
                Ok(length)
            })
            .unwrap();
        assert_eq!(calls.get(), 1);
        assert_eq!(raw.len(), length);
        assert!(raw.iter().all(|byte| *byte == index as u8 + 1));
        std::hint::black_box(raw);
    }
}

#[test]
fn proc_document_constructor_and_four_reads_requests() {
    let before = kernel_walk::calls();
    let (_, counts) = capture(|| {
        for _ in 0..256 {
            let mut scratch = std::hint::black_box(DocumentBuffer::new());
            assert!(scratch.bytes.iter().all(|byte| *byte == 0));
            four_reads(&mut scratch);
        }
    });
    assert_eq!(kernel_walk::calls() - before, 1024);
    assert_eq!(
        counts,
        Counts {
            allocations: 256,
            zeroed: 256,
            reallocations: 0,
            requested_bytes: 256 * 65_537,
            deallocations: 256,
            deallocated_bytes: 256 * 65_537,
        }
    );
    println!("HC074-PROC-DOCUMENT-MAIN {{\"instances\":256,\"reads_per_instance\":4,\"lengths\":[16,37,4096,128],\"callbacks\":1024,\"counts\":{},\"model_only\":true,\"timing_claim\":false,\"server\":false,\"product\":false}}",counts.json());
}

#[test]
fn proc_document_separate_stages_conserve_one_instance() {
    let (mut scratch, created) = capture(|| std::hint::black_box(DocumentBuffer::new()));
    let (_, reused) = capture(|| four_reads(&mut scratch));
    let (_, dropped) = capture(|| drop(scratch));
    let (_, complete) = capture(|| {
        let mut scratch = std::hint::black_box(DocumentBuffer::new());
        four_reads(&mut scratch);
    });
    assert_eq!(created.allocations, 1);
    assert_eq!(created.zeroed, 1);
    assert_eq!(created.requested_bytes, 65_537);
    assert_eq!(created.deallocations, 0);
    assert_eq!(reused, Counts::default());
    assert_eq!(
        dropped,
        Counts {
            deallocations: 1,
            deallocated_bytes: 65_537,
            ..Counts::default()
        }
    );
    assert_eq!(
        complete,
        Counts {
            deallocations: 1,
            deallocated_bytes: 65_537,
            ..created
        }
    );
    println!("HC074-PROC-DOCUMENT-STAGES {{\"instances_per_arm\":1,\"not_pooled\":true,\"created\":{},\"reused\":{},\"dropped\":{},\"complete\":{}}}",created.json(),reused.json(),dropped.json(),complete.json());
}

#[test]
fn proc_document_seeded_visible_prefix_and_single_callback() {
    let mut scratch = DocumentBuffer::new();
    let mut seed = 0x074d_5a18_u64;
    let mut callbacks = 0;
    for sample in 0..256 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let length = [0, 1, 2, 37, 4096, DOC_BYTES - 1, DOC_BYTES, DOC_BYTES + 1][sample % 8];
        let value = seed as u8;
        scratch.bytes.fill(!value);
        let result = scratch.read_with(|bytes| {
            callbacks += 1;
            assert_eq!(bytes.len(), DOC_BYTES + 1);
            bytes[..length].fill(value);
            Ok(length)
        });
        if length == 0 || length > DOC_BYTES {
            assert!(matches!(
                result,
                Err(KernelError::Invalid("proc-document-budget"))
            ));
        } else {
            let raw = result.unwrap();
            assert_eq!(raw.len(), length, "seed=0x074d5a18 sample={sample}");
            assert!(raw.iter().all(|byte| *byte == value));
        }
    }
    assert_eq!(callbacks, 256);
    println!("HC074-PROC-DOCUMENT-SEEDED {{\"seed\":\"0x074d5a18\",\"samples\":256,\"callbacks\":256,\"model_only\":true}}");
}

#[test]
fn proc_document_errors_are_propagated_once_without_retry_or_view() {
    let mut scratch = DocumentBuffer::new();
    for kind in [
        std::io::ErrorKind::Interrupted,
        std::io::ErrorKind::Other,
        std::io::ErrorKind::PermissionDenied,
    ] {
        let calls = Cell::new(0);
        let result = scratch.read_with(|bytes| {
            calls.set(calls.get() + 1);
            bytes[0] = 17;
            Err(std::io::Error::from(kind))
        });
        assert_eq!(calls.get(), 1);
        assert!(matches!(result,Err(KernelError::Io(error)) if error.kind()==kind));
    }
    for code in [4, 5, 9] {
        let result = scratch.read_with(|_| Err(std::io::Error::from_raw_os_error(code)));
        assert!(matches!(result,Err(KernelError::Io(error)) if error.raw_os_error()==Some(code)));
    }
    println!("HC074-PROC-DOCUMENT-ERRORS {{\"error_kind_cases\":3,\"raw_error_cases\":3,\"retry\":false,\"model_only\":true}}");
}

#[test]
fn proc_document_independent_ownership_and_long_to_short_tail() {
    let mut first = DocumentBuffer::new();
    first
        .read_with(|bytes| {
            bytes.fill(0xaa);
            Ok(DOC_BYTES)
        })
        .unwrap();
    let pointer = first.bytes.as_ptr();
    let mut second = DocumentBuffer::new();
    assert_ne!(pointer, second.bytes.as_ptr());
    assert!(second.bytes.iter().all(|byte| *byte == 0));
    second
        .read_with(|bytes| {
            bytes[0] = 7;
            Ok(1)
        })
        .unwrap();
    assert_eq!(first.bytes[0], 0xaa);
    drop(second);
    let raw = first
        .read_with(|bytes| {
            bytes[0] = 0xbb;
            Ok(1)
        })
        .unwrap();
    assert_eq!(raw, &[0xbb]);
    // Old tail is allowed in scratch, but never included in the returned view.
    assert!(first.bytes[1..].iter().all(|byte| *byte == 0xaa));
    assert_eq!(first.bytes.as_ptr(), pointer);
}
