# Local root creator death and original frame controls

The selected source is `f48fd4bf1de242d4130cd61459a3daed8437f33e`, compared with
`4e940734512960999000245384857acadf9bb2f0`. This is local functional evidence,
not server admission, a product benchmark or proof of all-writer cessation.

## Implementation

`root_creator_sigkill.py` is an external owning subreaper. It execs the real
ignored Rust creator, authenticates the test-only carrier and receives two
exact guardian pidfds. It kills the creator with its owned pidfd, requires its
actual SIGKILL exit status and verifies that the same guardians are adopted by
the harness. Cleanup signals/reaps only owned targets and adopted libtest
children, restores the previous subreaper setting and removes only fresh
fixture directories. No service, installed bundle or system-bus send is used.

Five cuts cover first ACK read, second ACK read, local armed assignment, API
return and both ACKs queued but unread. At queued-ACK cuts a test-only MSG_PEEK
checks exact Armed bytes without consuming them. Real `notice()` credential and
ancillary checks remain unchanged. Other cases kill or stop one guardian,
stop the second before its ACK, and retain both-dead/both-stopped negatives.
Direct and nested controlled writers must be alive before creator SIGKILL;
positive revocation must precede the unchanged fixture alarm. Each positive
case admits four later waves. The parent has closed every original fence OFD;
only guardians can retain its flock after creator death. An outside pipe alias
must block EOF until the harness itself closes that alias. Common-mode controls
sample the remaining live writer and fence states; they are not new expiry
claims and do not erase the older expiry negatives.

`OriginalRootGuardians` is crate-private and consumes the opaque original
context. The original owner derives fence, timer, deadline and context binding.
Preparation and enrollment each retain two full rounds; revalidation retains
one. Failed opening/closing checks poison the original owner and kernel controls;
duplicate or uncertain enrollment cannot retry. Construction failure cancels
only targetless children before retaining the refused original borrow.

The original nonroot frame gate deliberately uses a distinct simulated kernel,
while the root-only adapter gate uses actual cgroup2 and SIGKILL. The former
tests real original signatures/predicates/channel probe counts, not an actual
privileged manager channel. Neither gate constructs a production trust decision.
The pinned root still needs protected host-context admission and a combined
privileged original fixture before the original production path is usable.
Holder pidfd identity does not prove that all holders and descriptor copies
are confined to that root; the outside-writer negative must still block release.
Held-unit start, completion, fence release and server execution remain closed.

## Cohorts and raw evidence

`run.ps1` separates build/kernel and original-frame stages. It refuses source,
image or recipe drift, zero-test selection, existing fixture roots and output
overwrite. Builds use WSL UID1000; only fresh cgroup fixtures use local root.
The frame stage uses the previously authorized local WSL timesyncd stop/restore,
observes clock settling and a strict probe before creating any attempt, and
does not change a clock or an admission origin. It restores and verifies the
exact service in `finally`. Windows clocks and the dedicated server are untouched.
The nine retained bundle inputs are hashed before and after use as nonexecuted
test data; no installed-origin or production-key claim follows.

The acceptance verifier requires GNU 1.94 and MSRV 1.88 cohorts named
`gnu194-confirmed` and `gnu188-confirmed`, each with three twelve-case creator
matrices, three four-case real adapter matrices and three eight-case original
frame matrices. It also requires the old sixteen-case pair, twelve-case
transport and eleven-case kernel matrices three times per compiler, the old
seventeen-case guardian controls and the original Pending frame controls.
Complete source, Windows ordinary/compile-fail and carrier refusal checks are
separate logs. `inventory.json` hashes raw bytes; no BOM, newline or whitespace
normalization is permitted. Native raw-log trailing spaces are explicitly
allowed by this packet's pre-seal attributes, not trimmed from evidence.
`verify.ps1 -SelfTest` rejects metadata/hash/path/scope mutations. Closed packets
must never be edited or resealed.

## Preliminary results excluded from acceptance

- `red-baseline.log` records the old image selecting zero creator tests and
  failing the carrier handshake. This is a coverage gap, not a revocation bug.
- Early kernel screens precede the final live-before-kill and queued-byte
  assertions. They are not substitutes for source-bound selected cohorts.
- `frame-screen.log` and `kernel-adapter-screen.log` selected zero tests from an
  older image. They establish no successful scenario.
- `frame-corrected-screen.log` fails before the new join, at unchanged
  `Causal(Time)` during output preparation. Its trace records 1,399 authentication
  and signature calls before refusal. The failure alone does not distinguish
  clock movement from elapsed preparation cost; it is not converted into success.
- The new healthy frame scenario intentionally tries duplicate enrollment after
  its successful checks. `c1799824` corrects its outer expected state to refusal;
  runtime checks and deadlines are unchanged.
- `gnu194-selected` contains preliminary root checks but no accepted build
  receipt: the runner correctly refused source drift while the above test
  corrections were committed. It is not a final cohort.
- Earlier check/unit/lint screens retain development compile/visibility errors.
  `source-suite-confirmed.log` failed Windows linking while the preceding
  suite still owned its executable. It ran no tests; the final suite uses a
  fresh log after the first process exited.

## Confirmed results and retirement boundary

Both selected compiler cohorts completed with source/image/recipe identity,
unchanged bundle inputs, zero remaining fixture roots and restored active local
timesyncd. Each compiler passed three repetitions of every new matrix:

| Control | Combined selected executions |
| --- | ---: |
| Actual creator SIGKILL scenarios | 72 |
| Real root kernel adapter scenarios | 24 |
| Original full-frame simulated-kernel scenarios | 48 |
| Prior independent pair scenarios | 96 |
| Prior descriptor transport scenarios | 72 |
| Prior kernel scenarios | 66 |

The last row includes 24 old kill-caller SIGKILL cuts, distinct from the creator
cuts. The prior seventeen-case guardian matrix passed once per compiler and the
ten-case original Pending frame matrix passed once per compiler. The final
source suite passed 289/289; Windows passed 54 ordinary and 18 compile-fail
controls. Strict GNU current/MSRV and affected Windows lint, format, registry
and local non-promotable performance-contract checks passed. Nine cut-carrier
controls include malformed/truncated/foreign/extra-right negatives.

Healthy original join traces retain 68 authentication calls, 68 signature calls
and four channel checks; revalidation retains 34/34/two. These are validation
counts in the frame probe, not throughput or allocation measurements and not
a dynamic production constructor proof.

Final source audit identifies a separate retirement gap at this baseline:
ordinary Drop of the new original wrapper closes guardian channels but does not
itself refuse the borrowed original owner. Per-pair one-shot accounting alone
cannot exclude a fresh preparation after that borrow returns. This packet
therefore does not qualify original-owner retirement or production readiness;
a follow-up must poison the owner on capsule retirement and test retry refusal.
All above local counts remain scoped to their actual gates.

## Source commits

`b617254b` records the plan; `52d805da` adds the external creator harness;
`5468524b` adds the opaque original frame/kernel adapter; `c1799824` fixes the
test's expected duplicate-refusal state; `f48fd4bf` verifies queued Armed bytes.
Evidence and explanatory documentation are committed separately afterwards.

## Changed files

The supervisor implementation and tests are under
`tools/long-run-supervisor-074/src/`:

- `diagnostic_root_creator_tests.rs` and
  `../tests/root_creator_sigkill.py` provide the external creator cuts.
- `diagnostic_original_root_guardians.rs` and
  `diagnostic_root_guardian_original_kernel.rs` provide the typed join and real
  kernel adapter; four new root-guardian test/API files cover binding,
  full-frame controls and kernel faults.
- Existing root/context/pending test modules gain only exact additive
  registration hooks. The existing root pair gains cfg-test cut hooks; its raw
  post-clone implementation and old guardian protocols stay unchanged.

`crates/xtask/tests/support/performance_contract_074_root_creator.rs` and
`performance_contract_074_original_root_join.rs` preserve the old source bodies
through exact additive adapters. Their parent guard gains only registration
and adapter delegation. `docs/COMPAT.md`, `docs/testing/gated-test-registry.toml`,
the two new scoped contracts, collector design, performance README, the
intermediate plan and article record the gates and limits. This evidence
directory retains recipes and raw results separately from source changes.
