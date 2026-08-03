# Target aggregate summary

This table is a convenience view derived from the checked-in
`telemetry-summary.json`. For each target, `p50-of-case-p50` and
`median-case-p95` are medians across the 48 case/operation/repeat summaries;
`max` is the maximum of those summaries. Memory values are bytes. The raw
JSONL/CSV files and per-workload summaries remain authoritative.

| Target | Metric | p50-of-case-p50 | median-case-p95 | max | samples |
|---|---|---:|---:|---:|---:|
| HydraCache | container CPU % | unavailable | unavailable | unavailable | — |
| HydraCache | VmRSS | 321180672 | 326170726 | 612024320 | 180 |
| HydraCache | VmHWM | 321180672 | 326170726 | 612024320 | 180 |
| HydraCache | cgroup memory.current | 331210752 | 336354202 | 625758208 | 180 |
| HydraCache | cgroup memory.peak | 341603328 | 345815859 | 649388032 | 180 |
| Redis | container CPU % | 43.974 | 50.745 | 104.617 | 144 |
| Redis | VmRSS | 15352832 | 15405056 | 27344896 | 144 |
| Redis | VmHWM | 27172864 | 27172864 | 27344896 | 144 |
| Redis | cgroup memory.current | 8086528 | 8138752 | 20303872 | 144 |
| Redis | cgroup memory.peak | 20504576 | 20518912 | 20566016 | 144 |
| Hazelcast Community | container CPU % | 13.603 | 17.052 | 158.080 | 1247 |
| Hazelcast Community | VmRSS | 395313152 | 395319706 | 397426688 | 1247 |
| Hazelcast Community | VmHWM | 397766656 | 397766656 | 400248832 | 1247 |
| Hazelcast Community | cgroup memory.current | 376035328 | 376224768 | 378658816 | 1247 |
| Hazelcast Community | cgroup memory.peak | 378601472 | 378601472 | 381825024 | 1247 |

HydraCache CPU percentage is unavailable in this collector path, while its
process ticks and memory fields are preserved in the raw telemetry. JVM heap
is unavailable for Hazelcast and is not inferred from RSS.
