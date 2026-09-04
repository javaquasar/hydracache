const metadata = {
  schema_version: 1,
  observation_seq: 9,
  authority_epoch: 42,
  captured_at_unix_ms: 1_700_000_000_000,
  source: "live",
  completeness: "partial",
  stale_after_ms: 2_000,
  warnings: [{ code: "partial-observation", affected_count: 6 }],
};

export const dashboardEnvelope = {
  ...metadata,
  data: {
    cluster: {
      state: "active",
      quorum_ok: true,
      leader: "node-opaque-2",
      term: 7,
      authority_epoch: 42,
      metadata_authoritative: true,
      member_count: 3,
      voter_count: 3,
    },
    replication: {
      success_total: 120,
      failure_total: 2,
      backpressure_total: 1,
      under_replicated: 2,
      repair_debt: 4,
      degraded: false,
      zone_underspread: 1,
    },
    partitions: { total: 64, assigned: null, unassigned: null, distribution: [] },
    reshard: { phase: "moving", moves_inflight: 3, backfill_lag: 12 },
    cache: {
      entries: 64,
      retained_bytes: null,
      hits_total: 70,
      misses_total: 10,
      loads_total: 10,
      hit_ratio: 0.875,
      admission_queue_depth: 2,
      admission_rejected_total: 4,
      ttl_backlog: null,
    },
    consensus: { commit_index: 100, applied_index: 96, apply_lag: 4 },
    placement: {
      outcome: "committed",
      selected: 2,
      rejected: 4,
      latest_committed_epoch: 42,
      latest_applied_epoch: 41,
    },
    members: [
      member("node-opaque-1", "reachable", 1),
      member("node-opaque-2", "reachable", 2),
      member("node-opaque-3", "unreachable", 3),
    ],
    unavailable_fields: ["cache.retained_bytes", "cache.ttl_backlog", "members.cpu_percent"],
  },
};

export const modeledDashboardEnvelope = {
  ...dashboardEnvelope,
  source: "modeled",
  authority_epoch: null,
  warnings: [{ code: "authority-unavailable", affected_count: 1 }],
  data: {
    ...dashboardEnvelope.data,
    cluster: {
      ...dashboardEnvelope.data.cluster,
      state: "degraded",
      quorum_ok: false,
      leader: null,
      authority_epoch: 0,
      metadata_authoritative: false,
      member_count: 0,
      voter_count: 0,
    },
    consensus: { commit_index: null, applied_index: null, apply_lag: null },
    members: [],
  },
};

export function largeDashboardEnvelope(count = 120) {
  return {
    ...dashboardEnvelope,
    data: {
      ...dashboardEnvelope.data,
      cluster: { ...dashboardEnvelope.data.cluster, member_count: count },
      members: Array.from({ length: count }, (_, index) =>
        member(`node-${String(index + 1).padStart(3, "0")}`, index % 17 ? "reachable" : "suspect", index + 1),
      ),
    },
  };
}

export const formationEnvelope = {
  ...metadata,
  data: {
    items: [
      formation("node-opaque-1", "serving", null),
      formation("node-opaque-2", "blocked", "learner-behind"),
      ...[
        "cluster-identity-mismatch",
        "duplicate-node-identity",
        "generation-regression",
        "transport-unreachable",
        "peer-unauthenticated",
        "protocol-incompatible",
        "not-admitted",
        "quorum-unavailable",
        "authority-unknown",
        "draining",
        "source-unavailable",
      ].map((reason, index) => formation(`blocker-${index}`, "blocked", reason)),
    ],
    next_cursor: null,
    truncated: false,
  },
};

export const membersEnvelope = {
  ...metadata,
  data: {
    items: dashboardEnvelope.data.members.map((item, index) => ({
      ...item,
      consensus_role: index === 2 ? "learner" : "voter",
      client_count: index === 0 ? 4 : null,
      partition_count: index === 0 ? 32 : null,
      config_digest: index === 0 ? "sha256-v1:abc123" : null,
    })),
    next_cursor: null,
    truncated: false,
  },
};

export const partitionsEnvelope = {
  ...metadata,
  completeness: "complete",
  data: {
    authority_epoch: 42,
    observation_seq: 9,
    total: 64,
    assigned: 64,
    unassigned: 0,
    distribution: [
      { node: "node-opaque-1", primary: 32, backup: 32 },
      { node: "node-opaque-2", primary: 32, backup: 32 },
    ],
    under_replicated: 2,
    zone_underspread: 1,
    repair_debt: 4,
    reshard_phase: "moving",
    reshard_moves_inflight: 3,
    backfill_lag: 12,
    placement_trace_id: "trace_Opaque-42",
  },
};

export const placementTraceEnvelope = {
  ...metadata,
  completeness: "complete",
  data: {
    trace_id: "trace_Opaque-42",
    topology_epoch: 42,
    outcome: "committed",
    commit_index: 100,
    applied_index: 96,
    selected: ["node-opaque-2"],
    candidates: {
      items: [
        { node: "node-opaque-2", selected: true, reasons: [] },
        { node: "node-opaque-1", selected: false, reasons: ["zone-conflict"] },
      ],
      truncated: false,
    },
  },
};

export const consensusEnvelope = {
  ...metadata,
  data: {
    items: [
      consensus("node-opaque-1", 100, 100),
      consensus("node-opaque-2", 100, 96),
      consensus("node-opaque-3", null, null),
    ],
    next_cursor: null,
    truncated: false,
  },
};

export const recoveryEnvelope = {
  ...metadata,
  data: {
    items: ["clean", "repaired", "partial", "corrupt", "failed"].map((outcome, index) => ({
      schema_version: 1,
      scope: "node",
      outcome,
      phase: outcome === "failed" ? "failed" : "completed",
      artifact: "durable_value_store",
      records_checked: { value: 100, exact: true },
      records_recovered: { value: outcome === "repaired" ? 3 : 0, exact: true },
      records_discarded: { value: outcome === "partial" ? 2 : 0, exact: true },
      corrupt_records: {
        value: outcome === "corrupt" ? 1_000_000 : 0,
        exact: outcome !== "corrupt",
      },
      repair: outcome === "repaired" ? "completed" : "not_required",
      reason: outcome === "failed" ? "io-failure" : null,
      source: "live",
      completeness: index > 1 ? "partial" : "complete",
    })),
    next_cursor: null,
    truncated: false,
  },
};

function member(node, reachability, generation) {
  return { node, generation, role: "member", reachability, cpu_percent: null, rss_bytes: null, retained_bytes: null, uptime_seconds: null };
}

function formation(node, serving, blocker) {
  return {
    node,
    generation: 1,
    transport: blocker === "transport-unreachable" ? "unreachable" : "authenticated",
    admission: blocker === "not-admitted" ? "pending" : "admitted",
    consensus_role: "voter",
    catch_up: blocker === "learner-behind" ? "behind" : "current",
    serving,
    blocker,
  };
}

function consensus(node, commit_index, applied_index) {
  return { node, generation: 1, commit_index, applied_index, catch_up_target: commit_index };
}
