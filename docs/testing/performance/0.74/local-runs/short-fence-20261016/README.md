# Short host-fence lifetime: local causal evidence

Source: `5d536ed073e1f8dbd8d9298352d33ec618ea0914`.
Baseline runtime: `166c3d1a6c0fef770e5ee17174b79a0a14181a78`.
[Preregistration](../../diagnostic-short-fence-plan-20261016.md) was committed as
`e02d9037`; the private test-borrow clarification was committed as `a6b14ce0`
after the initial type errors, before a successful candidate run.
No server, Actions, product workload, release qualification or push is part of
this packet. Local debug tests are not product-performance measurements.

## What was proved and changed

A completed short host-root transaction could leave its flock held through an
inherited pre-exec child descriptor. The private raw File return relied on close,
not explicit unlock. The [Linux flock semantics](https://man7.org/linux/man-pages/man2/flock.2.html)
associate that lock with the shared open file description. CLOEXEC does not
remove the controlled fork alias while it remains before exec.

The raw-close negative control passes on the baseline: parent close leaves Busy
until the child explicitly unlocks. Three new regressions fail on that baseline
runtime with the private development checkpoint/tests overlaid: successful short
drop, admission error and real corrupt-ledger recovery. The last reaches variant
zero and fails Host(Busy); later baseline variants were not observed. These
development receipts do not attest a clean baseline build or an exact archived
development overlay. The final source is pinned separately.

A private non-cloneable ShortHostFence now wraps only the successfully acquired
File, before fallible admission. Drop explicitly unlocks that original description.
No marker or persistent file is deleted. Unlock is best effort, not a durable
release receipt. Production gets no Deref, descriptor extraction or accessor.
A borrowed accessor exists only for Linux private tests. Exact-source static
restoration preserves all original predicates and the old test assertions.

The final four controls pass on both Linux compilers. All seven corrupt/pending
ledger variants refuse the transaction and then recover original revision one
while the child retains the old descriptor; file bytes/inodes/modes stay exact,
and backend calls remain zero. Another control protects the independently opened
next owner from old-child late unlock/close and failed contenders. Readiness uses
pipe acknowledgements, not a sleep. A bounded harness poll and owned pidfd/reaping
cleanup cannot authorize production progress; child code uses raw async-signal-safe
operations and targets no unrelated process.

This proves and fixes a reachable defect class. Neither historical natural
Host(Busy) capture retained its kernel lock owner's PID/fd, so neither occurrence
is conclusively attributed. Raw File construction-error paths in
HostExecutionClaim acquire/recover are deliberately unchanged and require a
separate audit. A cooperative short-fence release is not all-writer cessation,
authenticated original-start, completion or admission to later lifecycle cells.

## Retained command outcomes

| Final check | Rust 1.94 | Rust 1.88 |
| --- | ---: | ---: |
| Linux host unit group (includes four new controls) | 13 pass | 13 pass |
| Linux fixed-admission scan / output preparation / proof reuse | 4 / 14 / 5 pass | 4 / 14 / 5 pass |
| Linux IPC / lease / original fence / host integrations | 24 / 20 / 5 / 6 pass | 24 / 20 / 5 / 6 pass |
| Linux original-input ordinary regression | 99 pass, 4 ignored | 99 pass, 4 ignored |
| Windows portable IPC / lease / host integrations | 21 / 18 / 5 pass | 21 / 18 / 5 pass |
| Affected-package all-target/all-feature check and strict Clippy, Linux + Windows | all pass | all pass |

The four ignored original-input cases are existing registered manual fixtures.
No old integration test is skipped, serialized or retried. Final original-input
runs are sequential with respect to this matrix's builds/formatting, with their
normal internal test parallelism. Their elapsed times are not paired estimates.

The complete final static run passes 178 contracts; expected caught-panic canaries
can print panic text under --nocapture without a failed test. Formatting, docs,
registry (176 discovered gated tests, all registered) and the local performance
contract pass. Ship admission fails as expected and remains closed. Four final
documentation/registry/contract receipts follow the documentation edits.

Development outcomes are retained rather than replaced:

- `red.log`: one passing raw-close control, three causal failures.
- `type-development-red.log`: E0308/E0277 from two pre-existing private test
  consumers of the old return type, before the separately committed clarification.
- `green.log`: first four-control green development receipt; final matrix uses
  the source-pinned host-unit logs after the remaining child-flag/format refinements.
- `static-development.log`: 176 pass / two failures from exact-source adapters
  needed by the old static gates. Its captured body is truncated.
- `static-truncated.log`: 178 pass, but an explicitly truncated capture.
  `static.log` is the subsequent complete 178-pass capture. Neither truncated
  output is represented as a complete transcript.

Full workspace/release verify, the separate kernel suite, four-case manual causal
proof and compile-fail suite were not rerun in this slice. No new SIGKILL matrix,
binary/clean-build attestation, throughput/CPU/allocation claim or timing screen
was produced. The previous five-pair proof-reuse timings remain evidence only for
their pinned source cca2cb68, not qualification of this composed source.

## Offline verification

From this repository or its parent:

```powershell
& C:/Workspace/prj/jq/cashe/hydracache-074/docs/testing/performance/0.74/local-runs/short-fence-20261016/verify.ps1
```

The verifier checks source/tree/preregistration, the nine-path baseline delta,
eight selected source files, frozen inputs, command outcomes/counts and artifact
hashes. The manifest hashes every packet file except itself. This detects drift
relative to the retained manifest; unsigned logs and a self-hashed local packet
are not independent binary, execution or release attestation. It does not build,
execute a product, contact a host or change admission.
