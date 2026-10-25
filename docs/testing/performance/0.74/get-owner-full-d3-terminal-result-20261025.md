# GET owner terminal D3 rejection on the dedicated host

GET-owner is rejected for Release 0.74. The complete, valid 240-process real-network
allocation stage fails the preregistered 20% practical-effect floor in all four
required mTLS cells. This is a terminal negative admission decision, not a failed
semantic test and not a claim that the ownership transfer saves nothing. No CPU,
latency, goodput or allocator-retention acceptance follows from counting data.

## Frozen comparison and measurement scope

The [allocation contract](get-owner-full-d3-allocation-contract.toml) fixes five
independent A/A and five A/B pairs in each of 12 cells, counterbalanced by pair.
Its [placement amendment](get-owner-full-d3-allocation-v2-contract.toml) predates
both implementation and numerical execution. Every pair consumes the same clean
source `be0499a8cb0a2ad0b7328d6ac0cf6227114cebcc`, root/observer locks, GNU 1.94.0
release toolchain, seed 740074, binary corpus, schedule and diagnostic PKI. Only
the existing `get-owner` feature differs in A/B; both A/A roles execute off.
The coordinator source is `53b99dea6`; it is an operator-authorized finite
maintenance route, not production enrollment or original-start admission.

The measured quantity is gross **requested Rust System layouts** during the
canonical scheduled interval: colocated client/server, decode/dispatch/reduce/
encode/write/flush, byte oracles, task/wire drain and observation projection.
Startup, preload, PKI, warmup, final verification, shutdown and JSON serialization
are outside that interval. Neither System layouts nor process RSS certify GNU
allocator active/resident/retained storage. Ratios compare each surface with
itself, never RESP security with HC2 security or one native API with another.

RESP uses one physical connection, 128 execution slots and 1000 one-GET offers
at 1000000 offers/s with the v2 queue limit of 1024 and original timeout/drain bounds. Native
controls independently use the original eight slots and 128 offers at 10000/s.
The four native surfaces are embedded HydraCache, direct ClientSurfaceState,
HC1 and HC2 mTLS. Every process has the original 60-second ceiling. Both variants
use the same fixed private diagnostic material; only public seals enter evidence.

## Results and decision

All 240 receipts pass workload/source/material pins, exact success accounting,
byte/final-state oracles and explicit shutdown. Each RESP receipt independently
proves actual write-to-reply overlap of 50 for 4 KiB and 10 for 1 MiB. Every
attempt has an original-cgroup `populated=0` retirement record. All five A/A gross
ratios per cell stay within the unchanged 1% noise ceiling.

The estimator uses every one of the five A/B log ratios and the original
two-sided 95% t(4) interval, with t=2.7764451051977987. Admission requires the
**upper cost bound** at most 0.80 in each affected cell. Percentages below are
geometric-mean reductions, not throughput improvements.

| Transport | 4 KiB / depth 50 reduction | Upper cost bound | 1 MiB / depth 10 reduction | Upper cost bound | Allocation floor |
| --- | ---: | ---: | ---: | ---: | --- |
| RESP2 plain | 23.4698% | 0.7653015134 | 33.2864% | 0.6671459597 | pass |
| RESP3 plain | 22.4019% | 0.7759812647 | 33.2774% | 0.6672367092 | pass |
| RESP2 mTLS | 15.7384% | 0.8426415783 | 19.9770% | 0.8002311132 | fail |
| RESP3 mTLS | 15.2500% | 0.8475000034 | 19.9738% | 0.8002638295 | fail |

Each of the four independent native gross-allocation ratios is exactly 1.0 in
all five A/B pairs, below its unchanged 1.05 ceiling. This is allocation-only
nonregression, not CPU, p99 or goodput clearance. The mTLS 1-MiB results are close
to the floor but do not meet it: rounding 19.9770% to 20% is not acceptance.
No pair is omitted, no secure cell is substituted with plaintext, and no threshold
is revised after observation.

The mandatory allocation failure is sufficient to reject this candidate. The
remaining timing, concurrency/transition/slow-reader, full retention and hosted
semantic acceptance gates are therefore **not run for admission**; they are not
silently marked passed. The earlier scripted D3a success remains valid in its
narrow scope and does not override this real secure-transport failure. Additional
profiling would be a new work decision, not permission to retry this cohort.

## Preparation defects and first results

Allocation-v1 images were built but never numerically executed. Review found
that its socket count followed execution slots and that `pipeline_depth` is an
outstanding limit, not an operation multiplier. Schema 2 binds physical placement
separately and reconstructs actual overlap from original wire timestamps. No
response-paced batch or multiplication of the denominator is introduced.

The first Windows debug semantic control proves the required depth in its first
six cells but refuses the RESP2 mTLS / 1-MiB observation guard; shutdown succeeds.
That first invalid observation and the preceding compile error remain unchanged.
It is not rerun or relabeled as a GNU result. The first GNU release controls pass
all eight cells independently with feature off and on, before numerical sealing.
The complete Windows fast suite passes 103 cases with the explicit GNU semantic
preflight ignored; the added placement-refusal test also passes. Strict focused
checks remain separate from the retained failed debug control.

The first simple scope fixture encounters ENODEV after PID1 removes its cgroup.
Missing/ENODEV is never accepted as an empty-writer proof. The final coordinator
instead retains owned subgroups in a new delegated unit through ExecStopPost,
checks original boot/device/inode, waits for `populated=0`, and requires exclusive
reacquisition of the inherited fence. Separate synthetic controls cover healthy
exit, absolute deadline and external pidfd SIGKILL of the owner, including forked
writers and copied fence descriptors. First subgroup, timer-GC and harness
refusals are retained rather than rewritten as successes.

## Authorized maintenance and recovery

Only `hydracache-performance-supervisor-074.service` is intentionally stopped,
after checking its original PID 8839/generation, installed binary/receipt, boot
and absence of campaign/lifecycle markers. The shared host fence is acquired
only after that exact service is inactive. Each worker enters its own root-owned
subgroup before dropping to UID/GID 1000 and inherits the fence descriptor.
PID1 owns an absolute per-attempt deadline and a bounded outer service lifetime;
SSH is not the process owner. The launch SSH session closes while the same
coordinator continues the 240-process series.

All original writer groups are proved empty before fence release. Supervisor
restoration is autonomous and verified: active/running, PID 377415, NRestarts=0,
same boot and installed executable/receipt, and no campaign/lifecycle markers.
The complete transient unit lasts about 220 seconds, including cleanup. No
release is installed, clocks/AppArmor are unchanged, and no 6-hour/24-hour
qualification or expensive Actions run is started. This does not close the
separate production original-start/enrollment/lifecycle gates.

## Evidence and selective rollback

The [raw content-addressed packet](local-runs/get-owner-full-d3-network-allocation-20261025/index.json)
retains all 240 stdout/stderr streams, intents, child identities, retirement
records, full pair order, exact seals, first refusals, build/check transcripts,
fault controls and final host snapshot. Private PKI and ELF images are excluded;
their hashes and public receipts remain. The offline reader rechecks raw bytes,
all wire/semantic predicates, source/feature placement and the fixed estimator.

The terminal rollback disables only the private GET-owner runtime selector.
The canonical borrowed reducer is restored; historical differential/property
helpers remain test-only, together with all existing integration guards. The
empty Cargo feature is retained solely as a compatibility tombstone and cannot
activate the candidate. Fresh allocation-stage CLI execution on retired source
is refused; historical receipts remain replayable. The canonical runtime's
LF-normalized SHA256 remains
`c98ab563d4fc9181735d432484118561ad9bf773d32fb2865f4a591dcc50e932`.

Release performance proposals accepted: zero. C74 remains unresolved, the
qualification manifest and frozen 0.73 artifacts are unchanged. This closes the
GET-owner experiment for 0.74; it does not declare the whole release ready.

## Selective rollback verification

The new terminal regression test first fails against the still-pending registry,
then passes after disabling the runtime selector. Its preceding test-compilation
error is also retained. The final focused checks pass:

- 178 Redis-compat cases with all features, with 23 existing opt-in cases ignored;
- 19 Redis/server lifecycle cases, including TLS/Auth and drain;
- the terminal regression, four historical GET-owner contract guards and five
  W9b guards, including the unchanged canonical runtime digest;
- eight allocation-worker unit cases, with its explicit GNU wire preflight
  ignored rather than rerun on retired source;
- all-target/all-feature check and strict Clippy for Redis-compat/xtask and the
  standalone observer, plus formatting;
- six offline coordinator controls and eight packet mutation controls;
- documentation consistency and `performance-contract-check --release 0.74`
  in local, non-promotable mode.

The first contract-check invocation omits its required release argument and
refuses before evaluation; the corrected command passes. This is a command
correction, not a second numerical attempt. All scoped Rust command captures
retain unchanged source/HEAD before and after execution. These checks do not
replace the previously red full-workspace/security gates or create ship admission.

```text
python docs/testing/performance/0.74/local-runs/get-owner-full-d3-network-allocation-20261025/test_verify.py -v
python docs/testing/performance/0.74/local-runs/get-owner-full-d3-network-allocation-20261025/verify.py
```
