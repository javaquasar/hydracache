# Forced D-Bus fragments and kernel ancillary truncation

This continues the [GNU FD candidate](dbus-linux-fd-result-20261028.md).
The local receive leaf accepted a partial descriptor set despite kernel
MSG_CTRUNC. A separately tested guard now returns InvalidData and releases the
received descriptors. The final 42-case catalog, check and strict Clippy pass
on Rust 1.94 and 1.88. This is a standalone candidate, not root dependency
adoption, a performance result or original execution/server admission.

## Boundaries and unchanged inputs

Private nonroot WSL directories contain exact copies of the preceding source,
registry archives and Linux CLOEXEC patch. The new probe names rustix directly
so that test adapters can call its typed syscall API. After the first fixture
pin refusal, that edge is bound to the already locked 1.1.6 identity/checksum.
All 62 package identities and all non-probe lock rows stay exact. No upstream
version, root lock, feature union, deny exception or MSRV changes. The fixture
lock has one added probe dependency edge; it is not byte-identical to its parent.

The framing tests use zbus's actual public ReadHalf.receive_message and
WriteHalf.send_message defaults. Their syscall adapters use complete, pinned
source-extracted fd_recvmsg/fd_sendmsg leaves from the selected runtime. A single
readonly hook records the returned kernel truncation bit; stripping it recovers
the exact leaves. FDS_MAX remains 1024, matching the original utils constant.
The extracted leaves are not a separately qualified system-manager connection.

The inherited native runner is composed under an independent whole-body SHA.
Every fixed entry points its disposable worker at the same exclusive context.
The CI owner's original subreaper/pidfd/ECHILD cleanup is unchanged. Outer local
1200-second, inner capped 1100-second session origins, physical/native 4 GiB
floors, five-second capacity/retirement and 250-ms whole-frame guards remain.
No fragment resets its frame deadline. No service, clock, installation, server,
product workload or expensive qualification is invoked.

## Forced I/O observations

The successful 164-byte fixture contains one FD and a fixed payload. Adapters
limit the actual sendmsg/recvmsg buffer lengths to (write/read) 1/1, 3/5 and 7/2.
Every scenario requires multiple positive kernel operations, complete exact wire
bytes, bounded per-call lengths, one FD only on the first write, one received FD,
valid decoded identity/payload and exactly one final flush. For the 1/1 case,
164 successful writes and 164 successful reads are observed. Other read counts
may vary with scheduling; they are not performance acceptance thresholds.

Four EOF cuts stop after bytes 1, 15, 16 and 163. Each sends a real FD with the
prefix, shuts down only its own write side, and requires the full supplied prefix
to be consumed followed by refusal, never a completed message. Per-inode FD
census requires no surviving received alias. A separate real write-side shutdown
after three bytes produces a failed second write: no repeated FD, no flush and
no receiver message. These are controlled socket faults, not logical error stubs.

## Real MSG_CTRUNC without shrinking the receiver

A dedicated child process first proves normal receipt of all eight queued
descriptors without truncation. It then queues another eight while its original
limit is intact, temporarily lowers only its own soft RLIMIT_NOFILE, and calls
the same receive leaf with its unchanged 1024-FD ancillary buffer. It records
actual kernel MSG_CTRUNC. The original soft/hard limit pair is restored before
assertions/reporting; no host or parent limit changes. This resource condition
is described in the primary [Linux unix socket documentation](https://man7.org/linux/man-pages/man7/unix.7.html).

The first complete executable fault baseline is **41 passed / 1 failed** on
Rust 1.94. The child observes eight sent, one returned, kernel truncation true
and rejected=false. The partial result is dropped and only the original fixture
FD remains. This demonstrates silent success at the selected syscall-leaf
boundary, not an exploit or accepted original systemd delivery. The parent
fails because its isolated child refuses the predicate. Child libtest output
is escaped into one line so parent accounting cannot count it twice.

The [isolated guard](../../../../tools/dbus-fragments-074/reject-ctrunc.patch)
checks the returned CTRUNC bit before handing any FDs to its caller. The existing
ancillary buffer's destructor closes received descriptors on that early error;
the per-inode census checks this after the limit is restored. Byte/ancillary
limits, original CLOEXEC flag, EOF and other error branches remain unchanged.
No buffer-size threshold is reduced to manufacture a passing test.

## First refusals and final result

The original build never starts tests: the added fixture edge erroneously asks
for rustix 1.1.4, while the offline locked input contains only 1.1.6. No fallback
fetch or lock regeneration occurs. The wrong-pin source/streams remain; the
corrected baseline has its own root and sealed-parent version control.

After the truncation guard, all 42 tests first pass on 1.94, but strict Clippy
refuses five deprecated AsyncFd::new calls in the test adapter. That first green
functional result does not count as a green step gate. A separate source view
replaces only those five calls with an owned-socket registration helper and the
required documented unsafe AsyncFd::register call. Socket FDs stay owned, open,
nonblocking and unchanged until wrapper drop; no numeric close/replacement,
extraction, forgetting or get_mut occurs. Shutdown does not close the FD.
An inverse control recovers the earlier test body exactly. No lint is suppressed.

The final complete catalog passes **42/42 on Rust 1.94 and 42/42 on 1.88**, with
both all-target/all-feature probe checks and strict Clippy. The unchanged
truncation oracle now records eight sent, zero exposed, rejected=true,
kernel_flag=true, restored limit and only the original alias. The child selects
one test with 41 filtered; the parent selects all 42 without ignored/filtered
cases. The inherited 38 cases remain in the actual catalog. All first refusals
and native image/source/accounting/owned-retirement receipts remain distinct.

## Retained evidence and focused checks

Six Python controls pass: all fixed worker/context pairs, exact catalog
membership/refusals, the sealed-parent direct dependency identity, source-guarded
patch/observer inverses and the five registration-call replacements. Focused
Rustfmt passes for authored test sources and the actual patched runtime leaf.
No lint or policy exception is added. All CI ownership receipts retire without
errors, and the final native session verifies vendor bytes and an empty owned
fixture directory. No fresh root package/workspace or dependency-policy pass
is inferred from this component result.

The source-only commit is `a8bda3a2e76a5250c5c24d10aeb83785dde3d776`.
The [primary packet](local-runs/dbus-fragments-20261028/index.json) contains
2641 logical references and 473 distinct raw objects. Index SHA-256:
`ab86dca0ad6ed6f03fc295a47fe22bc20a14f4238b7b773f2b0823a437b09939`.
The receive leaf's postimage SHA-256 is
`f54f8160a3cdb0c9cdd68189a462079d94d4d46c6821458eaf544609bb933588`.
First input/source views, command streams, compiler artifact/catalog/accounting,
child output, byte/lock inverses and ownership receipts are retained. The normal
parent protocol traces match the preceding candidate. Native binary/target trees
are excluded. Verification reads and compares artifacts without executing
archived code. All contexts retain their own boot/attempt identity, with no
mixed-lifetime qualification claim.

The first inert verifier completes its member/native-record assertions but
refuses its postflight: its local leaf byte variable shadows the source-snapshot
metadata variable. A separately named corrected verification retains distinct
metadata/leaf variables and the original postflight checks. The first verifier
source/refusal is retained. Packet bytes and native outcomes do not change.

## Still open

This qualifies controlled private framing and the pinned syscall-leaf boundary,
not full system-bus/authentication behavior under exhaustion. A real manager,
original ReplyState/owner/FD binding and all-writer cessation are not exercised.
CLOEXEC and per-process FD cleanup do not revoke descriptors held by other
writers. Production lifecycle, SIGKILL/lost-ACK durability and reconnect remain
separate work. WouldBlock/Interrupted, backpressure and cancellation coverage is
not inferred from these chunk tests. Non-Linux targets, other features, final source/effect/distribution
review, root integration and complete package/workspace gates are still required.
