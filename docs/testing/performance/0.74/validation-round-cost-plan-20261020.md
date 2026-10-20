# Read only WSL time diagnosis and validation round attribution

Baseline is `8ecb405b4068236970bcf72563a72ee3e7ca9c2e`. Read only Windows/WSL
clock, boot, configuration and bounded journal observations are authorized.
Never set time, change configuration, stop/restart a service or WSL, or access
the server. New local synthetic fixtures remain separate from product workloads.

Record WSL running state before entering it, current/prior boot visibility,
time synchronization configuration/status, time-related journal/kernel records
and relevant Windows time-change events. Use one bounded persistent observer
process to sample Linux realtime/boottime/monotonic/raw clocks alongside Windows
UTC and monotonic arrival observations. Keeping that observer alive does not
change VM idle policy. Distinguish directly observed clock reversals/boot changes
from a possible actor; missing/volatile logs cannot attribute an old event.
Do not assume a read-only call to a stopped distribution preserves its old boot.

Attribute the eight existing ValidationFrameStep operations without changing
their order, live predicates, callbacks, error/effect order or 300/60/10 bounds.
Inactive test-only scopes take numeric snapshots of existing kernel/live/auth
counters and aggregate per-step elapsed/count deltas inside each preparation
phase. They never reset counters, retain a validation result, or add a guard
clock. Distinguish inclusive nested spans from wall time outside instrumented
children; never add overlapping live/kernel/auth spans as exclusive CPU.

Unit controls must preserve values/errors/panic identity, nesting and thread
locality, inactive behavior and parent counts. Exact source reconstruction must
recover all preceding bodies and old assertions, including the existing Starting
markers. No new runtime candidate or accepted allocation/latency gain is in scope.

Freeze source/image/recipes before three serial healthy profiles per GNU 1.94
and MSRV 1.88 lane. Compare all parent counts with preceding complete profiles;
per-step kernel nodes/edges must conserve counts and observed child time where
the snapshot stacks are empty. Require all eight completed steps in each full
round. Retain the first original refusal and stop that compiler's cohort; do not
retry or splice partial results into a healthy cohort. A bounded stable clock
observation is not a replacement for live admission or a proof of past stability.

Run focused tests, affected-package check/strict clippy, full historical source
guards, formatting, docs, registry and local performance contracts before cost
probes. Independently attempt each unchanged unprofiled five-case local matrix
once; refusals remain closed gates. Bind initial/final boot/configuration and
unchanged original bundle/qualification and six preceding sealed packets.
No server run, installation, supervisor/clock/service operation, full release
qualification or product workload is permitted. Keep evidence integrity separate
from completeness, time continuity and execution/trust admission.

## Local outcome

Read-only diagnosis finds Ubuntu stopped before entry and persistent records for
the earlier boots. The old refusal boot contains two initial timesyncd
synchronizations; Windows' selected time-change query returns no events. Neither
fact establishes the actor behind the old reversals. The first observer fails
boot-argument validation before sampling; its failure remains retained. A second
implementation obtains boot through a read-only cat subprocess and collects
180 samples in one process/boot. There are no negative adjacent deltas, but
Linux realtime/boottime advances 89.554825/89.554827 s versus Linux RAW
93.269314 s and Windows QPC 93.264936 s. Time correspondence is not proved.
No clock, service configuration/control or WSL idle policy is changed.

Instrumentation `362f25660992dfaf0ef83873a78996c8a6decbb0`, measured source
`f1e6ea87e0e67b3e1dcdde0bc563a79cc9e8644a`, adds only test scopes/getters.
All 477 preceding bodies reconstruct exactly and all 446 runtime paths remain.
Three observer controls preserve inactive/nested/thread-local behavior, original
errors/panic identity, parent counters and escaping-scope rejection. Both lanes
pass focused tests, cargo check and strict lint. All 361 source/contract assertions
and repository checks pass. Initial canary and recipe-parser failures remain
disclosed rather than erased; no old assertion or runtime predicate is changed.

Three complete GNU profiles retain all parent counts across ten phases. All eight
steps complete 46 times in outputs; kernel nodes/edges/roots conserve counts and
time, and every output kernel observation is attributed exactly once. ReservedLive
owns 85.62–85.82% of the eight-step sum (2691.086–2833.414 ms) and 736 of 782 auth
calls. Its kernel Boot/Namespace wall outside children occupies
484.11–516.05/416.49–434.55 ms at 129904/259808 calls. The separate live view of
ActiveAuthority occupies 420.48–455.39 ms outside its children and overlaps the
kernel view. These are local instrumented wall observations, not exclusive CPU,
cache API latency, throughput or an accepted optimization gain.

The first MSRV profile returns 101 at unchanged kernel clock-order, realtime
backwards 4304620710 ns with advancing boottime and unchanged boot; causal expiry
does not refuse. Its remaining profiles are not run. The independent GNU matrix
passes all five cases; the MSRV matrix refuses healthy with realtime backwards
1825312028 ns. No retry, later failed-lane case or partial-profile promotion.
Profile origins and postflight share boot 48b26652-49a9-406f-b987-35c7a1a725cc,
distinct from the earlier standalone observer. Configuration bytes remain equal;
postflight finds no matching owned root-fixture directory/cgroup. This is not a
single cross-boot campaign and does not explain away the live failures.

No runtime candidate is adopted. Complete cross-compiler attribution, both-matrix
acceptance and native execution admission stay closed. Further local work should
isolate per-call representation/syscall/authority costs within ReservedLive,
especially fresh boot/namespace checks, without reusing observations or weakening
their error order. Stable time/environment admission remains independent; any
clock/service change or new native/server control needs separate authorization.

Evidence commit `9868d51f32f94a83c4165465acf1351b6f616179` seals 129 members
plus inventory, SHA256
`b089573e0dc544df769c07b64be41562f107cdc1add64b1cd0c9512df2d882d6`.
Strict verification rehashes six preceding packets and rejects 21 summary
mutations, while explicitly retaining false cohort/both-matrix/native admission
flags. Changes remain local; no push is made through the red live gate.
