//! Commit-order controls. Reference generic append is unchanged, not the predictor.
use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

struct Blob(Vec<u8>);
impl Serializable for Blob {
    fn serialize(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }
    fn serialized_len(&self) -> usize {
        self.0.len()
    }
}
impl BorrowedItem for Blob {
    fn visit(&self, sink: &mut impl FnMut(&[u8])) {
        sink(&self.0);
    }
}
fn noise(n: usize) -> Blob {
    let mut seed = 0x0741028_u64;
    Blob(
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
fn capacities(w: &CompressedStreamWriter) -> [usize; 3] {
    [
        w.output.capacity(),
        w.uncompressed_block.capacity(),
        w.compressed_block.capacity(),
    ]
}
#[test]
fn retained_finish_matches_original_growth() {
    let mut item = noise(128);
    item.0.extend_from_slice(&[b'r'; 32]);
    let mut w = CompressedStreamWriter::with_block_threshold(64);
    let mut reference = CompressedStreamWriter::with_block_threshold(64);
    assert!(w.try_append_borrowed(&item, 167));
    reference.append(&item);
    let append = capacities(&w);
    let reference_append = capacities(&reference);
    assert!(append.iter().zip(reference_append).all(|(a, b)| *a <= b));
    let actual = w.finish();
    let original = reference.finish();
    assert_eq!(actual, original);
    assert_eq!(original.len(), 155);
    assert_eq!(original.capacity(), 256);
    assert!(actual.capacity() <= original.capacity());
    println!(
        "HC074_FRAME_UNIT:{}",
        serde_json::json!({"variant":FRAME_VARIANT,"append":append,
      "reference_append":reference_append,"finish":actual.capacity(),"reference_finish":original.capacity()})
    );
}
#[test]
fn accepted_prefixes_keep_original_output_growth() {
    for threshold in [1, 2, 16, 64, 1000, 16384, 65535] {
        for length in [1, 35, 160, 257, 4096] {
            let mut w = CompressedStreamWriter::with_block_threshold(threshold);
            let mut reference = CompressedStreamWriter::with_block_threshold(threshold);
            for n in [length, 3, length / 2 + 1] {
                let item = noise(n);
                let mut preview = reference.clone();
                preview.append(&item);
                let raw = reference.uncompressed_block.len() + n;
                let old =
                    reference.output.len() + raw + if raw > threshold as usize { 7 } else { 4 };
                let exact = old.max(preview.finish().len());
                assert!(w.try_append_borrowed(&item, exact));
                reference.append(&item);
                assert_eq!(w.output, reference.output);
                assert_eq!(w.uncompressed_block, reference.uncompressed_block);
                assert_eq!(w.output.capacity(), reference.output.capacity());
                assert!(
                    capacities(&w)
                        .iter()
                        .zip(capacities(&reference))
                        .all(|(a, b)| *a <= b)
                );
            }
            let bytes = w.finish();
            let expected = reference.finish();
            assert_eq!(bytes, expected);
            assert_eq!(bytes.capacity(), expected.capacity());
        }
    }
}
#[test]
fn maximum_simulation_refusal_keeps_all_capacities() {
    let item = noise(65535);
    let mut w = CompressedStreamWriter::with_block_threshold(1);
    let old = capacities(&w);
    let before = w.clone();
    assert!(!w.try_append_borrowed(&item, 65542));
    assert_eq!(capacities(&w), old);
    assert!(w.compression_context.is_none());
    assert_eq!(w.output, before.output);
    assert_eq!(w.uncompressed_block, before.uncompressed_block);
    assert_eq!(w.finish(), before.finish());
}
#[test]
fn commit_refuses_a_bad_first_internal_frame_before_effects() {
    let mut w = CompressedStreamWriter::with_block_threshold(64);
    let original = w.output.clone();
    let before = capacities(&w);
    for prefix in [&[1, 2][..], &[0, 0, 0][..], &[2, 2, 0, 17][..]] {
        assert!(catch_unwind(AssertUnwindSafe(|| w.commit_simulated_prefix(prefix))).is_err());
        assert_eq!(w.output, original);
        assert_eq!(capacities(&w), before);
    }
}
