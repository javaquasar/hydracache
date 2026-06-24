# HydraCache Release Plan Index & Roadmap

Human-readable mirror of `docs/plans/releases.toml` (the machine-readable
authoritative manifest, validated by `cargo xtask doc-check`). When the two disagree,
`releases.toml` wins — update both together.

This file answers three questions for every release: **what** it delivers, **why**
(the problem it solves), and **after what** it can be done (dependencies) — plus what
it **unblocks**. Each plan also carries the same summary in an "At a glance" block at
its top. All plans share the invariants in [`../RULES.md`](../RULES.md) and the gate
discipline in [`../GATES.md`](../GATES.md); they do not redefine those rules.

## How to read this roadmap

- **Two tracks.** `0.37`–`0.38` are the **database** track (query-result caching
  correctness). `0.39`→`0.47` are the **cluster/distributed** track, with `0.44` a
  **foundation** release (deterministic simulation testing) inserted before the
  remaining features so they are developed against the simulator. The cluster track is
  strictly sequential: each release hardens or builds on the previous one.
- **"After what."** A release should not be started until its `depends_on` release is
  done. The dependency DAG below is the source of order.
- **Status honesty (RULES R-7/R-11).** `shipped` means the release's gates passed.
  The `0.43` debt-closure gates now validate the `0.42`/`0.43` multi-node and
  multi-zone claims over a real networked transport; future claim changes must stay
  tied to explicit release gates.

## Dependency DAG (what comes after what)

```
v0 foundations
      │
      ▼
0.37 DB production hardening ──► 0.38 DB correctness automation
                                        │
                                        ▼
                              0.39 cluster staging hardening
                                        │
                                        ▼
                              0.40 internal production pilot
                                        │
                                        ▼
                              0.41 distributed-grid roadmap + first slice
                                        │
                                        ▼
                              0.42 production grid hardening ┄┄► (debt) V0_43_DEBT_CLOSURE_AND_REFACTOR
                                        │                          (make 0.42/0.43 multi-node REAL,
                                        ▼                           absorbs V0_43_CONTINUATION_…)
                              0.43 geo-distribution & elasticity
                                        │
                                        ▼
                              0.44 deterministic simulation testing (DST)  ◄ foundation
                                        │
                                        ▼
                              0.45 active-active multi-region
                                        │
  