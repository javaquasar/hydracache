# First server preparation attribution: expiry before enrollment completes

After explicit approval of the 20-file transfer and one isolated run, the paid
Linux host executed exactly one healthy preparation fixture with existing cost
observers. Source commit `c293f190b6a52914bbde0f2c6b2e8f96b0af82a5`, exact GNU 1.94
release image SHA256
`d25675c71977c4dc48d4cbdf2973081f5d37189c4339e7de5465f571e2ad94c7`.
The [pre-registered plan](server-preparation-profile-plan-20261028.md) describes
the different private-namespace fixture composition and its no-send boundary.

## Actual result and retained first refusal

The libtest exits 101: **0 passed, 1 failed, 0 ignored, 1,592 filtered**, 13.45 s
printed total. Seven phase reports appear, the last refusing in `enrollment`.
`placement`, `fixed-slice` and consuming join are unobserved. The first causal
refusal is `expired` at `final_clocks`: issued 1,913,468,990,790,781 ns, observed
1,913,478,990,960,219 ns, age **10,000,169,438 ns**, against the unchanged
10,000,000,000-ns window. Both boot IDs are
`e81671d6-bf23-4ebf-b8fe-5dad1f45866f`. No original send, service-start, readiness,
worker release, completion, reconnect or product workload occurred. This is a
failed preparation fixture, not a successful lifecycle rehearsal.

The raw shared fixture prints `server=false` as a legacy static field. It is
retained verbatim; SSH/root receipts and the outer observation identify this
actual run as `server=true`. Do not alter raw logs or mistake that static field
for evidence that execution was local.

## Phase observations, not repeatable performance claims

| Phase | Returned | Wall ms | Process CPU ms | Auth/signature calls |
| --- | --- | ---: | ---: | ---: |
| prepare-consent | yes | 546.685 | 546.663 | 68 / 68 |
| accept-consent | yes | 284.102 | 284.106 | 34 / 34 |
| outputs | yes | 6,668.333 | 6,581.329 | 782 / 782 |
| enrollment-frame | yes | 340.558 | 340.478 | 34 / 34 |
| fixed-manager-message | yes | 324.122 | 317.955 | 34 / 34 |
| owned-root-preparation | yes | 946.210 | 945.729 | 102 / 102 |
| enrollment | no | 143.653 | 143.584 | 17 / 17 |

These distinct phase windows total 9,253.663 ms wall / 9,159.844 ms process CPU;
they do not cover the whole attempt. CPU excludes child processes. Outputs is the
largest observed phase. Within it the kernel Boot view records 153,088 calls,
1,549.880 ms inclusive / 1,421.750 ms outside its traced children; Namespace has
306,176 calls / 1,135.057 ms. The live KernelContext view records 8,464 calls /
3,718.514 ms, ActiveAuthority 17,066 calls / 956.566 ms, and LocalContext 4,232
calls / 899.667 ms. Kernel/live views overlap and must not be added to form a
disjoint cost budget or CPU attribution. Authentication also nests in those views.

This sample establishes substantial parent-process CPU consumption during
outputs under the existing observation schedule. It does not show fsync-specific
latency, distinguish syscall CPU from parsing/validation/observer overhead, prove
global clock stability or an optimization gain. Wall minus process CPU is not
an I/O measurement. The host was not exclusive, no same-semantics baseline pair
ran, and the fixture differs from the original-service WSL attempt. Do not claim
a server/local speed ratio. The tiny expiry overshoot is an observation point,
not an estimate of remaining work. No unchanged retry was performed.

## Containment and host postflight

The detached original CI envelope reports test-failure/101, `retired=true`, no
errors, with the original 1,200-second envelope, 4 GiB floor, five-second capacity
poll and five-second retirement. ECHILD retirement concerns only owned fixture
descendants; it is not production all-writer cessation authority. Collection
reads only this same consumed attempt. No replacement or second run is dispatched.

Preflight/postflight JSON agree exactly: supervisor active/running, PID 377415,
NRestarts=0 and invocation unchanged; boot, protected executable/receipt hashes,
host mount tree/namespaces and clock-service state unchanged; all nine campaign/
lifecycle markers and prior/surviving fixture cgroup prefixes absent. There is
no release install, account/enrollment update, /opt or /etc write, host service
change, clock adjustment, AppArmor change or common host-lock acquisition.
Root-only temporary source/images/logs are retained rather than broadly deleted.

## Next boundary

Continue source/security/package integration independently. For execution,
inspect the already recorded outputs/validation cost offline and repair only
confirmed leaf implementation work with unchanged call count/order and origins.
No further live run is included in this consumed authorization. Original delivery
still needs the compatible systemd 257 environment; retirement before manual
cleanup, durable lifecycle/cells 2–4, SIGKILL/lost-ACK and same-attempt reconnect
remain open. No performance candidate is accepted or reopened by this profile.

The [raw packet](local-runs/server-preparation-profile-20261028/index.json) retains
the exact source/effect inputs, initial sandbox TEMP control refusal, transfer and
stage receipts, seven remote files, first refusal, phase analysis and original
baseline parent index. Native binaries and the transferred tar are excluded from
Git; image hashes and receipts remain. The initial automatic transfer rejection
preceded execution and was resolved by explicit operator consent; it is not a
runtime test failure. A later local read-command quoting error caused no SSH
execution and no fixture retry; corrected reads observed the same terminal root.
