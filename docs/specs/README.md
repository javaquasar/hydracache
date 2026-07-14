# HydraCache Raft Election And Recovery Model

`RaftElection.tla` is an independent bounded protocol model. It covers
pre-vote/vote messages, delayed/dropped/duplicated messages, elections,
unavailability and restart, committed log prefixes, apply, snapshot install,
membership epochs, and stale authority after removal. It is deliberately
smaller than the Rust implementation model in W23: agreement between the two
is evidence from independent descriptions, not shared code.

The fast config uses three nodes and small term/log/message bounds. The nightly
config adds a fourth node, wider bounds, and the post-fault liveness property.
`RaftElectionCanary.tla` is a separate negative model that permits two leaders
in one term. The main module does not extend or import it.

## Traceability

| TLA invariant | Invariant catalog id | Rust implementation evidence |
| --- | --- | --- |
| `AtMostOneLeaderPerTerm` | `HC-RAFT-INV-ONE-LEADER-PER-TERM` | `crates/hydracache-cluster-testkit/tests/invariants.rs::invariant_catalog_flags_each_seeded_violation` |
| `TermsNeverDecrease` | `HC-RAFT-INV-TERM-MONOTONIC` | `crates/hydracache-cluster-raft/tests/raft_corpus_vectors.rs::raft_corpus_stale_term_install_snapshot_is_rejected` |
| `CommittedIndexNeverDecreases` | `HC-RAFT-INV-COMMIT-MONOTONIC` | `crates/hydracache-cluster-raft/tests/leadership_handoff.rs::leadership_handoff_preserves_committed_prefix_and_exactly_once_proposal_outcome` |
| `CommittedPrefixNeverConflicts` | `HC-RAFT-INV-COMMITTED-PREFIX` | `crates/hydracache-cluster-raft/tests/raft_message_filter.rs::reordered_appends_do_not_corrupt_committed_prefix` |
| `AppliedNeverExceedsCommit` | `HC-RAFT-INV-APPLIED-LE-COMMIT` | `crates/hydracache-cluster-raft/tests/model_check.rs::bounded_model_check_membership_and_commit_invariants_hold_for_up_to_4_nodes` |
| `SnapshotIdentityMatches` | `HC-RAFT-INV-SNAPSHOT-IDENTITY` | `crates/hydracache-cluster-raft/tests/snapshot_corruption.rs::misdirected_snapshot_with_valid_checksum_is_rejected_on_identity_mismatch` |
| `SnapshotIndexNeverDecreases` | `HC-RAFT-INV-SNAPSHOT-MONOTONIC` | `crates/hydracache-cluster-raft/tests/snapshot_delivery_chaos.rs::newer_snapshot_then_delayed_older_snapshot_never_rolls_state_back` |
| `RemovedNodeCannotRegainAuthority` | `HC-RAFT-INV-REMOVED-NODE-NO-AUTHORITY` | `crates/hydracache-cluster-raft/tests/raft_message_filter.rs::leader_promotion_does_not_resurrect_draining_member` |

## Toolchain

`tla-toolchain.toml` pins the stable TLA+ Tools release, download URL, SHA-256,
and minimum Java version. Changing any pin or a model bound is a reviewed proof
surface change. Structural validation runs without Java. TLC execution writes
commit-bound evidence through the gated-test registry; a local missing Java or
jar is reported as an explicit skip, while CI sets `HYDRACACHE_REQUIRE_TLC=1`
and must fail rather than skip.
