# CPU attribution within fresh leaf revalidation

Baseline is `b6f84539`. The previous actual-phase CPU packet prioritizes fresh
record/marker revalidation, not buffer allocation specifically. Attribute its
held metadata, open_at, before metadata, parent metadata, initial buffer,
read_to_end and after metadata calls without changing runtime or admission.
Reuse the existing source-checked owners Observer and fixtures; add no new
runtime copy, cached observation or validation schedule. Preserve all live
checks, refusal order, retained File lifetimes and original 300/60/10 windows.

For each depth one/seven and record/marker kind, compare ten cases: actual
method, uncounted traced method, seven independently selected expression
windows, and an empty observer callback. A selected run times only one site;
every other expression remains uncounted and still executes. Keep all fresh
open/metadata/read operations and hold returned values through the original
comparisons. The empty callback executes once per leaf but does no filesystem
work. It estimates a measurement floor, not an interchangeable validation.

Use the same checked Linux CLOCK_THREAD_CPUTIME_ID functions, with a source
canary comparing their bodies to the preceding probe. Retain resolution and
64 adjacent clock-pair observations. Reject errors, malformed readings,
overflow and negative deltas without fallback, clamping or time repair.
Synthetic clocks test selection, value/error forwarding, delayed destruction,
counts and refusal paths. The allocation observer stays inactive throughout
CPU batches. Its installed allocator bookkeeping still contributes overhead.

Each compiler has one sequential capture after builds/checks finish. Each
depth/kind cluster uses 32 warmups and twenty 1024-operation samples per case:
ten forward rotations and ten reverse rotations balance every position.
Record raw outer batch CPU, selected-site CPU sum and callback counts. Require
selected counts = depth times 1024, other counts zero, and nested sums no
larger than outer CPU. Report outer CPU per phase call separately from site
CPU per leaf. Do not subtract the empty cost or sum separately observed sites
into an exact CPU partition. Overlapping short-site/empty ranges cannot prove
a site-specific improvement. No numeric candidate acceptance threshold applies.

Separately run the frozen actual/traced owner allocation/read decomposition
and unchanged eight-case full control, with existing warmups/iterations. Their
counts and payload/name hashes/epochs must agree with the sealed owners packet;
traced totals must still equal actual totals. CPU samples are observations in
the diagnostic binary, not kernel-only CPU, production latency or API throughput.
Read-only environment receipts describe unpinned WSL without tuning it.

Freeze non-overwrite check/capture/review recipes before measurement. Bind
committed source inputs, binary and recipe hashes, retain all failed preflights
and never retry a failed measured invocation into a healthy cohort. Run focused
tests, affected all-target checks and strict all-feature lint on GNU 1.94/MSRV
1.88, historical source contracts, fmt/docs/registry/local performance checks.
Review structural JSON facts, all eighteen prior immutable inventories and
the frozen qualification digest, then seal the new packet once.

No runtime hypothesis is accepted from these counters alone. A subsequent
isolated candidate needs its own preregistered paired control and fault proof.
Do not run another live matrix, server operation, installation, supervisor
action, clock/service change, product workload, release qualification or push.
Earlier live/time failures and independent delivery, all-writer cessation,
lifecycle, SIGKILL/lost-ACK, reconnect and enrollment gates remain closed.

## Local outcome

The [packet](local-runs/authority-leaf-cpu-20261021/README.md), capture source
`b99c1c759630cf632c0a13a905c3ddf9d822def5`, retains all 800 CPU rows per
compiler and twenty samples per cluster/case. Neither preflights nor measured
invocations fail; none is retried. Source canaries and synthetic clocks retain
the exact prior clock bodies, observer expressions and returned-owner lifetime.

At depth seven, selected initial-buffer medians span 238–273 ns per leaf;
empty medians span 192–215 ns, with overlapping ranges. Open spans
1088–1276 ns, metadata 589–743 ns and read 811–864 ns in the selected
median windows. Their instrumentation and lifetime scope do not establish
pure allocator cost or an exact CPU partition. No buffer optimization is
accepted; all outliers remain without subtraction or filtering.

Owner/full controls match prior allocation/read counts, sites, payloads and
epochs. Both lanes pass 210 authority tests, ancillary checks, all-target
checks and strict all-feature lint. All 385 source assertions and repository
gates pass. A separate bounded read-only file-name encoding candidate may
test request reduction with cold/warm and fault controls; no CPU gain follows
from the aggregate open window. Workload, qualification, original windows
and old live/time refusals remain frozen; no server or product gate opens.

Evidence `9a460ee7e0ac2b2f9f829b9fd8c8be4a8b28f27f` seals 59 members plus
inventory SHA256
`4a4c317f7e9cc878b57aa205324d865577cd17772bff3dea0d7e513c01b2d805`.
All eighteen prior packets remain intact; eight structural JSON and ten scope
mutations are refused. All 60 Git entries preserve raw bytes. The immutable
packet does not authorize the separately
[preregistered candidate](authority-open-name-plan-20261021.md) as a gain.
Qualification SHA256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
