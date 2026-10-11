# Fixed authority journal local evidence

This packet verifies the separate fixed-path backend and the shared append-only
engine. It is local safety evidence, not root-path enrollment, product performance,
production start permission or release qualification.

Preregistration is `a39968e86fbcabcf511f094299e04ec7e1421932`; initial implementation
is `69bf28de5598958b3442c002e8db7cbd903ebfc4`. Named-root review is
`7fc627df7b7de0569d912fc68b613c403cd4ceb9`. The manifest binds the final reviewed
source/tree and each retained raw log by exact byte length and SHA-256.

The closed packet contains 51 raw logs. Final scoped Linux checks pass 419 tests
with one existing ignored; portable Windows package checks pass 195 tests.
Root contract/evidence/governance suites pass 103/13/23. Nine new Linux unit tests
and the existing fourteen journal tests pass in three serial repetitions; the
new exported Linux API test also passes. Strict lint, affected all-target MSRV
1.88 checks, rustdoc with denied warnings, formatting, local contract, docs and
book checks pass. A read-only link check validates 184 local targets. These are
local executions, not a new full-workspace or exact-candidate release receipt.

The missing exported API first fails to compile. A later regression reproduces
named-root replacement hidden by an original descriptor; the fix compares the
current named root too. The model replaces a nonroot temporary root, not `/` or
an actual host mount namespace. All filesystem mutations remain inside owned
temporary fixtures. No issuer, installer, server or product route is changed.

An existing same-byte rewrite test fails in the second review repetition:
filesystem clock granularity can leave timestamps unchanged. Its setup now
forces an observable mtime change and asserts stamp drift. No runtime predicate
is weakened, no sleep or retry is added, and no transient/root-owner tampering
detection is claimed. Five focused executions and three fresh serial executions
follow the fix; the original red is retained, not substituted or deleted.

`serial-linux-.log` retains only the last output of an earlier shell loop whose
filename lost its counter across the Windows/WSL boundary. The first two outputs
were overwritten before retention; that loop is not evidence for three repeats.
The later repetitions use three explicit filenames. Initial and intermediate
captures are distinguished from final reviewed checks in the manifest.

Run `python verify-packet.py` from this directory or pass its repository path.
The read-only verifier checks raw bytes, Git trees, final test totals, the new
test names in each repetition, red/green findings and all eight unchanged frozen
qualification inputs. The inherited plan digest mismatch remains an expected
qualification refusal, not a successful qualification dry-run.

Remaining gates are original signed-guard durable composition, uncertain
publication/restart lifecycle, independently enrolled real issuer/pins, controlled
positive root-path checks, physical power-loss behavior, production preparation,
watchdog/writer revocation, cleanup and authenticated original start. Root UID/GID
and sequential ancestry observations do not prove initial-host namespace authority.
The v1 format, bounds, admission booleans and frozen thresholds are unchanged.
