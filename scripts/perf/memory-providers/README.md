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
An evidence campaign must keep both its output and temporary build trees
outside the checkout. `prepare` creates detached clean worktrees for the exact
frozen cohort SHAs, performs locked release builds, retains immutable binaries
and manifests, then removes the temporary sources and Cargo targets:

```text
python scripts/perf/memory_campaign_071.py --output-root /var/lib/hydracache-memory/campaigns plan --campaign-id d0-001 --case M0-cold --cohort B0-release --cohort B1-instrumented
python scripts/perf/memory_campaign_071.py --output-root /var/lib/hydracache-memory/campaigns prepare --campaign-id d0-001 --build-root /var/lib/hydracache-memory/build
```

Example:

```text
python scripts/perf/memory-providers/system.py probe --output probe.json
python scripts/perf/memory-providers/system.py start --state state.json --raw raw.jsonl --pid 1234 --binary ./hydracache-loadgen
python scripts/perf/memory-providers/system.py mark --state state.json --phase cold
python scripts/perf/memory-providers/system.py snapshot --state state.json --phase cold
```
