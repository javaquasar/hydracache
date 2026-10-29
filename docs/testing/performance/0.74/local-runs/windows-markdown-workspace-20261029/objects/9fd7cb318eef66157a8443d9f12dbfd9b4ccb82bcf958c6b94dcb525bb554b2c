//! Fixed full-method synthetic cost harness. Never an admission clock.
use super::*;
use crate::counter::capture;
use crate::{mark_held, reset, rewind, Scenario};
use std::hint::black_box;
use std::time::Instant;

pub(super) const COSTS: [(usize, usize); 2] = [(65, 65), (2048, 2048)];
pub(super) fn owner(length: usize, dir: &File) -> Leaf {
    reset(Scenario::new(length, length));
    let leaf = Leaf::open(dir, "record".into(), 1001, 1002, length).unwrap();
    mark_held(&leaf.file);
    leaf
}
pub(super) fn calls(
    mut call: impl FnMut() -> Result<(), AuthorityError>,
    length: usize,
    n: usize,
) -> usize {
    let mut checksum = 0;
    for _ in 0..n {
        rewind(Scenario::new(length, length));
        black_box(call()).unwrap();
        checksum += black_box(length);
    }
    checksum
}
pub(super) fn interval(
    call: impl FnMut() -> Result<(), AuthorityError>,
    length: usize,
) -> (u128, usize) {
    let start = Instant::now();
    let checksum = calls(call, length, 512);
    (start.elapsed().as_nanos(), checksum)
}

#[test]
fn reference_only_preflight() {
    let dir = File::directory();
    for (length, budget) in COSTS {
        for sample in 0..7 {
            let retained = owner(length, &dir);
            calls(|| retained.revalidate(&dir, 1001, 1002, budget), length, 64);
            let (ns, checksum) = interval(|| retained.revalidate(&dir, 1001, 1002, budget), length);
            assert_eq!(checksum, length * 512);
            println!("HC074-SCRATCH-PREFLIGHT {{\"length\":{length},\"budget\":{budget},\"sample\":{sample},\"warm\":64,\"calls\":512,\"ns\":{ns},\"checksum\":{checksum}}}");
        }
        let retained = owner(length, &dir);
        calls(|| retained.revalidate(&dir, 1001, 1002, budget), length, 64);
        let (checksum, counts) = capture(|| {
            calls(
                || retained.revalidate(&dir, 1001, 1002, budget),
                length,
                512,
            )
        });
        assert_eq!(checksum, length * 512);
        println!("HC074-SCRATCH-PREFLIGHT-COUNTS {{\"length\":{length},\"calls\":512,\"checksum\":{checksum},\"counts\":{}}}", counts.json());
    }
}
