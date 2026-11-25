# How We Test Correctness

A cache that is wrong is worse than no cache: it serves stale or deleted data with full confidence. HydraCache therefore treats correctness evidence as part of the product, not as an internal detail. This page summarizes how that evidence is produced and where to find it.

## Principles

- **Boolean gates, no scores.** A release gate passes or fails. There is no "92% healthy" number to argue about.
- **Fail loud.** A violated invariant stops the run and prints the seed that reproduces it.
- **Determinism first.** Fault injection is seeded; every chaos and simulation run can be replayed exactly. Correctness assertions use logical signals (epoch, version, commit index), never wall-clock thresholds.
- **Every new code path is tested.** Coverage is ratcheted with a workspace floor; it can go up, not down.

## Deterministic Simulation

`hydracache-sim` drives a cluster through a simulated network, clock, and storage with a seeded RNG. In one process it can partition nodes, crash and restart them, delay, drop, or duplicate messages, and inject storage faults — then check the resulting history.

Invariant checkers run on every step, including:

| Invariant | What it rules out |
| --- | --- |
| `election_safety` | two leaders in the same term |
| `leader_requires_quorum` | a leader without a quorum of votes |
| `no_stale_leader_writes` | writes accepted by a deposed leader |
| `index_monotonicity` | commit or applied indexes moving backwards |
| `durability` | acknowledged writes lost after a crash |
| `convergence` | replicas that never agree after faults heal |
| `catchup_no_skip` | a lagging replica skipping entries while catching up |
| `event_after_commit` | change events emitted before the write is committed |
| `resource_bounded_growth` | memory, queues, or storage growing without bound |

A separate **linearizability oracle** checks recorded client histories for single-key operations that claim linearizability.

You can watch the same engine and checkers in the [interactive simulator](https://javaquasar.github.io/hydracache/demo/): every verdict on that page comes from the real checker, and every run can be shared and replayed by seed.

## Formal Models

The raft election and recovery protocol has an independent bounded **TLA+ model** (`docs/specs/RaftElection.tla`). It covers pre-vote and vote messages; delayed, dropped, and duplicated messages; restarts; committed log prefixes; snapshot install; membership epochs; and stale authority after removal.

Each TLA+ invariant is traced to a Rust test that exercises the same property in the implementation — for example, *at most one leader per term*, *committed index never decreases*, and *a removed node cannot regain authority*. A deliberately broken canary model proves that the model checker actually catches a violation.

## Endurance and Soak

Soak runs drive the simulator across many seeds for a time budget and produce a score-free `SOAK_REPORT.json`: the run is either clean, or it stops on the first violation with the seed to reproduce it. A short, fixed-seed soak runs on every pull request; longer runs and resource sentinels for the server and operator run nightly.

## Real Processes and Real Clients

Simulation is complemented by tests against real components:

- networked raft and peer-fetch transports between real processes;
- Redis and NATS invalidation transports against containers;
- the Redis RESP facade against mainstream Redis clients in several languages and a pinned real Redis as an oracle;
- SQLx against SQLite and Postgres;
- multi-hour stability runs of release candidates.

## Release Gates

One command runs the fast gates locally, and CI runs the same set on every pull request:

```powershell
cargo xtask verify
```

It covers formatting, clippy, the dependency policy, documentation consistency, workspace tests, rustdoc, the deterministic-simulation and soak fast budgets, raft failpoint crash safety, and the performance-budget contract. Time-heavy suites — long chaos and soak runs, benchmarks, container matrices — are scheduled separately and are part of release evidence.

## Where to Look

- `docs/RULES.md` — the invariants every change must respect.
- `docs/GATES.md` — every gate, its command, and what it guards.
- `docs/TESTING.md` — testing conventions and test tiers.
- `docs/specs/` — TLA+ models and traceability to Rust tests.
- `docs/soak.md` — how to run and read soak reports.

See also: [Consistency Model](consistency-model.md) for the guarantees these checks protect.
