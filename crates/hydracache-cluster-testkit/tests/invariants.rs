use std::collections::{BTreeMap, BTreeSet};

use hydracache_cluster_testkit::invariants::{cluster_invariant_violations, ClusterInvariantView};

#[test]
fn invariant_catalog_flags_each_seeded_violation() {
    let cases = [
        (
            "two leaders",
            ClusterInvariantView {
                leaders_by_term: BTreeMap::from([(7, vec![1, 2])]),
                voter_sets_by_node: BTreeMap::from([
                    (1, BTreeSet::from([1, 2, 3])),
                    (2, BTreeSet::from([1, 2, 3])),
                ]),
                member_sets_by_node: BTreeMap::from([
                    (1, BTreeSet::from(["member-a".to_owned()])),
                    (2, BTreeSet::from(["member-a".to_owned()])),
                ]),
                committed_command_ids: BTreeSet::from(["member-upsert:member-a:1".to_owned()]),
                applied_command_ids_by_node: BTreeMap::from([
                    (1, BTreeSet::from(["member-upsert:member-a:1".to_owned()])),
                    (2, BTreeSet::from(["member-upsert:member-a:1".to_owned()])),
                ]),
            },
            "multiple leaders",
        ),
        (
            "divergent voters",
            ClusterInvariantView {
                leaders_by_term: BTreeMap::from([(7, vec![1])]),
                voter_sets_by_node: BTreeMap::from([
                    (1, BTreeSet::from([1, 2, 3])),
                    (2, BTreeSet::from([1, 2])),
                ]),
                member_sets_by_node: BTreeMap::from([
                    (1, BTreeSet::from(["member-a".to_owned()])),
                    (2, BTreeSet::from(["member-a".to_owned()])),
                ]),
                committed_command_ids: BTreeSet::from(["member-upsert:member-a:1".to_owned()]),
                applied_command_ids_by_node: BTreeMap::from([
                    (1, BTreeSet::from(["member-upsert:member-a:1".to_owned()])),
                    (2, BTreeSet::from(["member-upsert:member-a:1".to_owned()])),
                ]),
            },
            "voter set diverged",
        ),
        (
            "lost committed entry",
            lost_committed_entry_view(),
            "lost committed command",
        ),
        (
            "divergent members",
            ClusterInvariantView {
                leaders_by_term: BTreeMap::from([(7, vec![1])]),
                voter_sets_by_node: BTreeMap::from([
                    (1, BTreeSet::from([1, 2, 3])),
                    (2, BTreeSet::from([1, 2, 3])),
                ]),
                member_sets_by_node: BTreeMap::from([
                    (
                        1,
                        BTreeSet::from(["member-a".to_owned(), "member-b".to_owned()]),
                    ),
                    (2, BTreeSet::from(["member-a".to_owned()])),
                ]),
                committed_command_ids: BTreeSet::from(["member-upsert:member-a:1".to_owned()]),
                applied_command_ids_by_node: BTreeMap::from([
                    (1, BTreeSet::from(["member-upsert:member-a:1".to_owned()])),
                    (2, BTreeSet::from(["member-upsert:member-a:1".to_owned()])),
                ]),
            },
            "member set diverged",
        ),
    ];

    for (name, view, expected) in cases {
        let violations = cluster_invariant_violations(&view);
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains(expected)),
            "{name} did not produce expected invariant violation {expected:?}: {violations:?}"
        );
    }
}

#[test]
fn canary_invariant_catalog_misses_a_lost_committed_entry() {
    let violations = cluster_invariant_violations(&lost_committed_entry_view());
    assert!(
        violations
            .iter()
            .any(|violation| violation.contains("lost committed command")),
        "canary models a broken invariant catalog that ignores lost committed entries"
    );
}

fn lost_committed_entry_view() -> ClusterInvariantView {
    ClusterInvariantView {
        leaders_by_term: BTreeMap::from([(7, vec![1])]),
        voter_sets_by_node: BTreeMap::from([
            (1, BTreeSet::from([1, 2, 3])),
            (2, BTreeSet::from([1, 2, 3])),
        ]),
        member_sets_by_node: BTreeMap::from([
            (1, BTreeSet::from(["member-a".to_owned()])),
            (2, BTreeSet::from(["member-a".to_owned()])),
        ]),
        committed_command_ids: BTreeSet::from([
            "member-upsert:member-a:1".to_owned(),
            "member-upsert:member-b:1".to_owned(),
        ]),
        applied_command_ids_by_node: BTreeMap::from([
            (
                1,
                BTreeSet::from([
                    "member-upsert:member-a:1".to_owned(),
                    "member-upsert:member-b:1".to_owned(),
                ]),
            ),
            (2, BTreeSet::from(["member-upsert:member-a:1".to_owned()])),
        ]),
    }
}
