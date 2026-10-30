//! Common diagnostic overlay; production builds contain no hook or TLS state.
use super::*;
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};

thread_local! {
    static ARMED: Cell<bool> = const { Cell::new(false) };
    static WAS_PRESENT: Cell<bool> = const { Cell::new(false) };
}
pub(super) fn after_context_ready(present: bool) {
    if ARMED.with(|v| v.replace(false)) {
        WAS_PRESENT.with(|v| v.set(present));
        panic!("injected unwind before compression, after context creation");
    }
}

#[test]
fn disabled_hook_has_no_effect() {
    after_context_ready(false);
    after_context_ready(true);
    assert!(!ARMED.with(Cell::get));
}

#[test]
fn context_is_absent_at_append_boundary() {
    let first = "first job ".repeat(30);
    let second = "second job ".repeat(30);
    let mut writer = CompressedStreamWriter::with_block_threshold(64);
    writer.append(first.as_str());
    assert!(
        writer.compression_context.is_none(),
        "context retained after first append"
    );
    writer.append(second.as_str());
    assert!(
        writer.compression_context.is_none(),
        "context retained after second append"
    );
    let bytes = writer.finish();
    let decoded: Vec<String> = deserialize_stream(&mut bytes.as_slice()).unwrap();
    assert_eq!(decoded, [first, second]);
}

#[test]
fn panic_after_context_creation_releases_only_context() {
    let first = "first job ".repeat(30);
    let expected = first.as_str().serialize_to_vec();
    let mut writer = CompressedStreamWriter::with_block_threshold(64);
    WAS_PRESENT.with(|v| v.set(false));
    ARMED.with(|v| v.set(true));
    assert!(catch_unwind(AssertUnwindSafe(|| writer.append(first.as_str()))).is_err());
    assert!(
        WAS_PRESENT.with(Cell::get),
        "fault must follow actual owned context creation"
    );
    assert!(!ARMED.with(Cell::get));
    assert!(
        writer.compression_context.is_none(),
        "unwind retained the context"
    );
    assert!(writer.output.is_empty());
    assert_eq!(writer.uncompressed_block, expected);
    assert_eq!(writer.compressed_block.len(), 64);
    // Original append is not transactional on panic. Do not finish/retry this
    // incomplete item or invent a stronger data-recovery promise.
    drop(writer);
}

#[test]
fn serialization_panic_keeps_original_prefix() {
    struct Broken;
    impl Serializable for Broken {
        fn serialized_len(&self) -> usize {
            3
        }
        fn serialize(&self, out: &mut Vec<u8>) {
            out.extend_from_slice(&[1, 2, 3]);
            panic!("injected serialization failure");
        }
    }
    let mut writer = CompressedStreamWriter::with_block_threshold(64);
    assert!(catch_unwind(AssertUnwindSafe(|| writer.append(&Broken))).is_err());
    assert!(writer.compression_context.is_none());
    assert!(writer.output.is_empty());
    assert_eq!(writer.uncompressed_block, [1, 2, 3]);
    assert!(writer.compressed_block.is_empty());
}

#[test]
fn oversized_item_is_refused_before_serialization() {
    struct TooLarge<'a>(&'a Cell<bool>);
    impl Serializable for TooLarge<'_> {
        fn serialized_len(&self) -> usize {
            u16::MAX as usize + 1
        }
        fn serialize(&self, _: &mut Vec<u8>) {
            self.0.set(true);
            panic!("must not serialize oversized item");
        }
    }
    let mut writer = CompressedStreamWriter::with_block_threshold(64);
    let capacities = [
        writer.output.capacity(),
        writer.uncompressed_block.capacity(),
        writer.compressed_block.capacity(),
    ];
    let entered = Cell::new(false);
    assert!(catch_unwind(AssertUnwindSafe(|| writer.append(&TooLarge(&entered)))).is_err());
    assert!(
        !entered.get(),
        "oversized admission must stop before serialization"
    );
    assert!(writer.compression_context.is_none());
    assert!(
        writer.output.is_empty()
            && writer.uncompressed_block.is_empty()
            && writer.compressed_block.is_empty()
    );
    assert_eq!(
        capacities,
        [
            writer.output.capacity(),
            writer.uncompressed_block.capacity(),
            writer.compressed_block.capacity()
        ]
    );
}

#[test]
fn zero_length_generic_item_keeps_original_behavior() {
    struct Empty;
    impl Serializable for Empty {
        fn serialized_len(&self) -> usize {
            0
        }
        fn serialize(&self, _: &mut Vec<u8>) {}
    }
    let mut writer = CompressedStreamWriter::with_block_threshold(64);
    writer.append(&Empty);
    assert!(writer.compression_context.is_none());
    assert_eq!(writer.finish(), [BlockType::NoMoreBlocks as u8]);
}

#[test]
fn separate_writers_and_threads_keep_their_own_frames() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<CompressedStreamWriter>();
    let handles: Vec<_> = (0..4)
        .map(|i| {
            std::thread::spawn(move || {
                let value = format!("owner-{i}-").repeat(100);
                let mut writer = CompressedStreamWriter::with_block_threshold(100);
                writer.append(value.as_str());
                assert!(writer.compression_context.is_none());
                let raw = writer.finish();
                let decoded: Vec<String> = deserialize_stream(&mut raw.as_slice()).unwrap();
                assert_eq!(decoded, [value]);
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
}
