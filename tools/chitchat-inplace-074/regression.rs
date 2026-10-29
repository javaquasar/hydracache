//! Production-helper checks against the explicitly separate immutable S oracle.
use super::*;
use std::cell::Cell;

thread_local! {
    static CALLS: Cell<[u64; 4]> = const { Cell::new([0; 4]) };
}

pub(crate) fn record(index: usize) {
    CALLS.with(|c| {
        let mut counts = c.get();
        counts[index] += 1;
        c.set(counts);
    });
}

pub(crate) fn reset() {
    CALLS.with(|c| c.set([0; 4]));
}
pub(crate) fn take() -> [u64; 4] {
    CALLS.with(Cell::get)
}

struct Blob(Vec<u8>);
impl Serializable for Blob {
    fn serialize(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }
    fn serialized_len(&self) -> usize {
        self.0.len()
    }
}

fn blob(size: usize) -> Blob {
    let mut seed = 0x0741028_u64;
    Blob(
        (0..size)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                seed as u8
            })
            .collect(),
    )
}

fn previous(w: &CompressedStreamWriter, n: usize) -> usize {
    w.output.len()
        + w.uncompressed_block.len()
        + n
        + if w.uncompressed_block.len() + n > w.block_threshold {
            6
        } else {
            3
        }
        + 1
}

fn reference(w: &CompressedStreamWriter, item: &Blob) -> usize {
    let mut copy = w.clone();
    copy.append(item);
    previous(w, item.serialized_len()).max(copy.finish().len())
}

#[test]
fn actual_predictor_matches_exact_reference() {
    for threshold in [1u16, 16, 1000, 16384, 65535] {
        for size in [1usize, 64, 4096, 39953, 65535] {
            let item = blob(size);
            let mut w = CompressedStreamWriter::with_block_threshold(threshold);
            w.append("node");
            let expected = reference(&w, &item);
            assert_eq!(w.counted_upperbound_after(&item), expected);
            w.append(&item);
            assert!(w.finish().len() <= expected);
        }
    }
}

#[test]
fn budget_matches_reference_without_losing_valid_admissions() {
    for threshold in [1u16, 16, 1000, 16384, 65535] {
        for size in [1usize, 64, 4096, 39953, 65535] {
            let item = blob(size);
            let mut w = CompressedStreamWriter::with_block_threshold(threshold);
            w.append("node");
            let expected = reference(&w, &item);
            for budget in [0, previous(&w, size), expected - 1, expected, expected + 1] {
                assert_eq!(w.fits_budget_after(&item, budget), expected <= budget);
            }
        }
    }
}

#[test]
fn prediction_restores_logical_state_and_bounds_capacity() {
    let mut w = CompressedStreamWriter::with_block_threshold(1000);
    w.append("node");
    let output = w.output.clone();
    let pending = w.uncompressed_block.clone();
    let scratch_len = w.compressed_block.len();
    let scratch_cap = w.compressed_block.capacity();
    let capacity = w.uncompressed_block.capacity();
    let item = blob(65535);
    w.counted_upperbound_after(&item);
    assert_eq!(w.output, output);
    assert_eq!(w.uncompressed_block, pending);
    assert_eq!(w.compressed_block.len(), scratch_len);
    assert_eq!(w.compressed_block.capacity(), scratch_cap);
    assert!(
        w.uncompressed_block.capacity() <= 2 * capacity.max(pending.len() + item.serialized_len())
    );
    w.append("after");
    let mut expected = CompressedStreamWriter::with_block_threshold(1000);
    expected.append("node");
    expected.append("after");
    assert_eq!(w.finish(), expected.finish());
}

#[test]
fn unwind_restores_pending_prefix_and_scratch_length() {
    struct Broken;
    impl Serializable for Broken {
        fn serialized_len(&self) -> usize {
            512
        }
        fn serialize(&self, out: &mut Vec<u8>) {
            out.extend_from_slice(b"partial");
            panic!("controlled serializer panic");
        }
    }
    let mut w = CompressedStreamWriter::with_block_threshold(16);
    w.append("node");
    let pending = w.uncompressed_block.clone();
    let output = w.output.clone();
    let scratch_len = w.compressed_block.len();
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        w.counted_upperbound_after(&Broken)
    }));
    assert!(failed.is_err());
    assert_eq!(w.output, output);
    assert_eq!(w.uncompressed_block, pending);
    assert_eq!(w.compressed_block.len(), scratch_len);
    w.append("after");
    let mut expected = CompressedStreamWriter::with_block_threshold(16);
    expected.append("node");
    expected.append("after");
    assert_eq!(w.finish(), expected.finish());
}

#[test]
fn old_budget_rejection_does_not_serialize_or_grow() {
    struct Forbidden;
    impl Serializable for Forbidden {
        fn serialized_len(&self) -> usize {
            60000
        }
        fn serialize(&self, _: &mut Vec<u8>) {
            panic!("over-budget serialization is forbidden");
        }
    }
    let mut w = CompressedStreamWriter::with_block_threshold(1000);
    w.append("node");
    let before = (
        w.output.clone(),
        w.uncompressed_block.clone(),
        w.compressed_block.clone(),
        [
            w.output.capacity(),
            w.uncompressed_block.capacity(),
            w.compressed_block.capacity(),
        ],
    );
    assert!(!w.fits_budget_after(&Forbidden, 1000));
    assert_eq!(
        [
            w.output.capacity(),
            w.uncompressed_block.capacity(),
            w.compressed_block.capacity()
        ],
        before.3
    );
    assert_eq!(
        (w.output, w.uncompressed_block, w.compressed_block),
        (before.0, before.1, before.2)
    );
}

#[test]
fn independent_writers_do_not_share_scratch() {
    std::thread::scope(|scope| {
        let mut jobs = Vec::new();
        for id in 0..8usize {
            jobs.push(scope.spawn(move || {
                for n in [64, 4096] {
                    let item = blob(n + id);
                    let mut w = CompressedStreamWriter::with_block_threshold(16);
                    let expected = reference(&w, &item);
                    assert_eq!(w.counted_upperbound_after(&item), expected);
                    w.append(&item);
                    assert!(w.finish().len() <= expected);
                }
            }));
        }
        for job in jobs {
            job.join().unwrap();
        }
    });
}
