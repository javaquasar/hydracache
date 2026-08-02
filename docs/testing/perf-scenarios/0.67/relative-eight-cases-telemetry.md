# Relative eight-case telemetry campaign

This branch (explore/0.67-telemetry-hazelcast) prepares a separate
exploratory study. It is not release evidence, qualification evidence, or a
bootstrap sample, and it never writes target/test-evidence.

## Targets and workload

The existing eight cases, SET/GET operations, payloads, key range, requests,
pipeline depth, client counts, fixed affinity, repeats, and target order are
preserved. Every case includes all three targets: our HydraCache library/server,
Redis, and Hazelcast Community. Each operation runs in this fixed order:
HydraCache, Redis, Hazelcast Community. Hazelcast uses the official
hazelcast/hazelcast image; the runner
refuses an unpinned tag and requires
hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:<full-digest> through
HAZELCAST_IMAGE. Resolve and record the digest before execution. The Python
client must be pinned separately (default: hazelcast-python-client 5.5.0) and
is checked at startup; a missing or mismatched client aborts rather than
producing a partial comparison.

Hazelcast does not expose the RESP protocol, so its matched workload uses the
checked-in scripts/perf/hazelcast-workload.py against an IMap. It preserves
the same operation/key/payload/client/pipeline parameters but is reported as a
separate protocol path, not silently labeled as RESP.

## One-second telemetry

collect-target-telemetry.py samples every second and writes JSONL and CSV per
target/case/operation/repeat. It records container CPU%, process CPU ticks,
VmRSS/VmHWM, cgroup memory current/peak/limit, effective affinity, PID,
container inspect metadata, image ID/digest, and host receipt context. JVM
heap fields remain explicitly unavailable unless JVM_HEAP_CMD is configured to
return JSON with used_bytes, committed_bytes, and max_bytes; RSS is never used
as a heap substitute. telemetry-summary.json contains p50/p95/max for each
metric and its sample count.

The host preflight IRQ guard remains unchanged. After the containers and Hydra
are ready, the exploratory harness captures an IRQ baseline and the post-run
guard fails closed on any new IRQ activity or affinity mapping; this avoids
mistaking container-startup NVMe counters for activity during the measured
workload. This baseline/delta guard is exploratory-only and never changes the
qualification/bootstrap guard.

## Reproduction on the dedicated host

Install the pinned host benchmark dependency before running; the harness
records the installed `redis-benchmark --version` and refuses a missing
binary. A custom executable may be supplied with `REDIS_BENCHMARK`.

    sudo apt-get install -y redis-tools
    python3 -m pip install --user 'hazelcast-python-client==5.5.0'
    docker buildx imagetools inspect hazelcast/hazelcast:5.7.0-slim-jdk21
    export HAZELCAST_IMAGE='hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:<recorded-digest>'
    export MEASUREMENT_AFFINITY='5'
    scripts/perf/run-relative-eight-cases-telemetry.sh \
      /var/lib/hydracache-perf/exploratory/relative-eight-cases-telemetry-$(date -u +%Y%m%dT%H%M%SZ)

The output directory must be copied unchanged into a date-stamped exploratory
results directory together with the branch commit, image metadata, raw logs,
hardware-validation.txt, and telemetry-summary.json. Do not copy these files
into the qualification artifact tree.

At the end of the run the script also writes:

- report.md — a readable report containing the exact source/environment,
  validation receipt, telemetry summary, and artifact index;
- artifact-manifest.json — every raw file's byte length and SHA-256;
- reproduction-command.txt — the exact branch, commit, image, client, affinity,
  workload, and sampling parameters.

These files are the review entry point. The raw JSONL/CSV and benchmark logs
remain beside the report so another reader can audit every aggregate.
