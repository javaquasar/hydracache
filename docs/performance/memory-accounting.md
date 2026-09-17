# Memory accounting and repeatable measurements

HydraCache 0.71 distinguishes three different quantities. A logical owner is a live cache entry,
tag membership, request/session, event, or other retained application object. The W1 counters
record those owners and their conservative retained-byte estimates. Process resident set size
(RSS) is the operating system's resident pages for the process; it also includes allocator arenas,
code, stacks, and mapped pages. Container memory may additionally include file cache and kernel
slab. A logical counter returning to zero does not require RSS to return to the cold floor: a
bounded allocator high-water mark is possible. Conversely, a low RSS sample cannot prove an owner
was released.

| Field | Meaning | How to use it |
| --- | --- | --- |
| `logical.entries`, `logical.*_records`, `logical.*_bytes` | Application-owned counts and conservative retained bytes | Check exact reconciliation after delete, expiry, reset, close, and drain. |
| `process.vm_rss_bytes`, `process.smaps_rss_bytes` | Resident process pages | Compare like-for-like phases and independently started processes, never a lone peak. |
| `process.smaps_anon_bytes`, `process.smaps_file_bytes`, `process.smaps_pss_bytes` | Anonymous, file-backed, and proportional resident pages | Separate allocator/heap behavior from mappings and shared pages. |
| `cgroup` memory fields | Container-accounted memory, including relevant file/slab pages | Diagnose cgroup pressure; do not silently substitute for process RSS. |
| Phase timeline and provider probes | Timestamped workload/collector state | Reject missing windows, provider drift, mixed identities, or workload errors before statistics. |

Use the [frozen statistics contract](../testing/memory/0.71/memory-statistics-v1.toml) for
five independent paired starts, alternating B1/C order, warmup, cadence, settling, and the
predeclared slope/effect/regression budgets. The [S7 host profile](../testing/perf-host-profiles/memory-reference-071-v1.json)
and admission receipts pin the host, CPU/IRQ policy, kernel, instrumentation overhead, binary
identity, scenario digest, source and workflow SHA. Run only one long cell at a time. Reboot and
re-admit after host drift; do not splice measurements from a partial 6- or 24-hour cell into an
accepted result. Preserve partial output as diagnostic evidence and use a new immutable campaign
ID for a corrected attempt.

For each sample, collect cold, fill, steady, expire/delete, reset, refill, post-idle and shutdown
checkpoints with their exact command and timestamps. Check counts and retained bytes against the
expected cardinality at every phase. Inspect `smaps`, cgroup anon/file/slab, CPU, throughput,
latency, errors, thermal state, IRQ distribution, background daemons and disk activity together.
Do not discard a slow sample merely because it is inconvenient: the frozen rejection policy only
permits identity mismatch, telemetry gap, runner instability, or a non-zero workload error.

The accepted AX42 D4 chain is indexed [here](../testing/perf-artifacts/0.71/ax42/d4/README.md).
It used the exact measured commit `da8d6de409a657e0260e7fbfb4ab31d8d6ad5ca8`; M3, M8, M9
and both serialized M10 24-hour cells completed. The original archives are retained outside the
server; the public branch contains sanitized derivatives. To verify a materialized evidence
checkout, run `python verify.py` at its root. To recheck admission from the extracted GitHub
receipts, run:

```text
cargo run --manifest-path crates/xtask/Cargo.toml --locked -- memory-campaign-check --release 0.71 --campaigns <extracted-campaign-directory> --require-ship --expected-source-sha da8d6de409a657e0260e7fbfb4ab31d8d6ad5ca8
```

The extracted directory must contain one subdirectory per immutable campaign ID, with each
`campaign-receipt.json`, identity, state, admission, and compatibility receipt in their original
relative locations. A green campaign admission validates identity and completion; numerical
memory claims still require the separate S10 disposition and release-claims decision.
