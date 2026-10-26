# Tokio registration ownership and the compatible security update

Tokio's declared workspace minimum and locked version move from 1.52.3 to
1.53.3. Complete lock comparison finds exactly one changed package version and
checksum; its dependencies, all other packages, feature list, Rust 1.88 MSRV
and deny policy remain unchanged. This implements the separately authorized
compatible security work, not a new performance proposal or original-start gate.
The bundled-roots candidate remains archived and unapplied.
Implementation commit: `2a4a771d48ed6ad54288fd15ea7c75b56f4bd80d`.

## Why a version change alone is insufficient

The [RustSec advisory](https://rustsec.org/advisories/RUSTSEC-2026-0386.html)
describes constructors accepting AsRawFd without enforcing descriptor ownership
or stability. The [upstream repair](https://github.com/tokio-rs/tokio/pull/8587)
deprecates those constructors and introduces an unsafe registration contract.
Deprecated constructors still exist: merely resolving the patched version does
not prove every caller obeys I/O safety.

The one direct workspace caller already passed an owned, privately cloned File
to AsyncFd::new. These tests do not reproduce a use-after-free in that old
caller, and no test deliberately violates I/O safety. The change makes its
existing ownership obligation explicit at the new API boundary.

## Ownership and unchanged operation semantics

The private register_operation_alarm helper clones the retained File once and
transfers that duplicate into AsyncFd::register. The only production consumer
keeps the result immutable and local to Operation::bound. It does not expose,
replace, leak or manually close the registered descriptor. Closing another alias
does not close this distinct descriptor number. Cancellation and return drop the
registration before closing its owned File; failed registration drops the error's
private duplicate while retaining the original. Upstream's exact registration
and Drop implementations are archived with their license and provenance.

An exact source audit restores the new bound call to the old expression, removes
only the named helper and new test-module declaration, then compares the complete
remaining channel module with the captured baseline. The kernel module is
unchanged. Both BOOTTIME reads, their order, biased alarm selection, error mapping,
non-consuming readiness, the original two-second absolute timer and every live
manager/kernel predicate remain. No 300/60/10 origin or window is renewed.

## Baseline and regression controls

The first unmodified old-version channel baseline passes nine tests, with the
same two manual system-manager cases ignored. A second old-version baseline adds
four ordinary ownership controls and the private helper still using the old API.
All four pass. The exact same test bytes then pass on the patched API on both
GNU Rust 1.94 and MSRV 1.88. A positive baseline is expected here: ownership was
already respected by this caller, unlike the generic unsound API described by
the advisory.

Each new test runs a single explicitly selected case in its own bounded synthetic
process. This keeps exact FD-number assertions independent of other libtests'
descriptor churn; the surrounding cohort retains default parallelism. No manager
connection, original-owner attempt, privileged enrollment or service is used.

- Alias lifetime: the registered duplicate stays valid after the original and
  another alias close, and closes when the registration drops.
- Cancellation and FD reuse: sixteen pending cancellations close their private
  duplicate; the same number is reused for another open file description, which
  still receives readiness. The original operation deadline stays fixed.
- Registration failure: sixteen regular-file refusals leave the original open
  and restore the exact descriptor set. No arbitrary invalid RawFd is registered.
- Unpolled cancellation, ready success and ordinary error return retain the
  source File, descriptor set and original deadline.

Existing ordinary tests separately cover actual BOOTTIME expiry, refusal before
polling expired work, retained unread alarm readiness, path/credential/pidfd/boot
checks and exact D-Bus reply shape. These controls do not prove process-crash
retirement or termination of every writer alias.

## Focused checks

| Scope | Result |
| --- | --- |
| Initial old-version channel baseline | 9 passed, 2 unchanged manual ignored |
| Old-version paired ownership baseline | 4 passed |
| GNU 1.94 patched ownership and channel cohorts | 4 and 9 passed, 2 manual ignored |
| GNU MSRV 1.88 same cohorts | 4 and 9 passed, 2 manual ignored |
| Complete ordinary Windows RESP package | 183 passed, 23 unchanged ignored |
| Windows server mTLS and HC/2 daemon process | 15 passed |
| Windows MSRV session and SPKI TLS cohort | 5 passed |
| Windows supervisor/RESP/server all-targets check and strict all-feature Clippy | Passed |
| GNU supervisor all-targets check and strict all-feature Clippy | Passed |
| Supervisor formatting and exact source/lock/policy audit | Passed |
| doc-check and local non-promotable 0.74 performance-contract | Passed |
| Independent offline packet/tamper controls | Six passed |

These overlapping cohorts are not a summed workspace milestone. The baseline
and default GNU check retain a pre-existing unused Reference-variant warning in
an authority capacity test. The all-feature strict Clippy gates pass without
silencing it, changing that test or adding an exception.

Against unchanged clean advisory DB
`84d943a6deeb8cc6c6f632531aaf4f1c62275239`, cargo deny remains advisories FAILED,
bans/licenses/sources OK. Nine findings remain: RUSTSEC-2024-0449 and
RUSTSEC-2026-0367/0368/0369/0370/0371/0372/0381/0383. Tokio's finding disappears
from this exact graph. Earlier ten-finding and archived roots-candidate
eight-finding results retain their original graphs; no combined roots/Tokio
candidate is measured or admitted. This is a pinned-DB comparison, not a fresh
all-advisory assessment or proof of all transitive callers' I/O safety.

## Release boundary and retained implementation

The [append-only evidence packet](local-runs/tokio-security-20261026/index.json)
retains every focused command/result, raw source inputs, both ownership baselines,
the unchanged oracle, original kernel code, complete graph comparison and public
upstream source/license. The active regression lives in
[diagnostic_manager_alarm_tests.rs](../../../../tools/long-run-supervisor-074/src/diagnostic_manager_alarm_tests.rs),
not only in an archive. Run the ordinary Linux gates with
`cargo test -p hydracache-long-run-supervisor-074 --lib manager_alarm_ --locked`
and the separate `--lib manager_channel_` selector. MSRV uses explicit +1.88.0.

The 246-member index is SHA256
`2f984fa20ea11841a8b69a961cbdea872238b97baa00f47fc87c70057879e83e`.
Run its `verify.py` and `test_verify.py` with Python 3.11 or later to validate
all raw objects, exact one-node lock change, unchanged paired oracle, full channel
restoration, kernel/policy/qualification identity, first verdicts and counts.
The [11-member offline-control receipt](local-runs/tokio-security-20261026/verification/index.json)
retains those commands, verifier bytes and six tamper-control results; SHA256
`cb4dfcd38181cb1793cb3e92d4dc316b4bc9f4a054e9a9be168f7ccb57489da5`.
The test-only re-pin cases exercise semantic rejection after bypassing the hash
boundary inside the unit test; they do not authorize a new archive or modify its
committed verifier pin.
The first staged Git audit checks 125 packet blobs with zero byte mismatches,
retained separately in verification/staged-git-first.json. Explicit -text
attributes preserve the sealed raw bytes instead of normalizing archived logs.

The frozen qualification manifest remains SHA256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
No V6 replay, clock/service/server change, installation, product workload or
expensive qualification occurs. Complete package/source-successor review,
remaining security repairs, authenticated release beyond ReadyObserved,
all-writer cessation, durable completion/cells 2–4, crash/lost-ACK/reconnect,
production trust and a compatible admitted host still gate release closure.
Tokio's scheduler/dependency change also needs final compatibility and native/
RESP non-regression qualification on the eventual exact C74; no performance
improvement or non-regression is inferred from these correctness tests.
