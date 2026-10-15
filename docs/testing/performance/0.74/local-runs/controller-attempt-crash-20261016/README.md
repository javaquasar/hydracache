# Original controller attempt process loss evidence

This packet tests the [original controller attempt](../../diagnostic-controller-attempt-design.md)
under actual owned-process SIGKILL, rather than inferring process loss from
logical callbacks or the legacy signed-intent harness. It follows the
[preregistration](../../diagnostic-controller-attempt-crash-plan-20261016.md)
and [local contract](../../diagnostic-controller-attempt-crash-contract.toml).
Only the local joined process-loss boundary can close; server execution remains
unadmitted.

## What the matrix checks

Seed `0x0740c095` orders eight existing write checkpoints and AfterReturn. Each
run contains nine exact owned-child pidfd SIGKILL cases, one uninterrupted
AfterReturn control and twenty fresh inspections. The original signed request,
OIDC/build/worker/kernel/authority chain and uninterrupted common fence remain
owned at the checkpoint. Its kernel sender is the fixture child itself, not an
independent production controller. Keys and public configuration roots are
synthetic fixtures; no real private issuer key or bearer token is retained.

The common flock is busy before termination and available after signal-9 wait.
Created retains an empty exclusive file; PrefixWritten retains incomplete JSON;
later cuts retain strict canonical data and switch from mode 0600 to 0400 at
sealing. Complete data matches original configuration digests, builder policy/
receipt and active authority revision/epoch/policy. Exact fixed-leaf inventory
contains no lifecycle record, campaign marker or durable refusal.

Two separately started readers accept only BeforeCreate as empty and unadmitted.
All retained partial or complete context returns fixed-leaf Conflict without
adoption, repair, deletion, retry or acknowledgement reconstruction. Inventory
comparisons preserve bytes, inode, UID/GID, mode, links, size and mtime/ctime;
read atime is excluded. Healthy control disposes only its private trust fixture
roots while preserving fixed intent and parent-owned authority. The parent
removes its own temporary fixtures after inspection, not through a recovered
admission route.

This is not power-loss, reboot, global nonce consumption or rollback evidence.
The runtime API, authority opener, storage engine, durable schema, product code
and qualification manifest are unchanged. Authenticated original-start,
all-writer cessation, owned lifecycle progression and real enrollment remain
open. No server, workflow, actual signing credential, expensive product workload,
qualification or push is included. No allocation or throughput measurement is
claimed.

## Baseline and retained negatives

Baseline `2616276735c16e527640cab98350c1815b7bd1c6` passes all ten existing
controller-attempt cases. The test-first manual gate fails with E0425 for its
missing harness function. [test-first.patch](test-first.patch) applies to that
exact baseline in an isolated temporary Git index without modifying the real
index or worktree.

The first development matrix fails before its checkpoint because a nested
TMPDIR violates the existing authority fixture's strict direct-/tmp origin.
The corrected private helper supplies a separate parent-owned permitted leaf;
the runtime opener and default helper remain unchanged. That development matrix
then passes, before the final extra authority/output-budget assertions.

The first Linux static contract run passes 164 and fails four Git-dependent
checks. A direct diagnostic shows native Linux Git cannot resolve this Windows
worktree's C:/ gitdir pointer. The final Git-dependent checks run with native
Windows Git, without changing repository metadata or dropping assertions.
Windows whole-workspace formatting also fails before execution with os error
206, its command-line length limit. Full workspace formatting and explicit new
module checks run in WSL instead, without reducing formatting coverage.

The first full original-input regression passes 68, fails one and ignores the
new manual case. Its failure is Kernel(Invalid) during bind_reviewed_build in
original fixture construction, before the new attempt or fault checkpoint.
The timesync journal records WSL startup synchronization, but does not establish
the cause of that failure. Later successes cannot retrospectively attribute it
to timing, compilation load or a specific runtime predicate; the cause remains
unresolved and the failed capture is retained.

## Final local results

Three independently started Rust1.94 matrices and one Rust1.88 matrix each pass
nine SIGKILL cases, one uninterrupted control and twenty fresh inspections.
The latter three executions use the exact registered test name after the source
commits. The complete original-input regression passes 69 with the manual case
ignored; its earlier 68/1 failure remains above. All eleven fence cases pass.
The new readiness protocol parser passes on both compilers.

Both affected packages pass all-target/all-feature check and strict Clippy on
Windows/Linux with Rust1.94 and Rust1.88, including a final repeat after the last
static assertion. Windows checks do not execute Linux-only process fixtures.
All 168 static contracts and six registry tests pass. The automatic scanner
still finds 176 covered gates; the separate explicit registry grows from 229 to
230 entries with the new manual unit gate. That manual entry is not an additional
automatically discovered integration-test gate.

Full workspace formatting passes in WSL, with an additional explicit check of
all changed Rust files. Documentation and the local non-promotable performance
contract pass; require-ship returns the expected closed-admission error. The
previous controller-attempt packet still verifies without changing its files.

## Evidence interpretation

The test implementation is `b5f340f53712098aa7a5ab5593a54b3a1341682e`;
the source cut including static contracts is
`11a7e5cfbf9a6365900e8f7fde72d28e3bc57385`. Preregistration and its authority
origin correction are `5cc3b644` and `783f5500`. Development captures describe
their intermediate states; the source cut does not retroactively identify them
as final-source executions. Final checks before these commits used the same
source bytes with documentation/evidence still dirty. Subsequent matrix repeats
use the committed source with those non-code changes still pending.

[manifest.json](manifest.json) inventories normalized UTF-8/LF command captures,
expected exit statuses, historical source blobs and frozen inputs. The verifier
also checks runtime/schema identity against baseline and requires every exact
matrix observation, not only a passing test count. Run [verify.ps1](verify.ps1)
for unsigned consistency checking. Neither a matching manifest nor console logs
constitute signed execution attestation or a clean-tree build claim.

Full workspace release verification and the older legacy process matrices are
not rerun by this test-only slice. Their historical packets are not relabeled as
new joined-composition results. Production readiness is not inferred from local
green guards or from the availability of the released common lock.
