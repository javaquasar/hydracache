# Fixed grouped owner process loss evidence

This packet tests the retained OriginalGroupedPreparation owner through the
cfg(test) seam of its shared production construction body. It does not execute
the bundle, send a manager request or enroll a fixed production install.
[Preregistration](../../diagnostic-fixed-grouped-process-loss-plan-20261017.md)
pins baseline 49db0f7f0b25089f72f9459f5d2fd5880d122e44. The harness implementation is
30bf496812da1121eeb9b6d2163501c4a80c4282. These results belong to the new owner,
not the older grouped candidate or its process-loss packet.

## Code and preservation

The new diagnostic_fixed_grouped_preparation_crash_tests.rs is a separately
namespaced copy of the prior process harness. An exact static comparison allows
only private test/environment/log renames, the fixed grouped constructor name
and one explicit rustfmt fold of a longer log line. Four mutation controls change
SIGKILL, constructor, inspection count and repeat count. The old parent harness
is exact after stripping only the two-line new child registration.

Every production grouped/material/message/alarm/validation/clock/deadline body
stays exact at the baseline. No API or durable/wire production format changes.
The earlier origin-test correction and its negative remain in the unchanged
previous packet, verified in an exact archived source view with an owned index
copy. The active index must remain unchanged during that prior proof.

## Process protocol and oracle

Each case owns a fresh selected libtest Child and private roots. Before waiting,
the parent opens pidfd for that exact unreaped child. A bounded readiness line
must name the chosen logical cut and the same Child PID. Only that pidfd receives
SIGKILL. Kernel terminal readiness and wait status signal 9 are both required;
the harness never discovers or signals a process by name.

Six cuts follow the frozen seed 0x07406c17 order:
returned-unacknowledged, bound, message, opened, armed, loaded. Five are existing
internal test checkpoints; the first is after successful return but before a
private acknowledgement. No new production observation is inserted. The original
Reserved/Starting/progress-receive offsets remain 300/60/10 seconds. Existing
60-second harness waits and outer timeouts do not renew those admission clocks.

Before release or death, the parent requires flock exclusion and checks sealed
intent/Reserved/Starting, exact output inventory and empty sinks. It snapshots
original ancestry, authority and interrupted context bytes and checked identity/
mode/link/size/mtime/ctime metadata. After confirmed death it requires flock
acquisition and unchanged original/authority snapshots; killed context stays
retained. Normal exits must finish full revalidation and clean temporary context.
Two fresh selected inspector processes per case then refuse existing history
without mutation. Killed context is preserved through both inspections before
the parent's owned fixture-root cleanup. No repair, retry, adoption or completion
is authorized.

An ordinary control exits zero and emits exactly one private ACK. A separate
control completes the same original revalidation and owner drop but exits zero
with no ACK. This models an omitted return notice, not manager delivery,
SCM_RIGHTS, network loss or a real systemd acknowledgement.

## Declared cohorts and local checks

The first Rust 1.94 screen passes one ordinary control and one post-return
SIGKILL, with four fresh inspections. Only its successful marker unlocks either
full matrix in the isolated runner. Each full compiler matrix has three repeats
of six kills, one ordinary control and one zero-exit ACK omission: 24 cases and
48 fresh inspections per compiler. Across both full matrices the declared totals
are 36 kills, six ordinary controls and six omitted-ACK controls. The separate
screen brings the complete declared packet to 50 cases and 100 inspections.

checks.sh runs current Rust 1.94 and MSRV 1.88 normal GNU release/debug protocol
units (two codecs and an inactive child entry), both original and fixed grouped
alarm units, unchanged parent protocol, trace units, 43 focused integration tests,
eight new and five old compile-fail boundaries, all-target check and strict
all-feature Clippy. Windows supervisor/xtask checks and strict Clippy, all 199
static contracts, the registry with 184 discovered gates, workspace format,
doc-check and local non-promotable performance-contract-check pass. This is not
the full milestone verify, require-ship or qualification.

## Completed outcomes

All declared cohorts pass without process-test retries or source/deadline changes.
The Rust 1.94 matrix takes 311.40 seconds; MSRV 1.88 takes 311.37 seconds. The
separate first screen takes 25.75 seconds. These are complete correctness-family
durations, not per-operation timing, throughput or a compiler performance claim.

| Cohort | SIGKILLs | Ordinary ACK controls | Zero exit without ACK | Fresh inspections |
| --- | ---: | ---: | ---: | ---: |
| First screen Rust 1.94 | 1 | 1 | 0 | 4 |
| Three repeats Rust 1.94 | 18 | 3 | 3 | 48 |
| Three repeats MSRV 1.88 | 18 | 3 | 3 | 48 |
| Complete packet | 37 | 7 | 6 | 100 |

cases.json derives exactly 50 unique case tuples from the three raw logs. The
verifier requires all 100 ordered inspection rows and the original seed/cut order.
All original/authority snapshots and interrupted history pass their unchanged
checks; lock acquisition succeeds only after confirmed child termination. Final
postflight finds no owned helper executable, preserves the active stable service
and installed artifacts, and reconciles all nine nonexecuted bundle inputs.

The log verifier's self-test accepts one synthetic codec cohort and refuses nine
mutations of ACK, signal, seed, membership, duplication, stage, repeat, summary
and event interleaving. Each pair of fresh inspection records must precede its
case result and follow the previous result.
It is not process evidence. verifier-initial.log retains a local generator-only
PowerShell parse failure: JavaScript replacement interpreted a literal dollar
anchor as replacement syntax. Regeneration with literal callbacks corrects the
helper without changing any harness, predicate or outcome. No failed process
cohort is silently retried into success.

## Host and reproduction boundary

The driver root is /tmp/hc074-fixed-grouped-process-20261017.GFI9jW, uid/gid 1000
and mode 0700. Existing owned direct /tmp context and authority leaves retain
their original ancestry checks. guard.sh pins boot, active supervisor PID 8839,
zero restarts and original start time, installed binary/receipt hashes, absent
active campaign/lifecycle contexts and absent diagnostic install. A private
flock serializes cohorts. The nine signed fixture files are copied as data only.

verify.ps1 reconstructs the four-file source capsule, compares current/committed
blobs, enforces exact packet inventory and hashes, binary/script provenance,
seeded raw/derived membership and each pair of fresh inspection rows. It checks
pre/post guards, independent executable-object postflight and all nine bundle
digests. The qualification manifest remains unchanged at raw SHA256
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.

No release installation, supervisor stop/restart, privileged mutation, clock
adjustment, product workload, pilot, observer or expensive Actions is performed.
These are selected nonexecuting acquisition/return cuts, not arbitrary in-syscall
death or power loss. Real fixed-install admission, authenticated original
send/start and actual manager ACK recovery, independent watchdog lifetime,
P2/P3 all-writer cessation, completion and later cells remain separate obligations.
The private one-shot timer is not an autonomous writer-revocation mechanism.
