//! Identical formatter bytes with isolated initial String capacity.
use super::*;
#[derive(Clone, Copy)]
pub(crate) enum Reading {
    Reference,
    Proposed,
    Actual,
}
fn reference(revision: usize, suffix: &str) -> String {
    format!("{revision:016x}.{suffix}")
}
fn proposed(revision: usize, suffix: &str) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(17 + suffix.len());
    write!(&mut output, "{revision:016x}.{suffix}").expect("String formatting cannot fail");
    output
}
pub(crate) fn render(mode: Reading, revision: usize, suffix: &str) -> String {
    match mode {
        Reading::Reference => reference(revision, suffix),
        Reading::Proposed => proposed(revision, suffix),
        Reading::Actual => name(revision, suffix),
    }
}
#[test]
fn authority_name_capacity_seeded_boundaries_and_arbitrary_suffixes_preserve_exact_bytes() {
    let mut revisions: Vec<_> = (0..=MAX_GENERATIONS).collect();
    revisions.extend([255, 256, 4095, 4096, usize::MAX - 1, usize::MAX]);
    for shift in (4..usize::BITS).step_by(4) {
        let boundary = 1usize << shift;
        revisions.extend([boundary - 1, boundary, boundary + 1]);
    }
    let mut seed = 0x074c_a9ac_u64;
    for _ in 0..256 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        revisions.push(seed as usize);
    }
    let long = "é\0json.commit".repeat(128);
    for revision in revisions {
        for suffix in ["", "json", "commit", ".json", "\0", "世界", long.as_str()] {
            let expected = reference(revision, suffix);
            assert_eq!(expected.len(), 17 + suffix.len());
            assert_eq!(expected.as_bytes()[16], b'.');
            for mode in [Reading::Proposed, Reading::Actual] {
                assert_eq!(
                    render(mode, revision, suffix).as_bytes(),
                    expected.as_bytes()
                );
            }
        }
    }
    eprintln!("name capacity seed=0x074ca9ac bounded-depths=0..128 seeded=256 suffixes=7");
}
