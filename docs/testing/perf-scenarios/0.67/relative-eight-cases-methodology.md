# Relative eight-case RESP campaign

This is an exploratory study for optimization work. It is intentionally separate
from release qualification, W3/W7 receipts, bootstrap samples, and ship-eligible
evidence. The campaign never overwrites `target/test-evidence`.

## Fixed methodology

- One dedicated `reference-v1` bare-metal host; HydraCache and Redis share the
  same host and loopback TCP boundary.
- HydraCache is pinned to measurement CPUs `1-4`; orchestration runs on
  housekeeping CPUs. Redis is requested with Docker `--cpuset-cpus 1-4` and the
  request is recorded, but the host's cgroup setup may reject enforcement. The
  report therefore records any cpuset warning and this campaign must not be
  treated as a strict affinity-controlled comparison unless that field is
  `none`.
- One pinned Redis OCI image (`redis@sha256:3aaec...6d7e`) and one selected
  `redis-benchmark` binary are used for every case.
- Every case runs SET and GET with the same value size, key range (10,000),
  clients, request count (100,000 per operation), and pipeline depth.
- Three repeats, with each case recording raw stdout/stderr and preserving the
  fixed Hydra-then-Redis execution order. No stability or superiority claim is
  inferred from a single number.

## Eight cases

| ID | Payload | Clients | Pipeline |
|---|---:|---:|---:|
| `p64-c10-p1` | 64 B | 10 | 1 |
| `p64-c10-p10` | 64 B | 10 | 10 |
| `p256-c10-p1` | 256 B | 10 | 1 |
| `p256-c10-p10` | 256 B | 10 | 10 |
| `p1024-c50-p1` | 1,024 B | 50 | 1 |
| `p1024-c50-p10` | 1,024 B | 50 | 10 |
| `p256-c1-p1` | 256 B | 1 | 1 |
| `p256-c100-p1` | 256 B | 100 | 1 |

## Hardware evidence and validation

The run records `uname`, CPU model/count, measurement affinity, the hash of
`/var/lib/hydracache-perf/runner-provisioned.json`, tmpfs verification, and
pre/post `reference-runtime-irq-guard` output. These checks are methodological
context, not a replacement for the fail-closed qualification gates. The Redis
container cpuset request and any host-level enforcement warning are also
recorded; an unenforced Redis cpuset is a stated limitation, not hidden.

## Reproduction

Run as `github-runner` on the selected host:

```bash
scripts/perf/run-relative-eight-cases.sh /tmp/hydracache-relative-eight-cases
```

Copy the resulting `relative-eight-cases.txt`, `hardware-validation.txt`, and
`raw/` files into an immutable, date-stamped study directory. Keep the branch
history and raw logs together so future optimizations can be compared against
the exact workload and hardware validation context.
