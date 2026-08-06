# 0.67.1 indicative performance characterization

HydraCache exposes an explicitly non-authoritative performance tier so that
developers can inspect reproducible same-host measurements while the protected
bare-metal `reference-v1` campaign is still incomplete. This tier is useful for
hypothesis formation and optimization planning; it is never a substitute for
qualification, bootstrap, reviewed activation, or frozen-candidate evidence.

The machine-readable boundary is
[`perf-policies/indicative-exploratory-v1.json`](perf-policies/indicative-exploratory-v1.json).
Every generated report and artifact manifest records that policy's digest and
the following immutable negative claims:

- `authoritative=false`;
- `capacity_bearing=false`;
- `qualification_evidence=false`;
- `bootstrap_evidence=false`;
- `ship_evidence_eligible=false`.

Numbers from this tier may be described only as same-host exploratory
characterization. They must not be presented as a capacity floor, production
sizing recommendation, portable target ranking, release qualification, or
evidence that `reference-v1` is bootstrapped.

## Filesystem and RAM-only diagnostic modes

The three-target relative-eight harness accepts two storage modes:

- `filesystem` preserves the existing exploratory behavior;
- `ram-only` requires the entire output directory below `/dev/shm` on a verified
  `tmpfs`. Raw logs, one-second telemetry, and HydraCache's diagnostic data
  directory remain there throughout the measured window.

RAM-only is a diagnostic control for distinguishing ordinary output/Hydra data
I/O from other sources of noise. It does not move rootless-Docker layers, kernel
metadata, binaries, libraries, or unrelated host activity into RAM. It does not
relax either IRQ guard and can still be rejected by NVMe or network interrupts.
It is intentionally unavailable to the qualification/bootstrap workflows.

Example:

```bash
export EXPLORATORY_STORAGE_MODE=ram-only
export HAZELCAST_IMAGE='hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:<reviewed-digest>'
scripts/perf/run-relative-eight-cases-telemetry.sh \
  /dev/shm/hydracache-indicative-$(date -u +%Y%m%dT%H%M%SZ)
```

The report generator writes `indicative-receipt.json`; the artifact manifest
includes the receipt and policy identity. Copy the completed directory to
durable storage only after the measured phase and retain the original bytes and
hashes. Never copy it into `target/test-evidence` or a bootstrap campaign.

## Relationship to the release gate

The strict `reference-v1` contract remains unchanged: exact workload, SLO,
repetitions, zero-error rule, `0.15` spread ceiling, calibration, affinity,
quota, privacy, full-dress chain, five samples, independent review, and frozen
candidate are all still required. An indicative result can guide a code change;
it cannot close any 0.67.1 work item.
