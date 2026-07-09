use std::panic::{catch_unwind, AssertUnwindSafe};

use hydracache_cluster_raft::RaftWireMessage;
use proptest::prelude::*;

proptest! {
    #[test]
    fn raft_wire_message_decode_never_panics(
        from in 1_u64..u64::MAX,
        to in 1_u64..u64::MAX,
        term in 0_u64..u64::MAX,
        payload in proptest::collection::vec(any::<u8>(), 0..1024)
    ) {
        let wire = RaftWireMessage { from, to, term, payload };

        let decoded = catch_unwind(AssertUnwindSafe(|| wire.decode()));

        prop_assert!(decoded.is_ok());
    }
}
