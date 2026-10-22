use super::journal::*;
use super::*;
use crate::counter::{capture, Counts};
use crate::{begin_round, trace};

pub(super) fn outcome<T>(result: Result<T, AuthorityError>) -> &'static str {
    match result {
        Ok(_) => "ok",
        Err(AuthorityError::Invalid) => "invalid",
        Err(AuthorityError::Budget) => "budget",
        Err(AuthorityError::Refused) => "refused",
        Err(AuthorityError::Io(error)) => {
            assert_eq!(error.kind(), std::io::ErrorKind::Other);
            "io"
        }
    }
}

pub(super) fn sum(a: Counts, b: Counts) -> Counts {
    Counts {
        allocations: a.allocations + b.allocations,
        zeroed: a.zeroed + b.zeroed,
        reallocations: a.reallocations + b.reallocations,
        requested_bytes: a.requested_bytes + b.requested_bytes,
        deallocations: a.deallocations + b.deallocations,
        deallocated_bytes: a.deallocated_bytes + b.deallocated_bytes,
    }
}
pub(super) fn main<L: Storage>(generations: usize) -> (Counts, crate::Trace, Journal<L>) {
    let mut read = Journal::<L>::build(generations);
    let mut total = Counts::default();
    for _ in 0..16 {
        begin_round();
        let (result, counts) = capture(|| read.revalidate(Fault::default()));
        assert_eq!(outcome(result), "ok");
        total = sum(total, counts);
    }
    (total, trace(generations * 2), read)
}
#[test]
fn reference_only() {
    for event in [
        crate::Event::Bind,
        crate::Event::Transition,
        crate::Event::Publication,
        crate::Event::External,
    ] {
        assert_ne!(event.name(), "empty");
    }
    for generations in GENERATIONS {
        let (counts, events, read) = main::<Leaf>(generations);
        assert_eq!(counts.allocations, generations * 2 * 16);
        assert_eq!(counts.requested_bytes, generations * 2113 * 16);
        assert_eq!(read.capacity(), 0);
        assert_eq!(read.originals().len(), generations * 2);
        assert!(!read.refused);
        assert_eq!(read.epoch, 1);
        println!("HC074-OWNED-REFERENCE {{\"generations\":{generations},\"rounds\":16,\"counts\":{},\"trace\":{}}}",counts.json(),events.json());
    }
}
