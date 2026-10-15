# Original GNU runtime and receipt consistency

The Linux normal-library join closes one specific gap: the signer-build receipt's
binary and runtime-manifest slots can now be checked against the same retained
original files. Previously those slots could be unrelated supplied byte slices,
including non-ELF and opaque runtime context. This new optional path does not
change the old verifier or turn its caller-selected expectations into policy.

## Acquisition and content binding

`retain_asserted_gnu_runtime` takes four path/digest assertions in fixed signer,
loader, libc and libgcc roles, an `AssertedSignerBuildSelection`, a receipt and
three supplied build companions: lock, build log and bootstrap context. It
checks every image's 64-byte..8 MiB bound and SHA256 syntax, then authenticates
the exact expected statement's signature under that selection before file IO.
The already selected signer issuer may be used through its existing factory;
no new issuer override, private-key operation or actual receipt producer exists.
Expectations must not be derived from the receipt being checked.

Acquisition calls the existing public root-owned reader for every role: numeric
UID/GID 0, current `/`, strict full-ancestor descriptor walk, required statx fields,
original reading thread and user/mount namespace checks. No path copy, chmod,
chown, owner/anchor override or arbitrary caller FD is accepted. That reader's
policy, limits and ordinary inventory API stay unchanged.

An internal bounded read brackets positional reads of the original leaf with
the reader's complete context/name/metadata/content check. Exact length, EOF and
digest must match. It preserves file offsets and exposes no public FD/bytes API.
The shared GNU parser checks those four snapshots. Canonical sorted-key JSON
plus exactly one LF of the resulting catalog is the runtime-manifest content;
the actual signer snapshot is the binary content. The existing receipt verifier
then matches all five artifact slots. Consequently even a correctly re-signed
opaque, noncanonical or other-runtime manifest refuses this join.

## Retention and refusal

`AssertedGnuRuntimeRead` owns all four original readers and retains only diagnostic
receipt/catalog hashes in addition to their bounded descriptor inventories.
Image snapshots are dropped after construction. A subsequent check revalidates
every original path, context, metadata and digest. It never adopts a replacement
inode, including an equal-byte replacement. A failed acquisition refuses acquired
inputs before drop; a failed join/recheck refuses every retained input. Refusal
is sticky and prevents later IO, even if original names or bytes are restored.

Construction bounds image buffers to four times (8 MiB plus one EOF-probe byte),
alongside the existing bounded ancestor/namespace inventories and GNU catalog.
Those image buffers are not retained by the returned owner. This is a resource
bound, not an allocation or throughput measurement.

The type is noncloneable, nondeserializable and thread-bound through its owned
readers; it exposes neither descriptors nor bootstrap/bundle conversion. No
controller, server or token/signing consumer accepts it. Construction is read-only
and publishes no persistent record, so it adds no SIGKILL write boundary.

These remain sequential observations. Numeric root in a reader's namespace is
not independently authenticated initial-host, filesystem, launcher or job policy.
Allowed root-writable mode bits do not revoke already-open writers; ownership
checks do not prove ACL/mount administration or absence of root mutation. The
join does not prove atomic four-file lifetime, loaded/relocated memory, later
loads, TLS/config inputs or original invocation. Its synthetic test signatures
establish byte/signature consistency, not actual production receipt custody.

## Tests and remaining production work

Eight Linux unit cases cover positive synthetic original roots, public rejection
of those nonroot fixtures, all-role budgets before IO, wrong signatures/domains/
keys, re-signed manifest substitutions, all companions, role/binary mismatch,
equal-byte replacement of each role, sticky all-input refusal, content/ancestor
drift and four independent thread-local owners. Seed `0x0740c077` exercises 64
correctly signed other-catalog substitutions. A separate root-reader test covers
bounded snapshot offset preservation and sticky size/content refusal. Existing
GNU integration cases share only their synthetic image builder with these tests;
the production parser and graph are not duplicated. Five compile-fail doctests
cover Clone, Deserialize, FD export, thread transfer and bundle conversion.

The [preregistration](diagnostic-signer-runtime-join-plan-20261015.md) and
[contract](diagnostic-signer-runtime-join-contract.toml) keep actual keys/tokens,
host/GitHub mutation and numerical admission outside this stage. Independently
admitted original runtime/root/launcher and protected signing-job receipt remain
before production bootstrap; authenticated worker original-start and every P2/P3
writer's cessation still precede completion, cells 2–4 and server rehearsal.
