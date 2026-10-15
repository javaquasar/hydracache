# Unsigned prebuilt copy preparation

The opt-in Linux preparer separates Cargo's writable hardlinked output from a
new file without modifying the source. It proves a bounded copy and closes its
own writer before returning read-only inventory. Expected bytes and parent
selection remain assertions; the result cannot authorize a signer or worker.
The [preregistration](diagnostic-prebuilt-copy-plan-20261015.md) and
[contract](diagnostic-prebuilt-copy-contract.toml) fix this boundary.

## File creation and retained inventory

`prepare_asserted_copy` accepts read-only CLOEXEC source and parent descriptors,
an exact `attempt-` plus 32 lowercase hexadecimal characters, and a bounded
asserted digest. It refuses root, a non-0700 parent or UID/GID mismatch, invalid
expectations, nonregular input and writable/O_PATH descriptors before creation.
The source may retain Cargo aliases; it is never chmodded or unlinked.

mkdirat reserves an absent attempt and fsyncs the parent. openat creates only
`diagnostic_controller`, exclusively, without following links. A 64 KiB buffer
copies positionally, preserving the source offset, with exact size/hash checks
and metadata bracketing. A second source content pass refuses a persistent
same-size mutation even when timestamps remain equal; no sleeps or timestamp
granularity assumptions are used. Only the new file gets 0555 after file sync. Its owned
writer is explicitly closed once, and close errors refuse. The function reopens
the same inode read-only, seals the new directory to 0555, syncs directory/parent
and rehashes through original descriptors. Rechecks bracket names and content;
failure permanently refuses that inventory guard. No File/raw FD or conversion
to bundle/launch authority is exported.

Existing and interrupted attempts are never overwritten, deleted, resumed or
repaired, even when their final bytes look complete. The public API leaves all
created state on disk on failure and on guard drop. Cleanup, if needed, is an
explicit owner operation, not an admission/retry path. Different attempts may
share one parent, so parent timestamps can change; parent identity/ownership/mode
and each attempt's original full stamps still have to match.

The Linux [open manual](https://man7.org/linux/man-pages/man2/open.2.html) describes
descriptor-relative/exclusive creation. [close](https://man7.org/linux/man-pages/man2/close.2.html)
can report delayed errors and must not be retried on Linux; [fsync](https://man7.org/linux/man-pages/man2/fsync.2.html)
requires a separate directory sync for directory entries. These mechanisms do
not authenticate a build or prove storage survives every hardware failure.

## Tests and actual process loss

Tests cover source hardlinks, new inode/mode/link identity, unchanged source,
source-offset preservation, short IO, exact chunk-boundary sizes, deterministic
corruption, EIO/ENOSPC seams, late close error propagation, seven injected cuts,
source mutation, name/directory substitution, extra entries, link/mode drift,
sticky refusal and same/independent-attempt concurrency. Compile-fail cases
forbid clone, FD export and bundle conversion.

An owned libtest subprocess pauses at each of seven checkpoints: directory
creation, file creation, copy completion, first sync, readonly mode, writer
close and directory seal. A local socket barrier identifies the checkpoint;
the parent SIGKILLs and reaps only that exact child. The same attempt then
refuses reuse, including a complete-looking final copy. This is real process
loss in the preparer, not execution of the prepared bytes or power-loss proof.

## Foreign writers remain a separate boundary

A counterexample opens a foreign writer while the new file is writable. The
initial readonly inventory passes while that descriptor remains open, and the
descriptor can still write after 0555. A subsequent inventory check refuses
and stays refused after content restoration. Thus mode bits and a matching hash
cannot be treated as writer revocation. Untrusted same-UID mutation/debug access,
concurrent fork inheritance and administrators remain outside this local proof.

Production use still requires independently protected ancestry/account/runtime/
job selection and authenticated receipt acquisition. This new inode is not the
original Cargo inode and is not silently substituted into the old bundle reader.
The optional `prepare_unsigned_prebuilt` example creates local inventory only;
it neither executes nor signs it. Controller/server routes, issuer pins, bundle
reader, dependencies and frozen qualification are unchanged. Worker original-start,
all P2/P3 writers, completion cells 2–4 and server rehearsal remain separate.
