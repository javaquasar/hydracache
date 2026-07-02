export const liveOverviewFixture = {
  source: "live",
  members: [
    {
      node_id: "node-1",
      role: "member",
      reachable: true,
      reachability: "reachable",
      generation: 1,
    },
    {
      node_id: "node-2",
      role: "member",
      reachable: true,
      reachability: "reachable",
      generation: 2,
    },
    {
      node_id: "node-3",
      role: "member",
      reachable: false,
      reachability: "unreachable",
      generation: 3,
    },
  ],
  leader: {
    node_id: "node-2",
    term: 7,
    epoch: 42,
  },
  partitions: {
    under_replicated: 2,
    count: 64,
  },
  consistency: {
    configured_default: "quorum",
    op_counts_by_level: [{ level: "aggregate", count: 9 }],
  },
  backup_age_seconds: 123,
  lifecycle: {
    reshard_phase: "moving",
    upgrade_phase: "idle",
  },
};

export const modeledOverviewFixture = {
  source: "modeled",
  members: [],
  leader: null,
  partitions: {
    under_replicated: 0,
    count: 0,
  },
  consistency: {
    configured_default: null,
    op_counts_by_level: [],
  },
  backup_age_seconds: null,
  lifecycle: {
    reshard_phase: "idle",
    upgrade_phase: "idle",
  },
};

export const noLeaderFixture = {
  ...liveOverviewFixture,
  members: [
    {
      node_id: "solo-1",
      role: "member",
      reachable: true,
      reachability: "reachable",
      generation: 1,
    },
  ],
  leader: null,
  partitions: {
    under_replicated: 0,
    count: 16,
  },
};

export function largeOverviewFixture(count = 120) {
  return {
    ...liveOverviewFixture,
    members: Array.from({ length: count }, (_, index) => ({
      node_id: `node-${String(index + 1).padStart(3, "0")}`,
      role: "member",
      reachable: index % 17 !== 0,
      reachability: index % 17 === 0 ? "suspect" : "reachable",
      generation: index + 1,
    })),
    leader: {
      node_id: "node-001",
      term: 9,
      epoch: 44,
    },
    partitions: {
      under_replicated: 6,
      count: 512,
    },
  };
}

export const metricsFixture = `# HELP hydracache_cache_hit_ratio Cache hit ratio
# TYPE hydracache_cache_hit_ratio gauge
hydracache_cache_hit_ratio{cache="server"} 0.875
# HELP hydracache_admission_rejected_total Total rejected operations
# TYPE hydracache_admission_rejected_total counter
hydracache_admission_rejected_total 4
# HELP hydracache_admission_queue_depth Waiting FIFO backlog depth
# TYPE hydracache_admission_queue_depth gauge
hydracache_admission_queue_depth 2
# HELP hydracache_cluster_members Cluster members
# TYPE hydracache_cluster_members gauge
hydracache_cluster_members{source="live"} 3
`;
