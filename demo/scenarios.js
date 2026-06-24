export const SCENARIOS = [
  {
    name: "default",
    title: "Default seeded run",
    summary: "Use the seed and step controls directly.",
  },
  {
    name: "minority_partition_cannot_commit",
    title: "Minority partition cannot commit",
    summary: "A partitioned minority makes no workload progress while invariants hold.",
  },
  {
    name: "leader_crash_failover_no_committed_loss",
    title: "Leader crash, no committed loss",
    summary: "A node crash and restart keeps the deterministic history valid.",
  },
  {
    name: "symmetric_partition_heal_converges",
    title: "Symmetric partition heals",
    summary: "Partitioned links heal and the latest invariant verdict remains green.",
  },
  {
    name: "each_quorum_region_loss_fails_loud",
    title: "EachQuorum under region loss refuses progress",
    summary: "Region-loss posture is presented as halted progress, not silent success.",
  },
  {
    name: "delete_vs_concurrent_write_no_resurrection",
    title: "Delete versus concurrent write",
    summary: "Delete/write stress remains inside the real invariant checker.",
  },
];
