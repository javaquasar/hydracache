# Original signer file input retention

The Linux reader now retains seven real descriptors: a bundle directory and its
six original files. It validates the existing separate signer receipt against
streamed file hashes and makes the first subsequent name/metadata/content
refusal sticky. This is an explicitly asserted local fixture boundary, not
production issuer selection, trusted ancestry or mapped executable/runtime.

## Exact original inputs

`retain_asserted_bundle(path, uid, gid, selection)` consumes a local asserted
selection and returns private `AssertedSignerBundle`. The absolute normal path
is bounded to 4096 bytes; symlinks and dot/traversal/empty components refuse.
Each component is opened descriptor-relative, no-follow, read-only, nonblocking
and CLOEXEC. The old fixed observer install root is explicitly refused. The
reader checks the final directory owner/mode, not independently trusted ancestors.

The six names are `diagnostic_controller`, `signer-build-receipt-v1.json`,
`Cargo.lock.root`, `build-log.jsonl`, `runtime-manifest.json` and
`bootstrap-context.json`. Enumeration retains at most six names, refusing an
additional or unexpected entry. The directory/binary require mode 0555 and
other files 0444; children must be nonempty regular single-link files with exact
asserted UID/GID and no special permission bits. Symlink, hardlink, directory
and FIFO substitutions refuse. Nonblocking open avoids waiting for a FIFO writer.

The existing signer model's byte ceilings remain exact. A 64 KiB scratch buffer
streams each original file. Only the receipt gets a temporary bounded byte copy
for strict JSON/signature verification; full binary/log/context bytes are never
retained. One private receipt core preserves the public supplied-byte verifier.
A crate-private digest comparison joins the original reader to that same core;
there is no public digest-to-authority constructor.

## Recheck and refusal

`check_original(&mut self)` takes no replacement input. It walks the original
path for identity comparison, enumerates the retained directory, reopens each
name relative to that directory, and hashes the original descriptors. Dev/inode,
UID/GID/mode, link count, length and nanosecond mtime/ctime are compared before
and after reads. Directory membership and root identity are bracketed again,
including a final no-follow path walk. Equal bytes under a replacement inode
are not the original file. Reopening names is comparison, not reacquisition of
authority or replacement of retained handles.

Every error latches refusal for that object. Restoring content/names cannot
repair it. Success returns only a borrowed diagnostic receipt digest, not a
File, FD, path, checked build or start/signing permission. The result cannot
clone or deserialize. Dropping it releases all originals through File RAII.
These checks describe observed intervals; they cannot make a future path exec
atomic or prevent a privileged writer acting after the last observation.

## Local evidence and remaining custody

The [preregistration](diagnostic-signer-bundle-plan-20261014.md) fixes owned
nonroot fixtures, synthetic keys and the unchanged signed profile. Tests cover
initial refusal, original/equal-byte replacement, file/root restoration,
in-place and membership drift, bounded chunk hashing, CLOEXEC and descriptor
release. A private child check isolates inode refusal even when root metadata
also changed; it is not a repair API. Seed `0x07403b23` covers 32 substitutions
and four independently retained originals. Compile-fail/API checks keep the
guard's exports and replacement arguments closed.

Runtime/bootstrap manifests remain opaque signed assertions. No interpreter,
loader, shared-library mapping, actual process `/proc/exe`, authenticated build
job, trusted issuer/pins, secret custody or token-bearing job is certified.
This reader is not connected to the producer, CLI, server or execution route.
The later [executable object observation](diagnostic-signer-executable-design.md)
borrows this same original bundle and an existing ProcessRead. It detects
observed different-inode exec and refuses both wrapper/bundle, but its actual
same-inode re-exec counterexample leaves original invocation and complete
runtime integrity unproved. It exports no authority and enables no producer.
Those independent selections and original mapped-process/runtime observations
must precede a reviewed pre-exposure bootstrap and credential coordination.
Original replay/common fence/start, all-writer cessation and later fixed cells
remain prerequisites for any separately admitted server/pilot/qualification.
