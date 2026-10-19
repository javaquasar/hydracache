# Independent local root guardians

Source `72f538063aa03d460843ed813a2c249e36223b24` adds a private active v2
guardian pair. Baseline is `037802aa0efc0e87ad847f1f285a0d043148535b` and the
previous [descriptor transfer packet](../cgroup-root-transport-20261019/README.md)
stays closed. This package tests local kernel revocation and reply mechanics,
not original runtime admission, server installation or product performance.

## Retention and irreversible revocation

Two clone3 children retain the same existing fence and timer open descriptions,
actual creator process pidfds, individual private channels and the pinned root
parent. They close all other inherited descriptors. Stack-only ancillary and
root validation admits exactly three CLOEXEC directory/kill/events rights from
the exact creator. The full inherited packet and retained/named kernel objects
must match an empty unfrozen domain before a rights-free Armed reply. Both
requests precede waiting for either reply. Parent acceptance requires the actual
clone child credentials and live pidfd; every missing, malformed or lost reply
remains uncertain and cannot reopen the one-shot operation.

Each child additionally opens an independent read-only event watch on the same
bound inode. The received events FD stays retained, but cannot be the shared
notification cursor for both children. Before blocking in revocation, a child
acknowledges its own event before the kill syscall, not afterwards. Otherwise a
late arrival could be read and acknowledged after kill, leaving a live writer
while the watcher sleeps. Revocation remains active across empty observations
and repeated late population; it never reopens a replacement kill target.

The parent inspects the existing timer through kernel fdinfo inside opening and
closing BOOTTIME samples. Clock, absolute-set flag, zero interval, remaining
value and terminal state must be consistent with the selected bound. This is
a consistency bracket, not a reconstruction of exact expiry identity or an
original-owner constructor. The actual existing timer remains the limit and is
never rewritten. There is no timer counter read, setter, creation or gettime in
the production mechanics. Linux's
[timer implementation](https://raw.githubusercontent.com/torvalds/linux/v6.6/fs/timerfd.c)
can forward an expired periodic timer in gettime; the negative periodic fixture
requires its expiry and readable event to remain unchanged after refusal.

After possible enrollment, ordinary parent Drop closes channels but does not
kill independent custodians. Partial creation before any send owns exact-child
cancellation before restoring signals can fail. No child unlocks the fence or
publishes completion. Best-effort Refused/Revoking notices are not cessation
receipts. A surviving guardian holds the original lock after all parent copies
are closed. Killing both holders loses that lock; stopping both retains the
lock but does not stop the writers. Both are explicit common-mode negatives.

## Actual local fixture boundaries

The marked root/manual matrix uses fresh temporary cgroup2 roots and exact
owned `/bin/cat` children running as UID/GID 65534. Successful migration writes
precede asynchronous guardian kill; a post-migration `/proc` snapshot cannot
be required from a process already being killed. No creator or guardian enters
the revocation root. Existing fixture helpers and old tests remain unchanged
apart from one exact module registration.

Four construction cases cover first/second clone failure and errors after
actual signal-mask restoration. They require the original signal mask and
creating thread's child list to match after exact owned reaping. Twelve active
cases cover expiry, actual channel disconnection, first/second guardian death
or stop, first/second missing ACK, duplicate enrollment, an external writer
alias and both common-mode failures. Late population repeats eight times after
the first descendants are dead. Positive cases require actual target SIGKILL.
Before population, live acknowledged children retain exactly nine descriptors:
five initial roles, three transferred rights and the independent event watch.

The harness closes every parent copy of the original fence OFD and verifies a
new opener remains blocked while a guardian holds it. Cleanup uses only owned
pidfds, then reacquires a separate fixture fence for the old fixture destructor;
this is test cleanup, never a runtime completion or recovery permission. The
hidden alias prevents EOF after root writers die, and EOF arrives only after
that alias closes. Guardians also discard unrelated writers inherited before
clone. Case numbers and the fixed `0x074c2021` schedule are replayable.

Actual creator SIGKILL, thread loss, joined original policy/epoch/artifact/frame,
held-unit delivery, durable completion, later cells and same-attempt reconnect
remain unproved on this path. Channel disconnection is not labeled creator
death. Local Armed bytes are not the original Armed capability. A valid packet
or private borrowed-FD constructor cannot grant production trust or start.

## Reproduction and raw evidence

Run `run.ps1 -Compiler 1.94.0 -Label <fresh-label>`, then the 1.88.0 cohort with
another fresh label. Selected sources must stay committed at the same HEAD.
Both compilers run eighteen ordinary root controls (five new), broad two-phase
and exchange controls, the unchanged seventeen-case guardian regression and
three repeats each of the new sixteen-case matrix, old twelve-case transport
matrix and old eleven-case root SIGKILL matrix. Manual gates require explicit
local-root markers; registry discovery alone does not execute them.

Receipts bind source, recipe and image before/after, nonroot build UID, fixture
UID, empty test-prefix cgroup contexts and unchanged active timesyncd snapshots.
There is no service or clock operation, installation, server contact, original
manager send or product workload. `inventory.json` hashes exact raw bytes with
no prefix or newline canonicalization. `verify.ps1 -SelfTest` checks the closed
file set and selected receipts, then rejects eight mutated inventories. Never
reseal or append to this closed packet; choose a fresh sibling directory.

## Preliminary refusals

`red-missing-tests.log` is a preliminary compile failure, not a functional red
proof: it includes an absent test module and unstable attributes on binary
expressions. Test-only argument expressions were wrapped in stable blocks.
`red-missing-contract.log` is the expected one-test missing-contract refusal.
The first unit screen retains four passes and one timer refusal because the
prototype expected a tab where the kernel emits a space in clockid. The
corrected screen passes all five controls without changing the absolute-bound
inequality. The gettime approach was removed as well to avoid its periodic
timer side effect. Uncommitted-source screens are not selected receipts.

The first MSRV cohort, `gnu188-selected`, stops in the byte-unchanged old Python
guardian regression before any new root/manual matrix. While enumerating a
guardian's fd directory, a transient descriptor disappears before readlink and
raises FileNotFoundError. The harness finally block kills/reaps its owned
processes. That partial cohort stays failed and has no successful receipt;
it is not used as acceptance. `gnu188-confirmed` repeats the same source,
recipe, checks and original deadlines in a fresh directory, with no predicate
or threshold change. This existing live-proc snapshot race remains a tooling
reliability risk, not a recovered successful first result. The old raw end path
closes its peer after signaling the target, so target completion does not make
the fd-table snapshot atomic. The failed descriptor's role was not retained;
the exception alone establishes neither a lost fence nor a product regression.

## Changed implementation files

`diagnostic_root_guardian.rs` owns ordinary parent mechanics;
`diagnostic_root_guardian_raw.rs` owns stack-only child mechanics;
`diagnostic_root_guardian_tests.rs` and
`diagnostic_root_guardian_fault_tests.rs` hold ordinary and marked-root controls.
All four are under `tools/long-run-supervisor-074/src`. The old kernel and root
fault files gain exact module registrations only. The new xtask source guard
and its parent adapter preserve all 389 old Rust bodies and old wire/registry
sections. COMPAT, the gated registry, a separate contract and local plan record
this reader scope and keep original/server readiness closed.

## Selected results

`gnu194-selected` and `gnu188-confirmed` both pass eighteen ordinary root
controls (five new), forty broad two-phase controls, thirty exchange controls
and the unchanged seventeen-case guardian regression. Both pass all-target
check and strict all-feature Clippy. Each runs three sixteen-case active pair
matrices: 96 selected scenario executions across the two compilers. The old
transport and root matrices also repeat three times each per compiler, for 72
and 66 scenario executions respectively. The latter retain 24 actual private
kill-caller SIGKILL cuts, not actual creator SIGKILL on the new pair path.
Recorded closing snapshots contain no fixture roots; time service snapshots
stay active without a service or clock operation.

The complete source suite passes 283 controls in 949.72 seconds, preserving
all 389 previous supervisor Rust bodies through exact additive registrations.
Windows passes 54 ordinary and eighteen compile-fail controls plus strict
affected Clippy. Formatting, docs, registry discovery (184 gates, all registered)
and the local non-promotable performance contract pass. Both preceding raw
evidence inventories and their negative controls still pass. Qualification raw
SHA256 stays
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

These results close the scoped local pair package only. The failed first MSRV
cohort remains failed and excluded from selected counts. Actual creator loss,
the original full-frame constructor and joined lifecycle/reconnect/server gates
remain subsequent work; no performance or production readiness claim follows.
