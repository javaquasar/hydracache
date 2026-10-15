# Original root owned input inventory in the reading thread context

The new Linux reader verifies an explicitly named input's numeric ownership,
full ancestor chain and bytes while retaining its original reading context.
It closes the previous copy inventory's missing ancestor check without promoting
that user-owned copy into a trusted bundle. Path and digest remain assertions;
initial-host, filesystem, runtime and job enrollment are separate requirements.
The [preregistration](diagnostic-root-owned-input-plan-20261015.md) and
[contract](diagnostic-root-owned-input-contract.toml) define the scope.

## Original path and context

The public constructor fixes numeric UID/GID 0 and the current `/` anchor. It has
no owner, namespace, FD or fixture override. Strict lexical validation precedes
IO: absolute path only, no empty/dot/dotdot/NUL/trailing components, at most
4096 path bytes, 32 components and 255 bytes per component. Expected SHA256 and
nonzero size up to 128 MiB are explicit inputs.

Every path component is opened descriptor-relatively, read-only with NOFOLLOW,
CLOEXEC and NONBLOCK. The guard retains the root, every ancestor and the original
leaf. Directories must be 0555/0755 and leaves regular, single-link and one of
0444/0555/0644/0755, with numeric owners 0/0. This system-file inventory profile
does not change the old bundle reader's exact 0555 requirement. No permissions,
owner or existing file is repaired.

statx must return every required type/mode/UID/GID/link/size/time/inode/mount-ID
field. Missing support refuses; device/inode is not a fallback for mount identity.
The [statx manual](https://man7.org/linux/man-pages/man2/statx.2.html) distinguishes
requested fields from the returned support mask and ordinary mount IDs from the
newer unique-ID facility. This reader uses ordinary mount IDs, not an asserted
global, cross-reboot or cryptographic identity.

The original reading TID, user/mount namespace objects and current root are
retained. Only fixed `/proc/thread-self/ns/user` and `mnt` magic links are followed
deliberately, with nsfs filesystem and NS_GET_NSTYPE checks. The
[namespace ioctl manual](https://man7.org/linux/man-pages/man2/ioctl_nsfs.2.html)
describes these kernel type queries. The guard is neither Send nor Sync, and
context checks bracket fresh path walks and bounded positional hashing. It
cannot be cloned, deserialized, exported as an FD or converted to a bundle.

Revalidation compares original descriptors and fresh names before and after
hashing, then walks the path again. Leaf metadata and content stay exact.
Ancestor device/inode/mount/owner/mode must match, but unrelated directory entry
changes do not freeze ancestor timestamps or lengths. First error permanently
refuses the object, before further IO on subsequent calls. No refresh API exists.

## Tests and observed input versus custody

The actual `/usr/bin/true` file is read but never executed. Owned temporary-tree
positives use a private test-only owner/anchor seam; the public numeric-root
entry rejects those inputs. Tests cover missing statx fields, all 13 stamp fields,
mode/owner/type/link/size faults, every path bound, FIFO/symlink/hardlink refusal,
equal-byte file and ancestor substitution, hash drift with deliberately reset
test metadata, context/thread/root drift, readonly/offset/drop behavior and
independent observers. Compile-fail cases enforce six API restrictions.

These are sequential observations in the caller's current namespaces. Numeric
root does not authenticate an initial host; same namespaces do not establish
an administrator or filesystem trust policy. Real namespace/mount replacement
is not performed; those drift tests mutate private expected observations. Root
can change writable system files, and a foreign retained writer is not revoked
by this read-only guard. No ELF declarations, loaded closure, later-load inputs,
TLS trust or mapped memory are certified by a digest and ownership alone.

The local example may inventory explicitly named GNU runtime files and must
refuse the previous user-owned prepared controller. It performs no chmod/chown,
copy, ldd, binary execution, signing, token request or job dispatch. Independently
selected runtime/job policy and signed receipt acquisition still precede actual
signer custody. Original worker start, all P2/P3 writers, completion cells 2–4,
server rehearsal and qualification remain closed.
