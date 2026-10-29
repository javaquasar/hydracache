//! Additional stateful controls for the unchanged P production implementation.
use super::*;

struct Bytes(Vec<u8>);
impl Serializable for Bytes {
    fn serialized_len(&self) -> usize {
        self.0.len()
    }
    fn serialize(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }
}

fn noise(n: usize) -> Bytes {
    let mut seed = 0x0741028_u64;
    Bytes(
        (0..n)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                seed as u8
            })
            .collect(),
    )
}

#[test]
fn populated_scratch_and_pending_survive_prediction_and_unwind() {
    let mut w = CompressedStreamWriter::with_block_threshold(16);
    w.append(&noise(35));
    assert_eq!(w.compressed_block.len(), 16);
    assert_eq!(w.uncompressed_block.len(), 3);
    let expected = w.clone();
    let item = noise(40); // Last block is partial; scratch must grow back to 16.
    assert_eq!(
        w.counted_upperbound_after(&item),
        expected.serialized_len_upperbound_after(&item)
    );
    assert_eq!(w.compressed_block.len(), 16);
    assert_eq!(
        w.compressed_block.capacity(),
        expected.compressed_block.capacity()
    );
    assert_eq!(w.uncompressed_block, expected.uncompressed_block);
    assert_eq!(w.output, expected.output);
    struct Broken;
    impl Serializable for Broken {
        fn serialized_len(&self) -> usize {
            40
        }
        fn serialize(&self, out: &mut Vec<u8>) {
            out.extend_from_slice(b"partial");
            panic!("controlled populated-state panic");
        }
    }
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || w.counted_upperbound_after(&Broken)
        ))
        .is_err()
    );
    assert_eq!(w.compressed_block.len(), 16);
    assert_eq!(w.uncompressed_block, expected.uncompressed_block);
    assert_eq!(w.output, expected.output);
    w.append("after");
    let mut expected = expected;
    expected.append("after");
    assert_eq!(w.finish(), expected.finish());
}

#[test]
fn actual_helper_preserves_mixed_multiblock_streams() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<CompressedStreamWriter>();
    for threshold in [16, 1000, 16384, 65535] {
        let mut w = CompressedStreamWriter::with_block_threshold(threshold);
        let mut expected = w.clone();
        let random = noise(4096);
        for item in [
            Bytes(vec![b'r'; 4000]),
            Bytes("λ😀Ключ\0".repeat(333).into_bytes()),
            random,
        ] {
            let bound = expected.serialized_len_upperbound_after(&item);
            assert_eq!(w.counted_upperbound_after(&item), bound);
            for budget in [0, bound - 1, bound, bound + 1] {
                assert_eq!(w.fits_budget_after(&item, budget), bound <= budget);
            }
            w.append(&item);
            expected.append(&item);
            assert_eq!(w.output, expected.output);
            assert_eq!(w.uncompressed_block, expected.uncompressed_block);
            assert_eq!(w.compressed_block.len(), expected.compressed_block.len());
        }
        assert_eq!(w.finish(), expected.finish());
    }
}

#[test]
fn ambiguous_rejection_restores_state_and_records_retained_capacity() {
    let mut w = CompressedStreamWriter::with_block_threshold(16);
    w.append(&noise(35));
    let expected = w.clone();
    let item = noise(4096);
    let bound = expected.serialized_len_upperbound_after(&item);
    let old = expected.output.len() + expected.uncompressed_block.len() + item.serialized_len() + 7;
    assert!(bound > old);
    let before = w.uncompressed_block.capacity();
    assert!(!w.fits_budget_after(&item, bound - 1));
    assert_eq!(w.output, expected.output);
    assert_eq!(w.uncompressed_block, expected.uncompressed_block);
    assert_eq!(w.compressed_block.len(), expected.compressed_block.len());
    let after = w.uncompressed_block.capacity();
    assert!(after >= before && after <= 2 * (3 + 4096));
    println!("HC074_PENDING_CAPACITY:{{\"before\":{before},\"after\":{after},\"logical_len\":3}}");
    w.append("after");
    let mut expected = expected;
    expected.append("after");
    assert_eq!(w.finish(), expected.finish());
}
