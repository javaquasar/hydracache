# Original root owned input and reader context inventory

Baseline is `f09330e3cc9c211edf6a94f814fb38c3b37e5cf6`. Continue the signer
prebuilt prerequisite with a separate read-only Linux reader. Do not change the
copy preparer or its retained evidence, the old bundle reader, issuer pins,
qualification, actual ownership, accounts, namespaces, mounts or services.

The public entry takes an explicit absolute file path and asserted bounded
size/SHA256. It selects numeric UID/GID 0 internally, never a caller owner or
an origin derived from an incoming bundle. Walk from the current `/` through
readonly NOFOLLOW/CLOEXEC/NONBLOCK descriptors, retaining every component.
Reject dot, dotdot, empty/trailing components, NUL, overlong/deep names, symlinks,
special files, unsafe ownership/permissions and multiply linked or oversized
leaves. Directories accept only 0555/0755; regular input files only
0444/0555/0644/0755. Root's owner-write bit is allowed for inventory, not a
weakening of the old bundle's exact 0555 requirement.

Require returned statx type/mode/owner/link/size/time/inode/mount-ID fields;
missing support refuses, with no device/inode-only fallback. Pin the original
reading thread, user/mount nsfs descriptors and current root. Deliberately follow
only the fixed `/proc/thread-self/ns/{user,mnt}` magic links, checking nsfs type
and NS_GET_NSTYPE. Bracket construction/revalidation with those original context
objects and root. The guard is not Send/Sync, cloneable, serializable, an FD
export or convertible to old bundle/start/signing authority.

Rechecks compare fresh full name walks and retained descriptors around positional
bounded hashing; metadata alone cannot establish content. Directory identity,
mode, owners and mount ID are stable, but unrelated entry changes need not
freeze all ancestor directory timestamps. Leaf metadata/content stay exact.
First error latches refusal; no replacement context/path/digest is accepted.

Capture old copy tests and an expected missing-module test before code. Exercise
actual root-owned system-file positives read-only and the existing user-owned
prepared copy's refusal. Private test-only owner/anchor seams may exercise owned
temporary trees; they cannot be selected through the normal public API. Cover
all metadata fields, missing statx support, every path bound, links/types/flags,
equal-byte file/ancestor replacement, same-size writes, sticky failure, context/
thread drift, descriptor release, seeded mutations and independent concurrency.
Run repeated focused tests, existing copy/bundle/issuer regressions, current/MSRV
checks, strict lint, source contracts, doc/gate/format and local performance
contracts. Retain a separate read-only local inventory of the three GNU runtime
files without ldd, execution or copying. Commit implementation before inventory.

This observes numeric root ownership in the reader's current namespaces. It
does not authenticate the initial host, filesystem implementation, administrator,
independent runtime/job policy, absence of retained foreign writers or a signed
receipt. Root/mount changes are not exercised on this machine. Those conditions,
real signing/token coordination, worker original-start, P2/P3 all-writer cessation,
completion cells 2–4 and server admission remain closed. No product workload or
new performance claim is authorized by this reader.
