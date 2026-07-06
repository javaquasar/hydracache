# HydraCache Soak Runbook

HydraCache soak evidence is binary and reproducible: a run is clean or it stops on
the first invariant violation with the seed needed to reproduce it. The report has
no health percentage, score, throughput number, or ops/sec claim.

## Fast Gate

The PR-safe gate is short and deterministic:

```powershell
cargo test -p hydracache-sim --test soak_budget --locked
```

`cargo xtask verify` and CI run this as `soak fast budget`. It uses a fixed master
seed and a small seed cap, so repeated runs should produce the same clean outcome.

## Manual And Nightly Soak

Run the continuous simulator soak and save the score-free report:

```powershell
cargo run -p hydracache-sim --bin vopr --locked -- soak --master-seed 22530 --budget-secs 60 --steps-per-seed 512 --max-seeds 128 > SOAK_REPORT.json
```

Nightly CI also uploads `SOAK_REPORT.json` as an artifact and runs the ignored
resource/operator sentinels:

```powershell
cargo test -p hydracache-server --test soak_resource --locked -- --ignored --nocapture
cargo test -p hydracache-operator --test soak_kind --locked -- --ignored --nocapture
```

The operator kind soak is opt-in. Without `HYDRACACHE_OPERATOR_KIND=1`, it skips
gracefully. Since 0.59, member-role pods host the networked daemon grid; the
loopback daemon E2E is gated separately by `HYDRACACHE_RUN_NETWORKED_DAEMON_E2E`.
Partition and slow-disk kind faults still require the external chaos injector.

## SOAK_REPORT

The report shape is:

```json
{
  "master_seed": 22530,
  "seeds_run": 128,
  "total_steps": 65536,
  "wall_clock_secs": 60,
  "resource_bounds_ok": true,
  "outcome": { "status": "clean" }
}
```

On failure, `outcome` is:

```json
{
  "status": "failed",
  "seed": 123,
  "step": 456,
  "reproduce": "vopr --seed 123 --steps 456",
  "minimization": { "kind": "steps", "minimal_steps": 321 },
  "violations": ["invariant_name: detail"]
}
```

`minimization.kind` is `steps` for plain-seed step bisection, `schedule` for a
shrunk scheduled-fault failure, or `not_run` when a debug hook or non-simulator
failure did not go through a minimizer.

## Triage

1. Reproduce exactly with the `reproduce` command from the report.
2. If `minimization.kind` is `steps`, rerun with `minimal_steps`.
3. If `minimization.kind` is `schedule`, keep the shrunk schedule attached to the
   bug or technical-debt item.
4. Treat resource-bound violations like correctness violations: they stop the soak
   and require either a fix or an explicitly scoped debt item.
