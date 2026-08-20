# Memory provider protocol

Each adapter implements `probe`, `start`, `mark`, `snapshot`, `stop`, and
`normalize`. Provider state and samples contain only aggregate byte counts,
fixed phase names, and sanitized folded stack names. A run is invalid unless
all eight phases occur exactly once and in canonical order.

The 0.71 campaign controller expands the finite M0-M10 matrix, applies row
time caps, journals every attempt, and resumes without repeating successful
jobs. Run a bounded orchestration rehearsal before allocating a reference
host:

```text
python scripts/perf/memory_campaign_071.py doctor
python scripts/perf/memory_campaign_071.py plan --campaign-id local-rehearsal --case M0-cold --repetitions 1 --rehearsal
python scripts/perf/memory_campaign_071.py run --campaign-id local-rehearsal
python scripts/perf/memory_campaign_071.py resume --campaign-id local-rehearsal
python scripts/perf/memory_campaign_071.py finalize --campaign-id local-rehearsal
```

Rehearsal receipts are always non-promotable. Evidence execution remains
fail-closed until the dedicated daemon executor and admitted host are present.

For a closer pre-rental check, run the controller plus real M3 TTL and M5
high-fanout daemon cells in a cgroup-v2 Linux container. The output directory must be new and empty; the
named volumes make later rehearsals reuse the Cargo download and build cache:

```powershell
docker build -f scripts/perf/Dockerfile.memory-rehearsal-071 -t hydracache-memory-rehearsal:071 .
$evidence = (New-Item -ItemType Directory target/docker-memory-rehearsal-071).FullName
docker volume create hydracache-memory-rehearsal-target-071
docker volume create hydracache-memory-rehearsal-cargo-071
docker run --rm --init --memory 4g --cpus 4 `
  --mount "type=bind,source=$evidence,target=/evidence" `
  --mount type=volume,source=hydracache-memory-rehearsal-target-071,target=/workspace/target `
  --mount type=volume,source=hydracache-memory-rehearsal-cargo-071,target=/usr/local/cargo/registry `
  hydracache-memory-rehearsal:071
```

The container writes `docker-rehearsal-receipt.json`, a typed real-daemon
report, and the finalized 44-job matrix. It uses a synthetic Git snapshot, so
the receipt is always diagnostic and cannot satisfy the frozen B0/B1 evidence
gate.

An evidence campaign must keep both its output and temporary build trees
outside the checkout. `prepare` creates detached clean worktrees for the exact
frozen cohort SHAs, performs locked release builds, retains immutable binaries
and manifests, then removes the temporary sources and Cargo targets:

```text
python scripts/perf/memory_campaign_071.py --output-root /var/lib/hydracache-memory/campaigns plan --campaign-id d0-001 --case M0-cold --cohort B0-release --cohort B1-instrumented
python scripts/perf/memory_campaign_071.py --output-root /var/lib/hydracache-memory/campaigns prepare --campaign-id d0-001 --build-root /var/lib/hydracache-memory/build
```

Each admitted evidence job starts a fresh retained daemon binary, drives its
RESP-compatible memory phases, invokes the selected provider at every phase,
captures `/proc` and cgroup-v2 values, stops the daemon, and validates the
result with `cargo xtask memory-baseline-report-check`. Cells that still need a
specialized HC/2, persistence, or long-sequence executor are
rejected before the job begins; they can never fall back to a misleading RESP
approximation.

The protected workflow `.github/workflows/memory-reference-071.yml` imports
the admitted host, 0.67.1 activation, S5 overhead, and historical-mirror
receipts before building anything. It serializes a campaign through the
`memory-reference-071` environment, rechecks the live host fingerprint on a
rerun, wraps execution in the repository watchdog, and publishes every
completed job atomically to the configured mirror mount before advancing.

Create the historical input receipt before renting the measurement host. The
command archives only the frozen historical `results` and scenario documents,
writes to an explicitly approved protected mount, restores the object into a
temporary directory, and refuses the receipt unless every byte matches:

```text
python scripts/perf/memory_historical_mirror_071.py \
  --mirror-root /mnt/hydracache-protected-memory/history \
  --provider encrypted-object-mount \
  --retention-deadline 2027-08-20T00:00:00Z \
  --output /var/lib/hydracache-memory/admission/historical-input-receipt.json \
  --approve-protected-mirror
```

Example:

```text
python scripts/perf/memory-providers/system.py probe --output probe.json
python scripts/perf/memory-providers/system.py start --state state.json --raw raw.jsonl --pid 1234 --binary ./hydracache-loadgen
python scripts/perf/memory-providers/system.py mark --state state.json --phase cold
python scripts/perf/memory-providers/system.py snapshot --state state.json --phase cold
```
