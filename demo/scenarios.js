// Curated scenarios. `baseStep` is the step a preset world is already at when
// loaded (the engine's preset `steps`; checked by demo_scenarios_match.rs), so
// the timeline cannot rewind into the preset's own setup.
//
// Each tour step has `text` and optional `actions` the Guide panel can run.
// Action targets may be "$leader" (the current leader) or "$crashed" (the node
// the tour crashed last). Tour text describes what to look at; it never claims
// an outcome the engine has not produced.
export const SCENARIOS = [
  {
    name: "default",
    title: "Default seeded run",
    summary: "Use the seed and step controls directly.",
    baseStep: 0,
    tour: [
      {
        text: "A fresh three-node cluster starts without a leader. Run until one is elected and watch the vote and heartbeat signals.",
        actions: [{ type: "until", condition: "leader" }],
      },
      {
        text: "Subscribe client-a to the profiles namespace. Subscribers receive committed cache events.",
        actions: [{ type: "subscribe", client: "client-a", ns: "profiles" }],
      },
      {
        text: "Push profile-42. The write commits through the leader, replicates to every node, and is delivered to the subscriber. Compare the checksums in the Keys panel.",
        actions: [{ type: "push", client: "client-a", ns: "profiles", key: "profile-42", value: "fresh" }],
      },
      {
        text: "Crash the current leader. Two of three nodes are still a quorum.",
        actions: [{ type: "crash", node: "$leader" }],
      },
      {
        text: "Run until a leader exists again. The timeline records the crash and the observed leader change; the verdict panel shows what was checked.",
        actions: [{ type: "until", condition: "leader" }],
      },
      {
        text: "Restart the crashed node and run until the replicas agree on every sampled key.",
        actions: [
          { type: "restart", node: "$crashed" },
          { type: "until", condition: "agree" },
        ],
      },
    ],
  },
  {
    name: "minority_partition_cannot_commit",
    title: "Minority partition cannot commit",
    summary: "A partitioned minority makes no workload progress while invariants hold.",
    baseStep: 6,
    tour: [
      {
        text: "node-0's links to node-1 and node-2 are partitioned and the workload is off. Nothing has committed, and the verdict holds: refusing to commit is the correct outcome.",
      },
      {
        text: "Turn the workload on and run 10 steps. Watch the Committed counter and the verdict.",
        actions: [
          { type: "workload", enabled: true },
          { type: "run", steps: 10 },
        ],
      },
      {
        text: "Heal both partitioned links and run until the replicas agree.",
        actions: [
          { type: "heal", from: "node-0", to: "node-1" },
          { type: "heal", from: "node-0", to: "node-2" },
          { type: "until", condition: "agree" },
        ],
      },
    ],
  },
  {
    name: "leader_crash_failover_no_committed_loss",
    title: "Leader crash, no committed loss",
    summary: "A node crash and restart keeps the deterministic history valid.",
    baseStep: 12,
    tour: [
      {
        text: "This run crashed node-0 at step 4 and restarted it at step 8. It is now at step 12 with the verdict holding.",
      },
      {
        text: "Run the cluster on for 8 steps, then use the timeline to rewind: every earlier step is rebuilt exactly from the seed.",
        actions: [{ type: "run", steps: 8 }],
      },
      {
        text: "Try it live: crash the current leader and run until a new one is elected.",
        actions: [
          { type: "crash", node: "$leader" },
          { type: "until", condition: "leader" },
        ],
      },
    ],
  },
  {
    name: "symmetric_partition_heal_converges",
    title: "Symmetric partition heals",
    summary: "Partitioned links heal and the latest invariant verdict remains green.",
    baseStep: 10,
    tour: [
      {
        text: "node-0 and node-1 were cut off from each other in both directions for four steps, then healed. Check the Keys panel for per-node checksums.",
      },
      {
        text: "Partition node-0 -> node-2 yourself and run 6 steps, then look for diverged checksums.",
        actions: [
          { type: "partition", from: "node-0", to: "node-2" },
          { type: "run", steps: 6 },
        ],
      },
      {
        text: "Heal it and run until the replicas agree again.",
        actions: [
          { type: "heal", from: "node-0", to: "node-2" },
          { type: "until", condition: "agree" },
        ],
      },
    ],
  },
  {
    name: "each_quorum_region_loss_fails_loud",
    title: "EachQuorum under region loss refuses progress",
    summary: "Region-loss posture is presented as halted progress, not silent success.",
    baseStep: 5,
    tour: [
      {
        text: "Two of three nodes are crashed. node-0 alone is not a quorum, so it cannot become leader or commit: the lab shows halted progress, not silent success.",
      },
      {
        text: "Restart node-1. Two live nodes form a quorum again; run until a leader is elected.",
        actions: [
          { type: "restart", node: "node-1" },
          { type: "until", condition: "leader" },
        ],
      },
    ],
  },
  {
    name: "delete_vs_concurrent_write_no_resurrection",
    title: "Delete versus concurrent write",
    summary: "Delete/write stress remains inside the real invariant checker.",
    baseStep: 16,
    tour: [
      {
        text: "The workload ran while one message from node-0 to node-1 was delayed and one from node-1 to node-0 was dropped. The Verdict panel lists exactly which invariants the live checker evaluated.",
      },
      {
        text: "Run until the replicas agree, then compare checksums in the Keys panel.",
        actions: [{ type: "until", condition: "agree" }],
      },
    ],
  },
];
