# Preparing a separate unsigned prebuilt file

Baseline is `a7819987b3a7f5b4efea7925abeec1b29b9995a3`. The previous actual
unsigned Cargo output is 0755 with two hardlinks. Do not chmod or unlink that
source, sign it, execute it or call a copy authenticated original custody.

Implement an opt-in Linux preparer over an already opened read-only source and
an explicitly selected, owner-only parent directory descriptor. Expected bytes
and SHA256 are asserted inputs, not a build receipt or production policy. Create
one absent bounded attempt name with mkdirat and one exclusive fixed child with
openat/O_NOFOLLOW/O_CLOEXEC. Copy positionally with a fixed buffer and exact byte
budget, bracket original source metadata, and match the expected digest. Reject
special files, writable input descriptors, malformed expectations and unsafe
parent mode/ownership before creating anything. Never follow supplied paths,
overwrite, resume, unlink, repair modes on existing files or run external tools.

Only the newly created output receives 0555 after copying and file sync. Check
the sole owned writer's explicit close result without retrying close on Linux;
reopen read-only through the original directory, rehash, require a distinct inode
and single link, seal the new directory to 0555 and sync it and the parent.
Return a non-cloneable inventory guard, not a File/raw FD or execution capability.
Revalidation uses original descriptors and names with sticky refusal. Failed
attempts stay on disk and the same name cannot be retried, even if it looks complete.

This closes only copy preparation and this function's own writer lifetime.
An untrusted same-UID process or administrator can still acquire a writer during
preparation or change ownership/modes afterward. Retained foreign writable FDs
survive chmod; test that counterexample explicitly. No scanner or checksum may
assert whole-writer revocation. Independently protected ancestry/account/runtime/
job custody and actual signed receipt acquisition remain before production use.

Before implementation retain old bundle tests and an expected missing-module
failure. Add deterministic copy/fault/mutation tests, no-overwrite and concurrent
same-attempt/independent-attempt checks, source alias and source-mutation cases,
destination substitution and sticky refusal, owned writer flag/close checks and
compile-fail FD/clone/authorization guards. Exercise write/read short progress and
failure seams where possible; injected failures are not real SIGKILL or power loss.
Run Linux repeated focused tests, existing bundle/executable and issuer regressions,
current and MSRV checks, strict lint, source contracts, docs and local non-promotable
performance contracts. Preserve frozen qualification and all old pin/reader bytes.

A separate local example may prepare the previously inventoried unsigned binary
once, using its retained expected hash/size and a fresh parent under local cache.
It must not execute the binary, read a key, sign, request credentials, activate a
workflow or access a server. Commit code first, retain source/output identities
and mode/link observations afterward, and update the article with the remaining
foreign-writer limitation. No product performance claim or release admission follows.
