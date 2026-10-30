//! F input/transaction controls. Original serializer is the independent oracle.
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
fn old(w: &CompressedStreamWriter, n: usize) -> usize {
    let total = w.uncompressed_block.len() + n;
    w.output.len() + total + if total > w.block_threshold { 7 } else { 4 }
}
fn oracle(w: &CompressedStreamWriter, item: &impl Serializable) -> usize {
    let mut saved = w.clone();
    saved.append(item);
    old(w, item.serialized_len()).max(saved.finish().len())
}
#[test]
fn borrowed_strings_equal_public_bytes() {
    for s in [
        "",
        "x",
        "ёж🙂",
        "a\0z",
        &"я".repeat(32766),
        &"x".repeat(65535),
        &"z".repeat(65536),
    ] {
        let mut projected = Vec::new();
        s.visit(&mut |v| projected.extend_from_slice(v));
        assert_eq!(projected, s.serialize_to_vec());
    }
}
#[test]
fn tiny_mtu_counterexample() {
    let item = Blob(vec![37, 59]);
    let mut w = CompressedStreamWriter::with_block_threshold(1);
    w.append(&Blob(vec![11]));
    let budget = old(&w, 2);
    assert!(oracle(&w, &item) > budget);
    assert!(!w.try_append_borrowed(&item, budget));
}
#[test]
fn generic_bound_counterexample() {
    let item = Blob(vec![11, 37, 59]);
    let w = CompressedStreamWriter::with_block_threshold(1);
    assert!(w.serialized_len_upperbound_after(&item) >= oracle(&w, &item));
}
#[test]
fn exact_admission_and_wire() {
    for threshold in [1, 2, 3, 7, 16, 64, 127, 1000, 16384, 65535] {
        for n in [1, 3, 16, 35, 129, 160, 257, 4096] {
            for compressible in [false, true] {
                let item = if compressible {
                    Blob(vec![b'r'; n])
                } else {
                    noise(n)
                };
                let mut original = CompressedStreamWriter::with_block_threshold(threshold);
                original.append(&Blob(vec![17; threshold.min(35) as usize]));
                let bound = oracle(&original, &item);
                for budget in [
                    old(&original, n),
                    bound.saturating_sub(1),
                    bound,
                    usize::MAX,
                ] {
                    let mut w = original.clone();
                    let expected = bound <= budget;
                    let accepted = w.try_append_borrowed(&item, budget);
                    assert_eq!(
                        accepted, expected,
                        "threshold={threshold} n={n} budget={budget}"
                    );
                    let mut reference = original.clone();
                    if expected {
                        reference.append(&item);
                    }
                    assert_eq!(w.finish(), reference.finish());
                }
            }
        }
    }
}
#[test]
fn full_item_domain_and_unbounded_mtu() {
    let item = noise(65535);
    for t in [1, 2, 255, 16384, 65534, 65535] {
        let mut w = CompressedStreamWriter::with_block_threshold(t);
        w.append(&Blob(vec![1; t as usize]));
        let expected = oracle(&w, &item);
        let mut reference = w.clone();
        reference.append(&item);
        assert!(w.try_append_borrowed(&item, expected));
        assert_eq!(w.finish(), reference.finish());
    }
    let mut w = CompressedStreamWriter::with_block_threshold(65535);
    assert!(w.try_append_borrowed(&item, usize::MAX));
}
#[test]
fn refusal_preserves_bytes_and_all_three_capacities() {
    let mut w = CompressedStreamWriter::with_block_threshold(64);
    let mut mixed = noise(128);
    mixed.0.extend_from_slice(&[b'r'; 32]);
    assert!(w.try_append_borrowed(&mixed, 167));
    let before = w.clone();
    let caps = [
        w.output.capacity(),
        w.uncompressed_block.capacity(),
        w.compressed_block.capacity(),
    ];
    assert!(!w.try_append_borrowed(&noise(4096), 4269));
    assert_eq!(
        caps,
        [
            w.output.capacity(),
            w.uncompressed_block.capacity(),
            w.compressed_block.capacity()
        ]
    );
    assert_eq!(w.output, before.output);
    assert_eq!(w.uncompressed_block, before.uncompressed_block);
    assert_eq!(w.tail_cache, before.tail_cache);
    assert_eq!(w.compressed_block, before.compressed_block);
    assert!(w.compression_context.is_none());
    assert_eq!(w.finish(), before.finish());
}
#[test]
fn projection_panic_restores_original_cache() {
    struct Panic;
    impl Serializable for Panic {
        fn serialize(&self, _: &mut Vec<u8>) {
            panic!("not the borrowed path");
        }
        fn serialized_len(&self) -> usize {
            160
        }
    }
    impl BorrowedItem for Panic {
        fn visit(&self, sink: &mut impl FnMut(&[u8])) {
            sink(&[71; 128]);
            panic!("projection fault");
        }
    }
    let mut w = CompressedStreamWriter::with_block_threshold(64);
    let mut mixed = noise(128);
    mixed.0.extend_from_slice(&[b'r'; 32]);
    assert!(w.try_append_borrowed(&mixed, 167));
    let before = w.clone();
    let caps = [
        w.output.capacity(),
        w.uncompressed_block.capacity(),
        w.compressed_block.capacity(),
    ];
    let budget = old(&w, 160);
    assert!(catch_unwind(AssertUnwindSafe(|| w.try_append_borrowed(&Panic, budget))).is_err());
    assert_eq!(
        caps,
        [
            w.output.capacity(),
            w.uncompressed_block.capacity(),
            w.compressed_block.capacity()
        ]
    );
    assert_eq!(w.output, before.output);
    assert_eq!(w.uncompressed_block, before.uncompressed_block);
    assert_eq!(w.compressed_block, before.compressed_block);
    assert_eq!(w.tail_cache, before.tail_cache);
    assert!(w.compression_context.is_none());
    assert_eq!(w.finish(), before.finish());
}
#[test]
fn independent_writers() {
    let threads: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                let mut w = CompressedStreamWriter::with_block_threshold(3);
                let item = noise(129);
                let budget = oracle(&w, &item);
                assert!(w.try_append_borrowed(&item, budget));
                w.finish()
            })
        })
        .collect();
    let wires: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert!(wires.windows(2).all(|p| p[0] == p[1]));
}

#[test]
fn inconsistent_private_projection_fails_loud_without_loop() {
    struct Bad(usize);
    impl Serializable for Bad {
        fn serialize(&self, _: &mut Vec<u8>) {
            panic!("borrowed path required");
        }
        fn serialized_len(&self) -> usize {
            3
        }
    }
    impl BorrowedItem for Bad {
        fn visit(&self, sink: &mut impl FnMut(&[u8])) {
            sink(&[13; 4][..self.0]);
        }
    }
    for length in [2, 4] {
        let mut w = CompressedStreamWriter::with_block_threshold(1);
        let before = w.clone();
        assert!(
            catch_unwind(AssertUnwindSafe(|| w.try_append_borrowed(&Bad(length), 10))).is_err()
        );
        assert!(w.compression_context.is_none());
        assert_eq!(w.output, before.output);
        assert_eq!(w.uncompressed_block, before.uncompressed_block);
        assert_eq!(w.finish(), before.finish());
    }
}
