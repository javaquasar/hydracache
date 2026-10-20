# Retained sealed role file checks

Baseline is 773e6bf0. Implementation 32ec9525 adds descriptor-backed content
retention. Selected acceptance source is
82570db8589065df5711f2faf51d02406168cf79; final/checks.ps1 records exact compiler,
recipe and test executable digests before running, compares image digests after,
and requires unchanged HEAD and clean selected inputs at both ends.
Inherits docs/RULES.md and the [plan](../../held-role-files-plan-20261020.md).

## Scope and hypothesis

Checked receipt data previously compared caller-provided bytes without retaining
the files. PinnedRoles consumes that checked receipt and two already-open files,
requires exact immutable seals, read-only non-O_PATH CLOEXEC descriptors, bounded
ELF/content, signed lock/log and distinct kernel identities, and retains the
same descriptors/stamps. Every revalidation rehashes using positional reads and
becomes permanently refused after an error. It opens no path and exports no FD.

All role images here are synthetic ELF headers, with synthetic keys and invented
Cargo events. No ELF is executed by these tests. Content retention is not an
authenticated creator, original role process, protected build or live epoch.
This package performs no original join/send, watchdog arming, all-writer
revocation, SIGKILL/lost-ACK matrix, server/supervisor operation or product work.
Original 46 brackets, 300/60/10 origins and sealed preceding packets remain.

## Retained negative and preliminary results

windows-baseline.log passes the sixteen existing controls. missing-method-red.log
then records the expected missing pin_sealed_roles method before implementation.
first-green.log runs one new control, expanded-194.log runs the intermediate
25-test selection, and expanded-188.log includes all 26. They are development
checks, not selected-source receipts.

The root checks.ps1 recipe selects 3fa2d487. Its integration/image cohorts pass,
but the GNU capability-doc log contains a compiler panic in method diagnostic
rendering. Rustdoc counts that compile-fail example as successful; the root
194-receipt.json passed field therefore is not an accepted capability gate.
The complete root cohort is not accepted. multiline-doc-194.log reproduces the
panic despite E0599 and multi-line formatting. trait-doc-194.log instead checks
the absent IntoRawFd trait with E0277 and has no panic. The final recipe refuses
compiler panic text even when Cargo returns success.

The first repository strict lint rejects two source-list helper items unused by
the older bridge. Narrow test-helper annotations fix that warning without
changing old bridge bodies or allowing warnings globally. Both the failed and
corrected lint logs are retained. No runtime check/deadline is weakened.

## Selected acceptance controls

Final receipts/logs live under final/. Each GNU 1.94/MSRV 1.88 cohort runs two
complete 26-test rounds: sixteen unchanged static build-authority controls and
ten new Linux descriptor controls. They cover swaps/aliases/content mismatch,
each missing and one extra seal, writable/O_PATH/non-CLOEXEC descriptors,
ordinary files/directories/pipes/sockets, lock/log mismatch, sticky refusal,
metadata change, shared offsets and concurrent alias seeks. The same sealed
file's writable alias is tested for EPERM. Thirty-two content corruptions use
seed 0x0074_f11e on each round. Four explicit compile-fail capability examples
run per Linux compiler. Windows runs two rounds of the old sixteen tests and
two old capability docs; no Linux role behavior is claimed on Windows.

Twelve source/adapter controls reconstruct all 440 preceding runtime files after
removing one exact module hook and allowing one named source addition. Three
predecessor bridge controls, strict lint on Windows/GNU/MSRV, scoped fmt,
registry/docs and the local non-promotable performance contract are separate
gates. The full 321-test suite and preceding 27-case launcher process matrix are
not rerun by this content-only package; their sources remain byte-exact.

The final repository wrapper reaches a successful silent formatter, then refuses
because Tee-Object created no output file for its panic-text scan. That refusal is
retained in final/repository-wrapper-refusal.json, not hidden as a passing recipe.
final/complete-repository.ps1 binds the same source and preceding successful logs,
rechecks unchanged selected inputs before/after, reruns formatting and records
its explicit zero exit in repository-format-recheck.json. The repository receipt
binds both recipe digests and names this recovery; Linux/Windows receipts retain
their original completed recipe. No source changes or inferred formatter success
are used to close the missing final receipt.

verify.ps1 checks packet membership/hashes, source/recipe/image metadata, exact
test counts, seed, format recovery and false scope flags. Thirty-four in-memory receipt mutations must
refuse; they never modify packet files. The inventory is sealed only after all
four final cohort receipts exist. It is local evidence, not a production receipt
or a new wire format. Qualification manifest SHA256 remains
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.

Next consume these files with the same FixedPlacement, independently admitted
policy/controller/live epoch and original signed claims around the unchanged
complete frame; then bind actual helper/bootstrap/process identities. Protected
role receipt production, active collector, all-writer cessation, durable cells
and reconnect remain required before separately authorized server rehearsal.
