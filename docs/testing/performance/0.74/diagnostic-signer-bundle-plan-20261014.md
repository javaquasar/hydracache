# Original signer bundle descriptor retention

Baseline is `af8451ed215cf77a20926435e4f77cf06c6aa3ba`. The next local Linux
step retains real file descriptors for an explicitly asserted signer bundle.
It proves original file-input consistency, not independently selected issuer,
trusted ancestry, mapped executable/runtime or bootstrap authorization.

## Fixed local scope

Add Linux-only `diagnostic_signer_bundle::retain_asserted_bundle(path, uid, gid,
selection)`. All path/owner/selection arguments are explicit assertions, never
production configuration. Reject the old fixed observer install root. Walk
absolute normal path components with descriptor-relative O_NOFOLLOW,
O_DIRECTORY/O_NONBLOCK/O_CLOEXEC and no creation/write flags. Retain the final
directory plus exactly six original files, with no more than six directory
entries collected: `diagnostic_controller`, `signer-build-receipt-v1.json`,
`Cargo.lock.root`, `build-log.jsonl`, `runtime-manifest.json`,
`bootstrap-context.json`. Root/binary mode is 0555, other files 0444; asserted
UID/GID match, regular files have one link and no special permission bits.

Bounds stay those of the signer model: 8192 receipt bytes, 128 MiB binary,
1 MiB lock, 16 MiB log and 64 KiB per context. Stream hashes in 64 KiB chunks;
retain only receipt bytes, descriptors, stamps and digests, never the full
binary/log. Refactor existing signer-model receipt validation into one private
core with a crate-private original-digest comparison for this module. Keep its
public supplied-byte behavior, domain/schema and key-role restrictions exact.
No general public digest-to-authority factory is added.

Capture dev/inode/UID/GID/mode/link count/length/mtime/ctime before and after
reads. `check_original(&mut self)` accepts no replacement paths, policy, bytes
or descriptors. Reopen names only for identity comparison with the retained
root/files, then hash original descriptors and bracket metadata and root
membership again. First error sets sticky refusal; restoring original bytes
or names cannot repair that object. Return only a borrowed receipt digest,
no File/FD/path, checked-build conversion, Serialize/Deserialize/Clone or
execution/signing capability. Drop releases original descriptors through RAII.

This is read-only inspection of public fixture artifacts. No actual signing
key/token, process execution, /proc executable mapping, dynamic-loader/runtime
inspection, trusted root/ancestor enrollment, service, workflow or server route.
No durable/wire schema changes: the existing signer receipt remains exact.

## Tests and gates

Capture baseline signer/artifact cases, then missing-module compile refusal.
Use owned nonroot Linux temporary files and explicit synthetic signatures only.
Cover original positive; signature/content mismatch; missing/extra/oversized/
empty entries; relative/traversal/symlink ancestry; symlink/hardlink/directory/
FIFO children; unsafe modes/UID/GID; replacement of equal-byte files/root;
in-place writes; later restored names/bytes with sticky refusal; CLOEXEC and
original descriptor release. Seed `0x07403b23` covers 32 fresh file substitutions
and four independent concurrent guards. No wall-clock correctness assertions,
SIGKILL, product workloads or installed artifact reads.

Add compile-fail no-clone/no-deserialization/no-FD-export examples and an
integration API signature check. Run three Linux focused repeats, portable
signer and existing artifact/producer regressions, doctests, Rust 1.94 package
check/strict Clippy and MSRV 1.88, plus Windows portable regression/check/lint
and root contract/documentation/local performance gates. Record raw logs,
tested-source and unchanged frozen hashes. Keep preregistration, tested code
and evidence/article commits separate and push only after focused green gates.

Independent production issuer/pins, original mapped executable/runtime and
reviewed pre-exposure bootstrap, retained-body token coordination, original
receiver/replay/fence/start and all-writer cessation remain before server or
qualification admission.
