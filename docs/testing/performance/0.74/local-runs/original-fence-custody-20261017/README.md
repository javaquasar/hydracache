# Original fence custody process checks

The private one-shot custody prerequisite preserves the original common flock
after ordinary destruction of its original owner. Four new units and all ten
existing original-fence units pass. The declared six-case process matrix passes
three repeats in WSL debug and on each of the Rust 1.94/1.88 GNU release binaries:
18 local and 36 dedicated-host cases, including nine actual holder SIGKILLs in
total. These are ownership correctness checks, not a product benchmark or an
authenticated guardian/start certificate.

## Hypothesis and isolated change

An explicit flock unlock acts on the shared open-file description, not just the
descriptor being destroyed. The old destructor therefore releases an inherited
lock even while another process retains its duplicate. The saved baseline
counterexample fails custody_survives_original_drop at the independent common
lock contention assertion. baseline-tracked.patch records the tracked ownership
scaffolding at that failed screen; it is deliberately not a complete source
capsule because the new unit file was untracked then. The failing test body is
retained unchanged in the final source. No failed runtime outcome was reclassified.

Only the original private Fence gains an irreversible custody flag and a private
one-shot owner. Its unchanged inspector brackets duplication of the already
locked original description. The flag changes only after both inspections
succeed. Thereafter original destruction and custody destruction close their
aliases without explicit unlock. Default construction still selects the old
explicit-unlock behavior. The separate ShortHostFence and campaign claim owners
are unchanged. No new public constructor or descriptor, path, time, send, start,
recovery or completion API exists; there is no production custody caller.

The plan is [registered separately](../../diagnostic-original-fence-custody-plan-20261017.md).
Implementation source is 3334ab7461fe7237de23f89cf861200b3614b37b against
8d9f1ae4616237ca3e95d2e21130a9bb356301d0. implementation.patch is the exact
complete committed code/test/registration diff, independently checked against
those commits. All earlier inspectors, original grouped preparation, autonomous
engine and deadline origins remain unchanged.

## Process and acknowledgement oracles

The test uses the existing original Reserved fixture and its actual common
lock, not a substituted arbitrary lock. A nonexecuting raw-fork holder retains
only the custody descriptor and two test sockets. Signal blocking precedes fork;
the parent restores its exact old mask. The child uses bounded raw syscalls and
never runs Rust allocation, destructors, library IO or validation. The parent
pins its exact unreaped child with pidfd before observing it.

Each repeat covers close, ack-omitted, ack-disconnected, holder-kill,
original-last and repeat-refused, in that order with seed 0x0740c517. A separate
test structural barrier follows the optional ACK attempt. It allows the harness
to observe an omitted/disconnected ACK without interpreting it as successful
original readiness. Both legacy common-lock entry points must remain busy after
original-owner destruction. Terminal pidfd readiness and actual wait status
precede reacquisition; SIGKILL must report signal nine. In the reverse owner order,
holder exit cannot release the original owner, which must still revalidate.
Existing Reserved bytes remain exact. The holder is neither a watchdog nor a
controller/worker/collector and has no executable or completion authority.

The dedicated run used nonroot UID/GID 1000 in
/tmp/hc074-fence-custody-20261017.aRRelc, with TMPDIR restricted to that private
scratch directory. Five pre/post guards preserve the original supervisor PID
8839, zero restarts, boot identity, installed binary and receipt digests, absent
campaign/lifecycle/authority contexts and closed pilot path. The final driver
also finds no surviving fixture executable. Only test binaries/scripts were
uploaded; no release install, supervisor mutation or product workload occurred.

## Reproduction and evidence verification

From the repository root, run the units and the separately gated manual matrix:

```text
cargo +1.94.0 test -p hydracache-long-run-supervisor-074 --lib --locked original_fence::tests -- --nocapture --test-threads=1
cargo +1.94.0 test --release -p hydracache-long-run-supervisor-074 --lib --locked original_fence_custody_matrix -- --ignored --nocapture --test-threads=1
```

Use a nonroot Linux x86_64 process. Repeat the release command on Rust 1.88.
The retained guard.sh and run.sh describe the exact authorized dedicated-host
scope; their immutable host pins intentionally refuse a changed installation.
verify.ps1 validates all evidence hashes, exact case order/seed/repeats, compiler
and script digests, current/committed source blobs, full source capsule and
unchanged qualification input. Its -SelfTest accepts a complete matrix and
rejects ten malformed or failed variants. verify-prior.ps1 checks the unchanged
previous autonomous packet in an archived source view at 09b43743, using an owned
index copy and preserving the active index. It does not rewrite earlier proof.
The first wrapper restored absent Git variables as existing empty strings under
PowerShell 7.6, causing the next Git command in the same shell to refuse. That
tooling-only negative is retained separately. The corrected wrapper explicitly
removes originally absent variables and verifies exact restoration plus a fresh
repository lookup. No runtime source, deadline or process outcome changed.

The focused gates pass: 201 static contracts; 184 discovered gated tests all
registered; Linux 1.94/1.88 and Windows all-target checks and strict all-feature
Clippy; whole-workspace formatting; doc-check; local non-promotable performance
contract. The broader original-fence subtree, including empty-reservation
owners, also passes all 28 fast tests on each GNU release binary in WSL.
Full milestone verify and ship qualification are not claimed.

## Remaining execution boundary

This closes the explicit-unlock ownership defect only for the private custody
transition. Original full-frame enrollment, sole sink/writer transfer and parent
alias closure still need a typed join before any real worker start. The original
300/60/10-second origins and every live predicate remain mandatory. Whole cgroup
revocation, external guardian supervision, durable completion and later cells
remain separate prerequisites. As the SIGKILL control proves, killing the last
kernel owner releases flock: durable history must still prohibit reuse. No
performance gain, production admission or qualification input change follows.
