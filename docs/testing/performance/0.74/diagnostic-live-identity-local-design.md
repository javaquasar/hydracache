# Original diagnostic manager process and tree identity

Preregistered against clean `90aa84fcf887a4bfcb1ca4ecec9c31fee842085b`.
This W11 slice composes existing read-only observations; it does not prepare
production output directories, start or stop units, install a backend, run an
observer or change qualification admission.

The loaded invocation guard deliberately permits MainPID and lifecycle status
to change. The process reader retains one generation and pidfd; the recursive
tree reader retains one stable inventory. Checking these independently does not
establish that the manager's MainPID is the original process in the matching
cgroup node. The hypothesis is that a typed composition can reject mismatches
and propagate every observation failure without adopting replacement evidence.

The Linux entry point must borrow the original ProcessRead and TreeRead and own
the original InvocationGuard. Derive scope from validated startable state. Require
kernel tree origin, exact original boot and fixed diagnostic root path, nonzero
root device/inode, original MainPID, active/running status and success result.
Require the process's retained boot, PID and nonzero start ticks, and its exact
cgroup path to match the node containing that PID. Refuse any frozen node.
Two bounded original-manager observations bracket two process/tree checks.
Any manager, process, tree, membership or status failure permanently latches
the composite guard; restoration must not repair it. Preserve manager failure
details including helper cleanup rather than reducing them to a success.

Existing manager helper limits apply separately to each observation. Composition
introduces no retry, helper mode, wire document, syscall mutation, serialization,
signal, new pidfd or tree refresh. Local filesystem scans remain bounded by the
existing reader budgets, not a newly promised whole-operation wall-clock bound.

Tests first exercise the join and bracket ordering through private synthetic
seams: four cells, root and descendant membership, PID/status/result drift,
boot/root/origin mismatch, generation/path mismatch, frozen trees, both manager
positions, both process/tree positions, refusal after restoration and parallel
independent guards. An actual owned cat helper may verify rejection of foreign
scope and fixture trees, but cannot provide a positive diagnostic-cgroup proof.
Do not create/migrate cgroups to manufacture that positive locally.

This is a stable sequential observation, not an atomic snapshot or continuity
proof. Tree membership changes, including legitimate forks, conservatively
refuse the retained inventory. Manager ControlGroup does not authenticate the
caller-asserted root inode or original start. Output/executable/environment
binding, production ancestry, durable refusal, writer revocation, autonomous
watchdog and uncertain-intent reconciliation remain separate prerequisites.
No timing, allocation or throughput claim follows from these safety tests.

## Implemented read only composition

`pin_live_identity` derives the fixed scope and borrows the original readers;
fixture trees cannot pass its origin/path/boot check. The private join retains
the generation and root identity and owns the invocation/settings guard. Each
revalidation reads the original manager, checks process/tree/process twice and
then reads the original manager again. Manager failures retain
their bounded stdout/stderr and cleanup status. No new generation, inventory,
invocation or output capability is captured after refusal.

The existing invocation-only policy remains unchanged. The stricter composite
requires the original nonzero MainPID and active/running/success at both manager
observations. A PID listed in a sibling or another node cannot satisfy the join,
even if the PID occurs somewhere in the correct unit tree. Membership documents
and process generation remain separate reads; drift-and-return between them is
not detectable as an atomic transaction.

Nine private synthetic tests cover the join, failures, ordering and independent
guards. A tenth test owns a real cat process and temporary regular-file tree:
the helper stays alive, its foreign cgroup cannot pass fixed kernel process scope
and the fixture tree cannot become a kernel join. The synthetic booleans in the
private test view never construct public kernel readers. Positive live manager,
fixed diagnostic cgroup and original process composition still requires a later
controlled rehearsal, not an invented success from temporary documents.

## Local results and remaining enrollment

Implementation `44b62afe98bd9976ff1b4e3d3b23eae08e63fc07` is supported by
the [retained packet](local-runs/diagnostic-live-identity-20261010/manifest.json).
The baseline has seven loaded-manager passes; its Cargo name filter also filters
out every tree test, so no baseline tree pass is claimed. Complete focused runs
before and after ordering review each pass 220 Linux tests with one pre-existing
ignored. The final ten new tests pass three further compiled-binary repetitions.
Windows has 28 portable passes and zero Linux identity-join execution.

Root checks pass 82 performance contracts, 13 evidence and 23 governance tests.
Affected all-target checks, strict all-feature clippy, scoped formatting,
documentation/local non-promotable contract, links and book build pass. Full
workspace verify is not rerun for this isolated slice; require-ship remains
expected-red, native status confirmed exit 1, for unresolved candidate identity
and qualification. The frozen qualification digest is unchanged.

The packet keeps the missing-API compile refusal and initial formatting-sensitive
source-guard failure. Review also moves the second source check before the final
manager read; the final sequence matches the preregistered bracket. Earlier
alternating-order observations are retained separately, not passed off as final
implementation evidence. Byte/hash/count verification is offline retention QA,
not authenticated host execution or admission.
