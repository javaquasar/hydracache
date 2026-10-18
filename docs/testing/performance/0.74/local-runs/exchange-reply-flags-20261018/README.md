# Dispatch reply interoperability and ownership audit

Source `2076b5882a5ea47da4e426c7b3964f0a4c3f8b2b` corrects one private
METHOD_RETURN flag predicate on exact parent
`ff665c3b765ce0db61d54eb28d8e9e99a7d9b78e`. That parent preregisters the
[systemd and supervisor ownership audit](../../supervisor-boundary-audit-20261018.md)
and selected protocol correction. All execution is local, ordinary nonroot
WSL/Windows correctness testing. There is no server connection, release install,
service operation, original message send/start, product load or qualification.

## Baseline and failing test

Both original images from the manager-channel packet pass fourteen exchange
units. Baseline logs retain those results; the earlier packet binds their source
and image hashes. The added typed-header positive case then fails on Rust 1.94
with exit 101 and `bits=1 error=Reply`. The red image hash, Cargo artifact output,
test-registration patch and raw failure remain separate. Reconstruct its test
source from the accepted test capsule, registration patch and unchanged parent
runtime. This is a retained negative, not another accepted execution cohort.

The real-bus observation attributing METHOD_RETURN flags=1 is in the preceding
[manager-channel packet](../manager-channel-20261018/README.md), not a new
system-bus mutation here. D-Bus assigns NO_REPLY_EXPECTED no semantics on replies.
The correction admits exactly 0 or that bit on METHOD_RETURN; requests still
require zero flags, and ERROR classification stays unchanged. Wrong serial,
sender, destination, body, FDs, extra headers or closed reply window still refuse.
Unknown outcome cannot retry and a job path remains observation data only.

## Selected local gate

Rust 1.94 and MSRV 1.88 each pass nineteen exchange tests: the original fourteen
and five added exhaustive/cross-refusal tests. The new corpus covers all eight
known-bit combinations on returns/errors, all seven nonzero request forms,
correlation/body/FD substitutions under both permitted return forms, and early,
duplicate and lost-ack replies. Existing actual local SCM_RIGHTS tests and the
hidden-message-alias EOF control remain green; they are not original systemd
start or all-writer cessation proofs.

Additional filters pass nine manager units, seventeen descriptor-subtree units
and thirty-two storage/fence units per compiler. The seventeen include the same
nine manager units again: these overlapping filters are not fifty-eight distinct
tests. Both compilers pass affected all-target checks and strict all-feature
Clippy. Windows passes forty-four portable units, affected checks and strict
Clippy. Formatting, all 235 source contracts, registry/docs and the local
non-promotable performance contract pass. No full workspace milestone gate or
performance measurement is claimed for this isolated correction.

The first source-contract run retains 233 passes/two failures: its new exact
restoration string expected the flag expression on a different line from
rustfmt's output. Correcting that string changes no runtime predicate, canary or
old reference. The fresh complete 235-test run passes; the first log is retained
as negative evidence, not replaced by selected passing rows.

## Reproduction and scope

Run `run.ps1 -Stage Baseline` only with the prior packet's exact original images
still present, or rebuild its exact source into an isolated cache. Red and
Candidate stages discover their images through Cargo compiler-artifact output,
not a guessed filename; each stage checks image hashes before and after tests.
For the red stage restore the parent runtime plus accepted new tests/registration.
Never run that stage against the corrected runtime and call it a red baseline.
GNU check/Clippy commands and their completion markers are retained in the
compiler logs. `check-local.ps1` runs format, registry/docs and the local contract.

The manifest binds six full source capsules, 334 top-level package Rust/Cargo
inputs, exact candidate image hashes and the closed packet file inventory.
Unchanged assets inherit the exact parent; dependency sources and compiler/host
environment are not inventoried. This is source/image attribution, not a
bit-reproducible-build or signed installation claim. Binaries are not committed.
`verify.ps1 -SelfTest` reads files/Git only and checks seven negative receipt
controls. It neither starts processes nor contacts a host. The qualification
manifest's raw digest stays `11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

This accepts the reply correction only. Original admitted-host/frame/channel
joining, uniquely addressed original message/FD delivery, active watchdog and
all-writer revocation, durable completion/cells two through four, integrated
disconnect/attach, real trust enrollment and server rehearsal remain open.
