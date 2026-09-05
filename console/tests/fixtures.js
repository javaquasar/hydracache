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

export const capabilitiesEnvelope = {
  ...metadata,
  completeness: "complete",
  warnings: [],
  data: {
    capabilities: [
      { id: "cluster_formation", availability: "available", reason: null },
      { id: "consensus_progress", availability: "available", reason: null },
      { id: "persistence_recovery", availability: "available", reason: null },
    ],
  },
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

export const clientsEnvelope = {
  ...metadata,
  data: {
    authority_epoch: 42,
    observation_seq: 9,
    active_connections: null,
    accepted_total: null,
    closed_total: null,
    rejected_total: 3,
    pending_invocations: null,
    active_subscriptions: 2,
    active_sessions: null,
    buffered_bytes: null,
    reconnecting: null,
    slow: null,
    quota_rejected_total: null,
    cleanup_lag: null,
    detail_available: false,
    protocols: [
      { protocol: "hc1", version: "hc-1", active_connections: null, accepted_total: null, closed_total: null, rejected_total: 1, pending_invocations: null },
      { protocol: "hc2", version: "hc-2-alpha", active_connections: 1, accepted_total: 4, closed_total: 3, rejected_total: 2, pending_invocations: 0 },
      { protocol: "resp", version: "resp-2-3", active_connections: 0, accepted_total: 5, closed_total: 5, rejected_total: 0, pending_invocations: 0 },
    ],
  },
};

export const namespacesEnvelope = {
  ...metadata,
  data: {
    items: [
      {
        namespace: "orders",
        cache_count: 1,
        entries: 42,
        logical_bytes: 4096,
        retained_bytes: null,
        max_entries: 1000,
        max_bytes: 1048576,
        admitted_requests: 120,
        rate_limit_per_window: 1000,
        fair_share_count: 120,
        fair_share_per_window: 500,
        admission_rejected_total: 2,
        active_subscriptions: 1,
        near_cache_repairs_total: 0,
        persistence_status: "unavailable",
        usage_quality: "exact",
      },
    ],
    next_cursor: null,
    truncated: false,
  },
};

export const namespaceCachesEnvelope = {
  ...metadata,
  data: {
    items: [
      {
        namespace: "orders",
        cache: "client-surface",
        entries: 42,
        logical_bytes: 4096,
        retained_bytes: null,
        ttl_backlog: null,
        idempotency_records: null,
        backup_age_seconds: null,
      },
    ],
    next_cursor: null,
    truncated: false,
  },
};

export const healthEnvelope = {
  ...metadata,
  data: {
    aggregate: "FAIL",
    counts: { pass: 1, warn: 1, fail: 1, unknown: 15, disabled: 0 },
    thresholds: {
      raft_apply_lag_warn_entries: 100,
      raft_apply_lag_fail_entries: 1000,
      source: "reviewed_default",
      evaluation_version: 1,
    },
    checks: {
      items: [
        health("HC-AUDIT-001", "UNKNOWN", "audit", "Audit sink is available"),
        health("HC-AUTH-001", "UNKNOWN", "authority", "Authority quorum and leader are accessible"),
        health("HC-CLIENT-001", "UNKNOWN", "clients", "Client sessions and buffered bytes are within bounds"),
        health("HC-EXPIRY-001", "UNKNOWN", "expiry", "Expiry backlog is within its reviewed bound"),
        health("HC-FORM-001", "PASS", "formation", "Committed member formation is complete"),
        health("HC-HISTORY-001", "UNKNOWN", "history", "Optional history source is available"),
        health("HC-MEMBER-001", "UNKNOWN", "membership", "Member reachability, version and configuration agree"),
        health("HC-MEM-001", "UNKNOWN", "resource", "Retained memory and admission queue are within bounds"),
        health("HC-PART-001", "UNKNOWN", "partitions", "Partitions are assigned, replicated and zone-spread"),
        health("HC-PERSIST-001", "UNKNOWN", "persistence", "Persistence disk and backup freshness are healthy"),
        health("HC-PLACE-001", "UNKNOWN", "placement", "Placement constraints are satisfiable and applied"),
        health("HC-RAFT-001", "WARN", "consensus", "Raft apply progress is within bounds", [{ code: "apply-lag", value: 100, unit: "entries" }]),
        health("HC-REPAIR-001", "UNKNOWN", "repair", "Repair debt is clear and degraded mode is inactive"),
        health("HC-REPL-001", "UNKNOWN", "replication", "Replication has no failures or backpressure"),
        health("HC-RECOVERY-001", "FAIL", "recovery", "Durable recovery completed safely"),
        health("HC-RECOVERY-002", "UNKNOWN", "recovery", "No unaccounted corruption or data loss was verified"),
        health("HC-RECOVERY-003", "UNKNOWN", "recovery", "Recovery reconciliation is complete"),
        health("HC-RESHARD-001", "UNKNOWN", "reshard", "Reshard progress is not stalled"),
      ],
      next_cursor: null,
      truncated: false,
    },
  },
};

export const historyEnvelope = {
  ...metadata,
  source: "unavailable",
  data: {
    query_id: "cache_entries",
    state: "no_adapter",
    source: "prometheus_fixed_query",
    series: [],
    truncated: false,
  },
};

export const persistenceEnvelope = {
  ...metadata,
  source: "unavailable",
  data: {
    configured: true,
    enabled: true,
    destination_configured: true,
    storage_open: true,
    runtime_role: "member",
    backup_age_seconds: 45,
    backup_age_source: "runtime_observation",
    last_verified_backup_id: null,
    last_verified_backup_at_unix_ms: null,
    last_verified_restore_id: null,
    last_verified_restore_at_unix_ms: null,
    artifact_size_bytes: null,
    available_capacity_bytes: null,
    verification_state: "unavailable",
    recovery_state: "unknown",
    recovery_reason_code: "status-not-retained",
  },
};

export const operationsEnvelope = {
  ...metadata,
  data: {
    generation: 1700000000000,
    latest_sequence: 2,
    evicted_records: 0,
    retention_scope: "current_process_generation",
    items: {
      items: [
        {
          operation_id: "op-redacted-2",
          generation: 1700000000000,
          sequence: 2,
          kind: "backup",
          scope: "cluster",
          state: "accepted",
          requested_at_unix_ms: 1700000000000,
          accepted_at_unix_ms: 1700000000001,
          started_at_unix_ms: null,
          terminal_at_unix_ms: null,
          reason_code: null,
          source: "runtime_journal",
          completeness: "partial",
        },
        {
          operation_id: "op-redacted-1",
          generation: 1700000000000,
          sequence: 1,
          kind: "drain",
          scope: "node",
          state: "completed",
          requested_at_unix_ms: 1699999999000,
          accepted_at_unix_ms: 1699999999001,
          started_at_unix_ms: 1699999999002,
          terminal_at_unix_ms: 1699999999003,
          reason_code: null,
          source: "runtime_journal",
          completeness: "complete",
        },
      ],
      next_cursor: null,
      truncated: false,
    },
  },
};

export const auditEnvelope = {
  ...metadata,
  data: {
    generation: 1700000000000,
    latest_sequence: 2,
    evicted_records: 0,
    coverage: "management_operations_current_process_only",
    redaction: "no_keys_values_paths_credentials_or_actor_identity",
    items: {
      items: [
        {
          event_id: "op-audit-redacted",
          operation_id: "op-redacted-2",
          action: "backup",
          outcome: "accepted",
          occurred_at_unix_ms: 1700000000001,
          source: "runtime_journal",
        },
      ],
      next_cursor: null,
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

function health(id, status, category, title, evidence = [{ code: "required-input-missing", value: null, unit: null }]) {
  return { id, status, category, title, evidence, affected_count: null, remediation_code: `inspect-${category}`, remediation_link: "/docs/operations/management-healthchecks", source: "live", observation_seq: 9, evaluation_version: 1 };
}
