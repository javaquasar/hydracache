# Original worker leader mount namespace consistency

## Scope and baseline

Baseline `4c93f79987f2f582e75bd2d5cdc3a435d5b3bf4b`. Add an opt-in Linux read-only guard
for the original retained process leader and the original reading thread sharing
a mount namespace. Do not alter the user namespace, credential or signed opening
APIs. This standalone guard does not enroll a signed host or open production.

Construction revalidates the original process before opening its fixed `ns/mnt`
object, retains that ns directory, retains current `thread-self/ns/mnt`, and
requires the same nsfs device/inode with mount type. The original observer TID is
pinned; the guard is neither Send nor Sync. Only literal proc magic links may be
followed, with read-only CLOEXEC descriptors. Wrong fs/type, directory substitution,
namespace drift, original process exit or observer thread drift refuse.

Revalidation is generation/namespace/generation/namespace/generation.
Each namespace stage checks the original observer TID before access, current and
retained typed observer/worker objects, original and named worker ns directory
identity and same procfs device, then freshly named worker mount plus observer.
Stop on the first typed error and latch refusal. Original process is borrowed and
never recaptured, signaled, mutated or exported. No descriptor or identity getter,
reset, public callback or arbitrary path/namespace selector. Healthy drop preserves
the process. New unrelated guards do not share refusal.

The [kernel implementation](https://raw.githubusercontent.com/torvalds/linux/v6.1/fs/nsfs.c)
and [UAPI](https://raw.githubusercontent.com/torvalds/linux/v6.1/include/uapi/linux/nsfs.h)
define NS_GET_NSTYPE as the namespace type, not a textual name check.
Checking shared device/inode does not identify the initial host namespace or
prove mount-tree content immutability. Mount operations may change contents while
namespace identity is unchanged. The guarded process path observes its leader,
not every task. No all-thread, atomic/continuous, file-opener, signed policy or
authenticated original-start authority follows.

## Tests and gates

Capture portable baseline and missing API before implementation. Cover exact
generation order, every first error, prior refusal without IO, seed-driven identity
mutations, negative Send/Sync, actual owned helper repeated reads, non-mutating FD
flags, wrong namespace type and regular descriptors, process exit, named/retained
directory and descriptor substitution/restoration, observer TID drift before IO,
healthy drop and independently constructed concurrent guards. Kernel-foreign
mount namespace creation is not needed or authorized; synthetic distinct device/
inode cases test equality rules, not a real setns/unshare race.

Run scoped Linux and Windows supervisor tests/check/strict lint, root contract/
evidence/governance, format, docs/links/book, final registry and expected-red ship.
Retain exact raw logs and negatives. Existing packets, frozen qualification
manifest and its actual contract inputs stay byte-exact. No host infrastructure,
product workload, performance claim or full-workspace qualification. Rollback is
the previous binary without this optional diagnostic reader.

Next: separately compose this original mount guard with signed context and
credentials without allowing a replacement process, then bound all-thread
inventory and per-thread observations. Durable external authority and production
preparation/start remain separate.
