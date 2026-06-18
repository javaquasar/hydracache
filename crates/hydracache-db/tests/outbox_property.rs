use proptest::prelude::*;

use hydracache_db::InvalidationIntent;

fn segment() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::new()),
        "[a-zA-Z0-9:%/ _\\-]{0,24}".prop_map(|value| value),
    ]
}

proptest! {
    #[test]
    fn escaping_never_collides(a in segment(), b in segment(), c in segment()) {
        let intents = [
            InvalidationIntent::key(a.clone()),
            InvalidationIntent::tag(a.clone()),
            InvalidationIntent::entity(a.clone(), b.clone()),
            InvalidationIntent::collection(a.clone()),
            InvalidationIntent::key(format!("{a}:{b}")),
            InvalidationIntent::tag(format!("{a}/{b}")),
            InvalidationIntent::entity(format!("{a}:{b}"), c.clone()),
            InvalidationIntent::entity(a.clone(), format!("{b}:{c}")),
            InvalidationIntent::collection(format!("{a}:{b}:{c}")),
            InvalidationIntent::flush(),
        ];

        for (left_index, left) in intents.iter().enumerate() {
            for right in intents.iter().skip(left_index + 1) {
                if left != right {
                    prop_assert_ne!(
                        left.target_hash(),
                        right.target_hash(),
                        "distinct intents should not collide: left={:?} right={:?}",
                        left,
                        right,
                    );
                }
            }
        }
    }
}
