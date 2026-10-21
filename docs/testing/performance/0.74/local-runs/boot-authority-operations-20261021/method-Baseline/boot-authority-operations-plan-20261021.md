# Local Boot and authority operation costs

Baseline runtime is `b074f8f9f3d8f9766b3b04c59a8291941a7e3b1d`;
predecessor evidence/source is `c80a3fb8d18df0e2de43137a9aac7fd90c4e50ff`.
This step measures actual authority methods with the existing selected-operation
and full-phase probes, and adds an opt-in Boot operation control. No original
admission attempt, server or product workload is run. Original 300/60/10
windows, live predicates, fresh reads, FD ownership and failure order remain.

The Boot control compares actual fresh matches, an uncounted equivalent walk,
five singly selected thread-CPU windows (four opens, four filesystem checks,
one bounded read, one decode and four closes), and an empty window. Eight
forward and eight reverse rotations give sixteen 1024-call batch observations
per mode after 32 warmups. Retain all observations and 64 adjacent clock pairs.
No clock-pair subtraction; the empty callback has a different body/value type.
Selected windows include boundary-clock overhead and cannot be summed as an
observer-free CPU partition. Explicit closure follows original file-before-
decode and reverse directory declaration order. This equivalent walk is
diagnostic instrumentation, not an alternative production validation path.

Authority uses unchanged `authority_cpu_actual_phase_attribution`,
`authority_leaf_cpu_selected_attribution`,
`authority_open_name_allocation_cpu_attribution`, and the owners/full request
controls on GNU 1.94 and MSRV 1.88. Each compiler runs its bounded children
inside one ordinary-user WSL parent with metadata/profile/postflight identity
checks. Frozen iterations, seeds, warmups, payloads and cases stay unchanged.
Preserve all nonzero exits; no automatic rerun or sample replacement.

Only one candidate is authorized by this method: extend the existing bounded
24-byte read-only name encoding from short files to short directories. Longer
names retain CString; NUL refusal, syscall flags, errno, fresh metadata/read
checks and descriptor lifetimes must match. First retain and review baseline,
then differential/fault tests, implementation and paired candidate observations.
Accept only the evidenced allocation request reduction with unchanged semantic
controls; ambiguous CPU is recorded as ambiguous, never latency gain or proven
CPU nonregression. Any observed semantic failure rejects the candidate. The
new bounded storage/encoding already exists for file names; no new retained
state or cached successful guard is introduced. If unsupported, retain a
negative result and revert only the candidate.

The synthetic authority leaf name exceeds 23 bytes and retains its fallback;
only the three-byte `tmp` component changes. Two directory revalidations in
full inspect predict exactly two requests/eight requested bytes removed per
inspection, and one request/four bytes per isolated directory phase. The
existing `sub` helper case predicts one four-byte request removed. All other
helper/cold/warm cases, site counts and fresh read counts must remain equal.
These deterministic targets come from the source, not measured CPU thresholds.

All changed Rust packages require targeted tests, all-target checks and strict
all-feature lint. The manual Boot test is registered; historical source views
may remove only its exact hook and gate. Differential controls cover roots,
fresh identity/flags, original errors and kernel trace counts. Repository
format/docs/registry/local performance contracts and raw evidence integrity
remain required. Historical sealed packets are never modified; their old
HEAD-specific readiness guards are historical-source checks, not permission
to replace their baseline. No service, clock, installation, supervisor,
server run, qualification, push or admission change.
