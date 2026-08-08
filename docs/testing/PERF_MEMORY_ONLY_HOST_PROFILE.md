# Ubuntu 24.04 memory-only performance profile

`ubuntu-24.04-memory-only-v1` is an additional, explicit host profile. It does
not replace `ubuntu-24.04-reference-v1`, which remains the default strict
full-I/O-isolation profile for 0.67.1 qualification and bootstrap evidence.

The memory-only profile is diagnostic and non-ship. Its receipts cannot be used
as qualification, bootstrap, or release evidence. Results from the two profiles
must not be pooled.

## Contract

All builds, downloads, container extraction, and evidence materialization finish
before the measurement window. The measured executable, working directory, and
output directory reside below one `tmpfs` runtime root. Swap is disabled. The
window passes only when all of the following remain zero:

- global NVMe namespace counter deltas;
- current cgroup-v2 `io.stat` counter deltas;
- NVMe IRQ deltas on every measurement CPU;
- major page faults for the measured process and its waited-for children.

The launcher pins the child process to the profile's measurement CPU set and
preserves raw before/after snapshots plus an immutable JSON receipt. A command
failure also rejects the window.

## Invocation

Prepare and warm the exact executable in `tmpfs` before invoking the guard. The
output must not exist and must also be below the same runtime root:

```bash
sudo scripts/perf/reference-memory-only-window.py \
  --profile docs/testing/perf-host-profiles/ubuntu-24.04-memory-only-v1.json \
  --runtime-root /dev/shm/hydracache-memory-only-v1 \
  --working-directory /dev/shm/hydracache-memory-only-v1/run \
  --output-dir /dev/shm/hydracache-memory-only-v1/results/window-1 \
  -- /dev/shm/hydracache-memory-only-v1/bin/hydracache-loadgen ARGS...
```

Only after the guard exits may housekeeping CPUs copy the immutable result
directory from `tmpfs` to durable evidence storage. A missing counter source,
non-`tmpfs` runtime, enabled swap, changed device/IRQ mapping, or unknown profile
fails closed.

For the automated real-binary smoke sequence, build the release loadgen and
server first, then run:

```bash
taskset --cpu-list 0,5-7 scripts/perf/run-memory-only-measurement.sh \
  --run-id rental-2026-xx-xx-a --mode all
```

The orchestrator stages both exact binaries, performs `local` and
`client-surface` warm-ups, runs each measured window through the guard, creates
`memory-only-run.json`, and atomically materializes immutable results under
`target/test-evidence/0.67.1/memory-only/<run-id>`. Failed windows remain in
`tmpfs` for diagnosis and are never materialized as successful runs.

## Local Docker scope

Docker tests validate parsing, path containment, immutable receipt generation,
zero-delta acceptance, and injected disk/IRQ rejection with synthetic `/proc`
and cgroup fixtures. They make no bare-metal performance or IRQ-placement claim.
