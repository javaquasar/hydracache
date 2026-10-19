# Original root guardian owner retirement

Source `6098ab02fc5b7ed23cc5fa80e1ce221c8edbbbce` closes the normal-Drop
owner-reuse gap audited in baseline `f48fd4bf`. The immutable baseline packet
is [creator and frame evidence](../root-creator-20261019/README.md); its inventory
SHA256 is `8e58adb3fdca5fb83af98e27169d5e08aaa65438843c2f414d57bae5ab771161`.
Its local creator/kernel/frame passes do not supply an original retirement proof.

## Change and scope

Wrapper Drop now calls one retirement function before field release. It refuses
the original owner first, then kernel controls. Consequently returning the
exclusive borrow cannot reopen that original owner. Pending targetless field
cleanup remains exact. Possibly enrolled independent guardians are not killed
or unfenced by this destructor; peer closure requests their existing revocation.
No timer read/rearm, origin refresh, completion, unlock or execution API is added.
This gate concerns normal wrapper retirement, not a production durable global
enrollment/replay slot or arbitrary suppression of Rust Drop.

The failing `red-missing-drop.log` is a source regression for the absent Drop
implementation, not a claim of an executed production replay. Unit controls
exercise the actual refusal function and its ordering for pending and consumed
states. The original-frame probe exercises the same function with unchanged
original predicates, then creates fresh simulated controls and requires retry
to fail without authentication, signature or channel work. Reusing the old
refused controls would not isolate whether the owner itself was retired.
Actual kernel refusal and creator SIGKILL remain separately exercised gates.
The combined privileged original constructor and whole-writer confinement remain
unproved; production host admission, start, completion and fence release stay
closed.

## Reproduction and evidence

`run.ps1` records exact source, recipe and image identity. Build stages require
UID1000 and run strict current/MSRV checks, the retirement unit, old join units,
one twelve-case actual creator matrix and one four-case real kernel adapter
matrix. Frame stages run sequentially using the previously authorized temporary
local WSL timesyncd stop/restore, a settling observation and strict clock probe
before creating attempts. They run the retirement frame three times and the old
eight-case original frame once per compiler. Bundle inputs are hashed before
and after as nonexecuted fixture data. No server, supervisor or product is used.

Final build cohorts are `gnu194-quiescent` and `gnu188-confirmed`. Earlier
`gnu194-selected` / `gnu188-selected` build receipts bind implementation
`5651b12a`, before the frame regression was strengthened to fresh controls;
they contain no final frame receipt and are excluded from acceptance.
The original source-only red failure is retained. No existing historical packet
is changed or resealed.

## Negative GNU frame disposition

Both GNU `gnu194-confirmed` and the same-source quiescent retry fail at unchanged
output preparation with `Causal(Time)`, before retirement is invoked. Their
trace reports have 1,365 authentication/signature calls and no successful
retirement marker. The failure path can represent boot identity drift, BOOTTIME
moving before the issued point or challenge age reaching the unchanged ten
seconds. The issued/now pair was not captured, so no one cause is asserted.
The quiescent preflight separately observed CLOCK_REALTIME moving backwards
6,134,508,528 ns before any admission; this is not proof of the earlier
BOOTTIME failure's cause. Clock services were restored by the runner.

The positive `verify.ps1` requirements remain unchanged and cannot accept these
GNU frames. A separate `verify-negative.ps1` / `seal-negative.ps1` records the
mixed outcome and requires `retirement_frame_gate_passed=false`. Successful
unit, source and concrete-kernel checks do not waive the failed full-frame gate.
The next diagnostic must attribute the original causal window on GNU without
renewing its issuance or loosening the ten-second limit; no further identical
GNU retry is part of this packet.

## Final mixed result

GNU current and MSRV both pass the retirement unit, three existing join units,
twelve real creator SIGKILL smoke cases and four real kernel adapter cases at
the final source. The complete source suite passes 292/292 and preserves all
400 previous supervisor Rust bodies through exact adapters. Windows retains
54 ordinary and 18 compile-fail passes; affected strict lint, format, registry
and local non-promotable performance checks pass.

MSRV passes three independent retirement-frame repetitions and the old
eight-case original-frame regression. Each retirement frame checks 68/68
authentication/signature calls and four channel checks, then rejection through
fresh controls with zero additional auth/signature/channel calls. Its source,
image, recipe and bundle identities remain unchanged, and timesyncd is restored
to active. GNU has no passing retirement-frame receipt. Overall acceptance is
therefore false, despite the passing MSRV callback proof and concrete controls.

The first positive preflight was accidentally invoked while the last MSRV log
was still open and refused a file-sharing error; it is retained but is not a
functional gate result. The final positive preflight runs after all writers
close and rejects the missing GNU acceptance receipt. The negative verifier
records this distinction rather than changing the positive requirements.

The unchanged positive verifier remains intentionally red on missing GNU frame
acceptance. `verify-negative.ps1 -Preflight -SelfTest` validates the mixed gate
disposition and raw integrity without publishing an inventory.
`seal-negative.ps1` publishes it only once; subsequent negative verification
hashes raw bytes, including CRLF receipts and native logs. Ten metadata negative
controls also prevent promoting the failed frame flag. The pinned baseline
inventory cannot drift. Full source, Windows, lint, format, registry and local
non-promotable performance checks are separate receipts. Runtime fields and
original deadlines are unchanged; functional counts are not performance claims.

## Files

`diagnostic_original_root_guardians.rs` gains only the retirement function,
Drop implementation and test registration. New ordinary and full-frame tests
are `diagnostic_original_root_retirement_tests.rs` and
`diagnostic_original_root_retirement_frame_tests.rs`. The original probe API and
frame test gain exact additive hooks. The new retirement source guard preserves
all 400 prior supervisor Rust files through exact adapters, the registry gains
one manual gate, and `root-retirement-contract.toml` keeps admission closed.
The plan, collector design, performance README and optimization article explain
the distinction between per-pair consumption and retiring the original owner.
