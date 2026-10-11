# Local diagnostic fixture recovery evidence

This packet covers local recovery refusal, not production enrollment, product
qualification or an exact release candidate. The
[preregistered design](../../diagnostic-fixture-recovery-local-design.md) reuses
existing state/IPC decoders and authority refusal records without a migration.

The new inspector retains the cooperative fixture host fence and verifies bounded
descriptor-relative documents twice. The original fixture signed-authority bridge
refuses interrupted/pending or invalid lifecycle state and persists refusal only
to safe matching original authority. Quiet observations cannot authorize start,
adoption or cleanup. No lifecycle document is changed or replayed.

`manifest.json` binds source trees and raw closed captures. `verify-packet.py` is
read-only: it checks raw hashes, test results, retained negative attempts and the
unchanged frozen qualification inputs. Run it from the repository with Python 3.11
or newer. Raw test counts refer to harness executions, not performance samples.

Missing-public-API failure precedes implementation. Two failed test attempts retain
the fixture ledger's wrong default 0644 mode; the fixture is corrected to 0600,
not the validator. The initial root contract also retains a rustfmt-sensitive
source marker failure, followed by a corrected structural check. Four logical
publication faults preserve the original lifecycle error: incomplete pairs refuse
reopen; a complete but unacknowledged refusal reopens refused. None models a real
power interruption or controlled production-root enrollment.

An additional Linux root-test attempt cannot resolve the Windows absolute Git
worktree pointer. Adding a process-scoped safe-directory pin alone does not fix
that path. A read-only preflight with explicit existing metadata paths resolves
HEAD, but the mixed root-test attempt is stopped before the scratch-repository
test target: a process-wide GIT_DIR would also redirect its temporary Git writes.
The final root, book and packet checks therefore use native Windows tools without
that override. No Git pointer, worktree or global Git configuration is edited;
failed and interrupted captures are retained, not counted as green evidence.

Authenticated production intent, crash before any intent, reboot lifecycle,
root-owned output creation, watchdog/cleanup and real issuer enrollment remain
open. The inherited frozen-plan digest mismatch and zero accepted product
candidates continue to close qualification and ship admission.

## Completed local checks

The packet retains 37 closed raw captures. Baseline passes 403 tests with one
existing ignored. The extended Linux scope passes 494 tests with that same
ignored case; portable Windows passes 195. Fourteen new unit tests and one new
API test are included, alongside unchanged state/IPC regressions. Three serial
recovery repetitions each pass sixteen tests: fourteen new and two existing.
The final root run passes 105 contract, 13 evidence and 23 governance tests.
Three compile-fail doctests include the new fixed-origin exclusion case.

Strict lint, affected-package all-targets/all-features checks, MSRV 1.88 checks
on Linux and Windows, Linux strict rustdoc, formatting, doc-check, local contract
and book checks pass. The verifier checks 198 local Markdown file targets and
all raw hashes, while confirming the unchanged manifest and eight frozen inputs.
`--require-ship` and the actual qualification dry-run still refuse. These checks
are not a full-workspace final-candidate receipt or a performance measurement.
