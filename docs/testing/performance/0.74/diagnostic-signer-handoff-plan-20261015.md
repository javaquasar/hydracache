# One shot synthetic handoff in the original rooted child

Baseline: `d5d502d570c7ec062e911617f143b74992bc5742`. This local-only
hypothesis tests bounded transfer after original exec, exact 64-instruction TSYNC
installation and ND0. Actual tracer loss is a subsequent isolated hypothesis.
No key, OIDC token, issuer/job enrollment, worker authority, server operation,
product workload or qualification is permitted by this fixture.

## Channel ownership and fixed protocol

Create one unnamed AF_UNIX SOCK_SEQPACKET pair inside the original owner before
fork, with NONBLOCK and CLOEXEC. Only the original child inherits its endpoint at
FD3 across exec; the parent closes its child endpoint immediately after fork.
Duplicate the selected root/executable above FD3 only for this profile, close all
other descriptors in the child, and retain the original endpoint inode. Require
exact stdio plus FD3 before/after exec and installation. Verify FD3 identity before
ND0; afterwards the original trace permits only the fixed protocol syscall
sequence, so a descriptor replacement cannot run between checks.

Do not infer child PID from SO_PEERCRED: the pair was created before fork. Do not
accept a PID, command, path, FD, credential assertion or readiness bit from a
caller. The original owned trace authenticates each native syscall entry/exit;
the private channel carries bytes, not authority. The temporary same-UID runtime
root and trusted tracer remain fixture assumptions, not production custody.

Messages are exactly 16 bytes: `HC074-READY-v001`, `HC074-DUMMY-v001`, and
`HC074-ACK---v001`. The dummy is public test data, never a substitute production
credential. At readiness require native sendto(FD3, stack bytes,16,MSG_NOSIGNAL,0,0),
exact bytes and exit16, then receive the matching packet without retry. Require
the next original syscall to be recvfrom(FD3, stack buffer,16,MSG_TRUNC,0,0) before
sending any dummy. Recheck original context/mappings, latch the sole send attempt
before dispatch, require send16 and original recv exit16 plus exact received
stack bytes. A lost acknowledgement cannot trigger resend or replacement.

Require ACK through the same bounded send/receive sequence, then close(FD3) with
zero exit, exact stdio-only live inventory, exit_group(0) and original zero terminal
status. No other post-install syscall, ancillary FD operation, duplicate READY/ACK,
short/oversized record or wrong syscall/FD/flags/pointer is accepted. Parent reads
use MSG_DONTWAIT|MSG_TRUNC and 16-byte storage; oversized lengths refuse. Empty
queue is checked before the sole send. There is no socket pathname, listener,
credential consumer, public API or normal-library symbol.

## Files and falsification

Add private `diagnostic_signer_handoff_fixture.rs`, `_helper.rs`, and `_tests.rs`.
Reuse the original rooted owner, runtime materializer, policy and maps decoder;
add only a closed handoff selector, optional endpoint, profile-specific FD closure
and pre-install channel-syscall refusal. Factor the fixed helper compiler so the
old helper source/profile/remap/digest remain identical. Freeze all existing
bounds (512-byte policy,256 startup pairs,65536-byte/128-record maps,5-second
stall,4 images/directories,512-byte/3-read/8-record FD inventory).

Capture namespace13/rooted11 baseline and a missing-implementation red first.
Tests cover withheld dummy at initial/rooted/exec/installed/readiness stops;
healthy exactly-one delivery; wrong READY/ACK, duplicate READY/ACK,15/17-byte
READY, wrong FD, early channel use, leaked FD, exit/close, lost ACK, state replay,
foreign endpoint substitution, corrupted original context, actual owned SIGKILL
and Drop at installed/ready/delivered stops. Reject a foreign endpoint by the
retained original socket identity rather than treating bytes as a peer claim.
Use seed0x07404a11 and four independent single-thread owners. Actual tracer
SIGKILL/EXITKILL and authority-bearing orphan proof remain separate and false.

Run three focused repeats, old99 regression cases, strict Rust1.94 checks/lint,
MSRV1.88, Windows portable9/check/lint, helper lint, scoped formatting, source
contracts, docs/gates, normal-library exclusion, local contract and expected-red
require-ship. Register only this test wire format in COMPAT; keep all production
admission false. Keep source and evidence/article commits separate. New immutable
raw packets start with `* -text`; verify source/frozen digests and committed Git
bytes, including negative captures, before nonforce feature push. Old packets,
pins, workloads, thresholds and qualification inputs are untouched.
