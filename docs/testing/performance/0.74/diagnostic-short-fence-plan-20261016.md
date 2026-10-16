# Short diagnostic fence inheritance investigation

Baseline is 166c3d1a. Investigate one mechanism behind possible local Host(Busy):
lock_host_root returns a raw File, so a pre-exec fork alias can retain its flock
after the transaction's owner closes. CLOEXEC does not close an alias before exec.
The [Linux flock description](https://man7.org/linux/man-pages/man2/flock.2.html)
binds locks to open file descriptions; this is a hypothesis for the old captures,
not occurrence-specific attribution of those captures.

## Deterministic causal controls before implementation

Add Linux-only private unit tests with a thread-local one-shot post-acquisition
checkpoint. It exists only under cfg(test), takes a borrowed File, and changes
no production body or public API. At the checkpoint fork one exact owned child
which retains the original descriptor with CLOEXEC set. Only async-signal-safe
raw syscalls execute in the child. Pipe acknowledgements establish ordering;
bounded poll is a harness watchdog, not a correctness clock or signed deadline.
Keep no unrelated inherited descriptor in the child. Pin/reap the exact child;
panic cleanup kills only that owned unreaped child, with no user/server process.

Keep a raw-File negative control: dropping the parent alias must leave Busy
until the child explicitly unlocks. Add expected-red regression tests requiring
release at successful short-fence drop, admission-construction error, and each
of the seven corrupt/pending ledger variants from the old IPC fixture. Recover
the original revision while the child still retains the old description; preserve
state/ledger/pending bytes and perform no backend execution. Never modify or retry
the original integration test. Also prove an old child's late unlock/close cannot
release the next independently opened owner, and a failed contender cannot unlock
a live owner. Preserve the baseline reds and negative control outputs.

## Conditional isolated correction

Only after the mechanism is reproduced, return a private non-cloneable owned
short-fence guard instead of raw File. Wrap only after successful flock, before
fallible admission checks; Drop explicitly unlocks that original description.
No Deref, File clone/extraction or public capability is added. Adapt only private
fixed-admission test borrows to the contained File; preserve every assertion.
Long-lived HostExecutionClaim, original/fixture fences, campaign acquisition and
all durable markers/ledger/public methods stay unchanged. Broader campaign
construction-error paths are outside this correction and remain separate audit
work. Closing a cooperative short transaction is not all-writer cessation.

Add exact-source static restoration/canaries for the one guard substitution,
test-only hook and two test-borrow adaptations. Worker proof, source/epoch/kernel/
time guards, old tests, qualification inputs and previous packets stay exact.
No retry, backoff, sleep, serialization, ignored old test or deadline adjustment
may stand in for correct ownership. No numerical performance claim is planned.

Run new causal controls and fixed-admission tests on Linux current/MSRV, affected
packages check/strict Clippy on Linux/Windows current/MSRV, old IPC/lease/host/
original-fence integration tests, worker-proof tests, original-input regression,
static contracts, formatting, docs/registry/local checks and expected-red ship
admission. Retain any natural failure without rerun-to-green. New cases do not
retroactively identify either historical occurrence's child or kernel lock owner.
Record source/evidence separately and update article/readiness documentation.
Server, product workload, qualification, full release admission and push stay closed.
