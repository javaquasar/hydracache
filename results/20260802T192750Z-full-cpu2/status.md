# Pre-hardening CPU2 exploratory run

This earlier 144-workload run is preserved as a harness diagnostic. Its IRQ
delta guard passed, but it predates the explicit container-PID affinity
hardening and therefore is not a valid comparative result: rootless Docker
reported an empty cpuset and container processes were not proven pinned.

- Source: `117e6b69f44aca38cfa8681492c4630062e22249`
- Requested measurement affinity: `2`
- Use: harness/telemetry debugging only; no performance or SLO claim
