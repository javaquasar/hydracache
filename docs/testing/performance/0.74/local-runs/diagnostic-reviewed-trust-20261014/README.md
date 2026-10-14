# Reviewed diagnostic public trust verification

Implementation `fcc50c6cbf65f7746d3e398beee321de4fe817f4` reuses the reviewed
controller/builder selection from October 9. Baseline is `194585b8`;
preregistration is `e8acf3b9d213986185a4b4944bcdd93e89fe8087`.
The [design](../../diagnostic-reviewed-trust-design.md) specifies this opt-in
verification slice. It adds no production listener, installation or worker trust.

## Implemented behavior

`diagnostic_reviewed_trust::load_builder_policy` fixes the verifier and raw policy
digest outside supplied policy bytes. The existing canonical builder verifier
then authenticates receipts. KernelPacket.authenticate_reviewed_controller
requires that reviewed controller key and repository before existing kernel
sender/time authentication. Failure permanently refuses the original packet,
including generic authentication fallback. Principal allowlists remain supplied;
there is no valid production controller-request capture in this packet.

The retained hosted-build receipt signature verifies under the real previously
selected builder key. Its executable bytes are not read or executed here, so this
does not repeat actual-content verification or certify a currently installed
binary. Historical public selection is not current external revocation status.
Worker issuer, canonical production worker-policy body pin, policy/current
authority epoch and trusted host/root joins still need independent selection.

## Local checks

The Linux baseline passes 29 artifact, 22 IPC and 14 kernel checks, with one
manual inherited-sender proof skipped. Reviewed-source focused tests pass seven
portable trust checks plus 15 kernel checks in each of three repeats. The new
kernel check rejects wrong key, wrong repository and a fixture-signed request
under the reviewed key, then verifies sticky refusal across both methods.

The portable tests reject every one-bit-per-byte mutation of the 302-byte policy,
explicit key/schema/repository substitutions, malformed/budget/noncanonical
inputs and 128 signature-nibble mutations with recomputed receipt digests.
Eight independent parallel verifiers keep their own checked policy guards.
Linux artifact/IPC/Unix regression passes 60 checks; 58 doctests pass. Windows
passes seven trust, 15 artifact and 19 IPC checks. Current/MSRV all-target,
all-feature checks pass on both systems; current strict Clippy passes on both,
and MSRV strict Clippy passes on Windows. Linux MSRV Clippy is unavailable.

Root registry/performance/evidence suites pass 6, 124 and 13 checks. Format,
doc-check, local performance contract, documentation links and book build pass.
Require-ship returns the expected refusal while candidate/qualification remain
incomplete. No full release milestone or manual crash suite is claimed here.

## Retained development negatives

The first missing-API red capture also called a private canonical helper from an
integration test. The harness was corrected, and the second red capture isolates
the absent reviewed-trust module. Final code uses the existing public canonical
receipt serializer; no production visibility was broadened.

The first Linux gate sequence ends at unavailable Rust 1.88 Clippy. Earlier
current checks/lint and MSRV check succeeded; the separate final sequence repeats
available current lint and MSRV check successfully without installing components.

The first root invocation guessed nonexistent `gated_test_registry` and
`release_evidence_074` targets and executed no tests. Corrected targets pass.
An over-specific `v074` filter subsequently selected zero release-evidence tests;
that is not a coverage result. The complete 13-test target was then run and passed.
Raw failures and the empty selection are retained rather than removed.

## Integrity and remaining gates

The [manifest](manifest.json) pins raw captures, reviewed source, historical trust
evidence and frozen inputs. The [verifier](verify_packet.py) is read-only:

```powershell
C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-reviewed-trust-20261014/verify_packet.py
```

It checks byte integrity and fixed public selection, not current GitHub secret
availability, authority status, root enrollment or independent cryptographic
implementation. Product/Cargo inputs and qualification digest remain unchanged.
The [pre-server checklist](../../pre-server-closure-checklist-20261014.md) retains
worker trust, authenticated original start, watchdog/writer revocation,
completion/cells two through four and exact-host rehearsal before execution.
