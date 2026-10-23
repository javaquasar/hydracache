# Installed child consistency is not original authentication

The separate refusal-only --systemd-bootstrap-installed-v1 mode now checks an
asserted installed executable independently of the original sealed reference.
It compares the actual current executable with the installed inode and repeats
the complete observation. Even a matching private filesystem executable with
fully consistent forged claims exits 2 with original authentication closed.
This is a local prerequisite, not an installed or admitted production child.

The [component plan](bootstrap-installed-child-plan-20261023.md) follows the
[same-owner record materialization](bootstrap-installed-material-20261023.md).
The implementation adds diagnostic_bootstrap_installed_mode.rs,
diagnostic_bootstrap_installed_child.rs and its tests, plus the installed
executable and inventory leaves. Synthetic process controls live in
tests/support/bootstrap_installed_controls.rs under the existing current-image
integration target. Exact source/COMPAT guards extend the existing xtask target;
no dependency, Cargo target or gated-test registry entry changes.

## Two executable identities, one explicit child contract

Activation requires exactly three canonical assertions: actual current
LISTEN_PID, LISTEN_FDS=14 and the old twelve names followed by
installed-image:installed-entry. Arguments, count, unknown names, duplicates and
foreign environment entries refuse. The child slots are FD0..16, not the
parent's 33 physical descriptors: old FD0..14 retain their meanings, FD15 is the
asserted installed image and FD16 is the separate sealed HCBI0741 record.

A new bounded procfs enumerator accepts exactly these seventeen slots plus its
own temporary directory FD. It requires twenty entries including dot entries
and stops after at most twenty-one reads. The old fifteen-slot closed checker
is not relaxed or reused for the extended set. Instead, the unchanged private
fifteen-role capture runs its original predicates in their original order.

FD16 must be readonly, inherited without CLOEXEC, at zero offset, exactly 1280
bytes and protected by the four exact seals. Canonical decoding and repeated
metadata observations bracket the positional read. Its embedded HCBT0741 bytes
must equal the separately observed original FD14 exactly. Regular-role inode
aliases with either new role refuse.

FD15 must be readonly/non-O_PATH, at zero offset, a single-link regular file
with mode 0555 or 0755, nonempty and bounded by 128 MiB. Its device/inode/size and
full content hash must match the assertion. The old capture retains its ELF and
hash checks on sealed FD13; the canonical envelope requires equal content but
distinct installed and reference identities.

Fixed procfs thread-self/exe lookup must identify the same complete metadata as
FD15, not FD13. The new inspection handles are owned only by their opener and
close before the next closed enumeration. Repeated observations compare all
retained values; no inherited FD is adopted, repaired, remapped or exported.
These are sequential observations, not an atomic kernel snapshot.

## Authority remains deliberately closed

Neither a valid checksum nor a procfs type proves original custody, trusted root
ancestry, UID/GID namespace, clean signed build, protected policy or the actual
manager and guardian context. The unchanged parent still selects its fixed
root-owned source. This diagnostic leaf does not independently enforce or enroll
that root ancestry; owned non-root filesystem fixtures can therefore exercise
consistency without being granted authority.

The mode returns a permission error even after every consistency check passes;
dispatch always exits 2. It performs no start/send/exec/release, original parent
transport, child admission, writer cessation, completion or lifecycle advance.
Exact additive views preserve the whole old dispatch/mode/test bodies, original
kernel and manager/owner frames, record bytes and 300/60/10 origins/windows.

## First outcomes and corrected controls

The [first-result record](bootstrap-installed-child-first-result-20261023.json)
and immutable raw lane snapshots retain all earlier outcomes:

- The absent-reader baseline exited 101 before implementation.
- A development GNU check compiled with an unused owning inspection handle.
  Queries now use that handle explicitly; no lint suppression was introduced.
  This development result is a tool-output summary, not a raw evidence lane.
- The first Windows lane passed its controls and strict lint, then formatting
  rejected the new integration module position. Only the additive hook moved.
- The first MSRV image lane passed five controls and failed one combined fault
  control with shell exit 126 instead of reader exit 2. The old assertion ran
  before stderr collection, so the precise inner fault and errno were not saved.
  The first GNU lane passed all six corresponding process controls.
- The next Windows source guard caught an added blank line left after removing
  the relocated hook. Removing that new whitespace restores the entire previous
  integration file exactly; the source guard was not weakened.

The corrected helper collects bounded stderr before checking the exit status
and labels the particular fault. Filesystem executable controls run one test
thread to avoid overlapping fixture writers and forks. An additional controlled
open writer deterministically produces Text file busy and shell exit 126 without
entering the reader. That proves a possible infrastructure failure class, not
the cause of the first failure whose stderr is unavailable. No automatic retry,
original live rerun or change to admission windows was used.

Final controls cover canonical activation, record seals/flags/shape/offset and
same-byte substitution, bounded census parsing, executable modes and metadata,
matching filesystem execution with authentication still closed, same bytes at a
different current inode, extra/missing descriptors, changed embedded original
assertions and installed content drift. The new unit controls also verify that
refusal does not repair offsets or descriptor flags.

## Reproducible local result

The fixed [runner and retainer](../../../../tools/bootstrap-installed-child-074/README.md)
reject output overwrite and historical baseline reexecution. Final reviewed
lanes and metadata share one stable 141-file source snapshot. Initial and
intermediate lanes remain separate, including both source-contract refusals.

| Scope | Passed checks |
| --- | --- |
| Windows | 63 source contracts and 24 codec/bootstrap controls; all-target check, strict all-feature Clippy and affected-package formatting. |
| GNU 1.94 | 103 selected controls, including nine new child unit controls and four new process controls; all-target check, strict all-feature Clippy and exact selected unsigned build. |
| GNU MSRV 1.88 | The same 103 selected controls, all-target check and strict all-feature Clippy. |
| Metadata | Four Python controls, doc-check, 188 discovered/registered gates, local nonpromotable performance contract and diff checks. |

Plan commit: b680b437. Core source commit: d08b8df1. Checked source/tooling commit:
b16069261cac324d1274bcf953086ae57cc83455. Evidence commit ea48b632 retains the
exclusive [inventory](local-runs/bootstrap-installed-child-20261023/inventory.json),
SHA-256 da8b2edb4675eb6599aa3e84ab6243465cc7b51ab21c1299fdda70ae5c048fdf.
All 1800 members plus inventory were verified against 1801 exact staged Git blobs
before commit. A green consistency audit explicitly keeps original authority,
delivery, writer cessation, completion and admission false.

The selected unsigned GNU check image is 1631696 bytes, SHA-256
1007fa9e3f21db3cd217d94c3b3f03afdbc67dc6219d567bcfe5c7cd12de4d6e.
It was built from the checked worktree before commit, not from enrolled clean
signed production provenance. Neither its size nor correctness-test elapsed
time is a performance or original-deadline-fit result. The qualification
manifest remains unchanged at SHA-256
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.

## Next integration boundary

The next join must admit the particular sandbox child through the same original
owner and manager: process generation/pidfd, credentials, namespaces, cgroup,
installed executable and streams, under the existing complete frame and original
deadlines. Caller assertions and this reader's success cannot substitute for
that admission. Parent-to-child FD transport remains unopened.

Only after authenticated original delivery and all-writer cessation are joined
can completion, later cells and the new full SIGKILL/lost-ACK matrix advance.
Added hashing/inspection cost is unmeasured. No clock/service change, installation,
server action, product workload, performance cohort or original live attempt
was performed by this component.
