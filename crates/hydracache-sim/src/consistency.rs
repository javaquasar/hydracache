//! Consistency-level satisfiability for the simulator topology.
//!
//! The product rules live in `hydracache::ConsistencyLevel` (the `grid`
//! module), which is not compiled for `wasm32`. The simulator must produce the
//! same snapshot natively and in the browser, so it uses this small mirror of
//! those rules for its own topology: every node is a replica, all nodes are in
//! one region (`SIM_REGION`), and only live nodes can acknowledge.
//! `tests/consistency_parity.rs` checks the mirror against
//! `hydracache::ConsistencyLevel::{required_acks, validate}` for every cluster
//! size and live set it can reach, including the reason text.

use crate::snapshot::{ConsistencyLevelView, SIM_REGION};

/// Product consistency levels in the order the demo lists them.
pub const SIM_CONSISTENCY_LEVELS: [&str; 5] =
    ["one", "local_quorum", "quorum", "each_quorum", "all"];

/// Build one view per level for `total_replicas` replicas of which
/// `live_replicas` can acknowledge.
pub fn consistency_level_views(
    total_replicas: usize,
    live_replicas: usize,
) -> Vec<ConsistencyLevelView> {
    SIM_CONSISTENCY_LEVELS
        .iter()
        .map(|level| level_view(level, total_replicas, live_replicas))
        .collect()
}

fn level_view(level: &str, total: usize, live: usize) -> ConsistencyLevelView {
    // Mirrors ConsistencyLevel::required_acks for a single region.
    let (required, regional) = match level {
        "one" => (1, false),
        "local_quorum" | "each_quorum" => (quorum_for(total), true),
        "quorum" => (quorum_for(total), false),
        "all" => (total.max(1), false),
        other => unreachable!("unknown simulator consistency level {other}"),
    };
    // Mirrors AckRequirement::is_satisfiable and unsatisfied_reason.
    let satisfiable = total > 0 && live >= required;
    let reason = if satisfiable {
        None
    } else if total == 0 {
        Some("no effective replicas are available".to_owned())
    } else if regional {
        Some(format!(
            "regional quorum unsatisfied: {SIM_REGION} needs {required}, has {live}"
        ))
    } else {
        Some(format!(
            "requires {required} live acknowledgements from {total} replicas, only {live} are live"
        ))
    };
    ConsistencyLevelView {
        level: level.to_owned(),
        total_replicas: total as u64,
        required_acks: required as u64,
        live_replicas: live as u64,
        satisfiable,
        reason,
    }
}

fn quorum_for(replica_count: usize) -> usize {
    replica_count / 2 + 1
}
