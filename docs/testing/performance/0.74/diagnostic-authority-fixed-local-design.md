# Fixed authority journal: local implementation scope

Baseline: `bf08300cbdfa60876dd9b904b3bf8f6b6a5427aa`. This slice implements a
separate Linux fixed-path backend, not enrollment or a production launch.
The previously registered fixture contract remains historical and unchanged.

The fixed writer uses `/var/lib/hydracache-performance/worker-authority-074`.
Its public constructors accept checked policy or independent trust/context only:
no path, UID/GID, descriptor, fixture conversion, environment override or retry.
Effective UID and GID must both be zero, but numeric root is **not** proof of the
initial host namespace. No constructor creates directories or selects issuer pins.

Open `/` and each fixed component with no-follow, directory and close-on-exec
flags. Retain original descriptors and identity/ownership/mode stamps. Require
root:root ancestry, no group/other write or special mode bits, and an exact 0700
leaf. Reinspection brackets publication and compares every original object with
its current named object; ordinary directory timestamp/link-count changes are
not identity drift. Cross-filesystem ancestry is permitted; each journal file
must remain on the leaf filesystem. Mount/namespace enrollment is a separate gate.

Reuse the existing append-only engine: 128 generations, 2048-byte records,
65-byte commit markers, exclusive nonblocking flock, create-new publication,
file/directory fsync, exact independent bindings and sticky in-memory failure.
Explicit initialization requires an already provisioned empty leaf. An explicit
refusal is durable only after successful publication; other errors do not safely
become automatic journal entries. Cached numeric views are not fresh admission.
No migration, repair, truncation, automatic restart, epoch allocation or eviction.

Local verification uses the production metadata predicate on synthetic stamps,
the production ancestry walker on nonroot temporary directories via a private
test-only seam, the unchanged engine's fixture fault corpus, and exported API
shape/nonroot rejection tests. No root directory is created or modified, and a
positive modeled ancestry test must not be called positive fixed-host enrollment.
Test ancestor replacement, descriptor replacement, owner/mode policy, symlinks,
missing paths, locking, persistence and late/partial publication. Retain the
missing-API red before implementation, baseline and green raw output.

Signed worker guard durable propagation, actual enrolled root-path positive
tests, real issuer/pins, reboot/power-loss behavior and production start remain
unimplemented or unproved. Root/owner tampering and whole-host snapshot rollback
remain outside this local journal proof. The inherited qualification plan-digest
mismatch is not reconciled here; all frozen qualification inputs stay unchanged.

## Review observations

The named-root replacement model first fails: the original descriptor remained
valid while its current name changed. Revalidation now compares a fresh named
root to the original stamp before walking retained ancestry. This is a temporary
root model, not an actual host chroot/pivot-root experiment. A separate existing
same-byte rewrite test exposes clock-granularity assumptions; its setup forces
observable mtime drift without changing runtime predicates. Both findings and
their independent fixes are retained in the local packet.
