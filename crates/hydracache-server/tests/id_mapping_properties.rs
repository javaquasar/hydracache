use std::panic::{catch_unwind, AssertUnwindSafe};

use hydracache_cluster_transport_axum::ClusterOpaqueMessage;
use proptest::prelude::*;

proptest! {
    #[test]
    fn cluster_opaque_message_decode_rejects_malformed_loud(
        payload_base64 in ".{0,256}"
    ) {
        let message = ClusterOpaqueMessage {
            from: "member-a".to_owned(),
            to: "member-b".to_owned(),
            term: 1,
            payload_base64,
        };

        let decoded = catch_unwind(AssertUnwindSafe(|| message.decode_payload()));

        prop_assert!(decoded.is_ok());
    }

    #[test]
    fn cluster_opaque_message_round_trips_valid_payload(payload in proptest::collection::vec(any::<u8>(), 0..512)) {
        let message = ClusterOpaqueMessage::new("member-a", "member-b", 7, &payload);
        let decoded = message.decode_payload().unwrap();

        prop_assert_eq!(decoded.as_ref(), payload.as_slice());
    }
}
