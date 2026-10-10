# Fixed local account files under the signed worker policy

## Scope and preregistered refusal rules

Baseline is `4f7336a82eb5529f20609c6494dacec03664c55a`. Extend the approved
[local policy model](diagnostic-worker-policy-local-design.md) with a Linux-only,
read-only inspector for `/etc/passwd` and `/etc/group`. It borrows the original
checked policy and revalidates supplied current trust, envelope and asserted
context before filesystem access. An inspection or revalidation failure refuses
that original policy permanently, including after the file wrapper is dropped.
No policy, descriptor, UID/GID or start capability is exported.

Production inspection accepts only the fixed names beneath root-owned,
non-symlink, non-group/world-writable `/` and `/etc`. Retain directory descriptors
and identity/security metadata; unrelated directory timestamp changes need not
refuse. Leaves must be regular, singly linked, root-owned files without execute,
special or group/world-write bits. Open descriptor-relative with NOFOLLOW,
CLOEXEC and NONBLOCK; reject unsafe kinds before reading. These flags do not
provide a deadline for regular-file storage IO. Retain leaf device/inode,
ownership, mode, size and mtime/ctime nanoseconds. Reopened names and retained
descriptors must still match; no substitution or refresh is permitted.

Each file is nonempty and at most 1 MiB, with at most 16384 records and 4096 bytes
per line. A single bounded positional read per file must return exactly the
metadata length, with unchanged before/after metadata. Two complete rounds
bracket named-directory/leaf checks and compare the original whole-file policy
digests and mapping. This is sequential point-in-time evidence, not atomic
cross-file observation, continuous host proof or privileged writer revocation.
Concurrent trusted account maintenance conservatively refuses; retry requires a
new separately approved policy/guard, not restoration of the original.

Use a conservative ASCII, LF-terminated local-file grammar: seven passwd fields,
four group fields, canonical decimal IDs, nonempty unique names and unique numeric
IDs, no comments/blank lines, CR/NUL, compat `+`/`-` records or malformed member
lists. Root entries are permitted; the worker remains nonroot. Require exactly
one fixed worker user and group, matching signed UID/primary GID. Explicit group
records listing the worker must exactly match signed supplementary GIDs, not an
inferred primary union or an NSS result. Reject unknown local member names.
Whole documents, including unrelated entries, are digest-bound.

An explicitly named temporary fixture API uses caller-asserted nonroot ownership,
walks ancestors without symlinks but does not certify production ancestry, and
returns a distinct fixture type with no production conversion. Reject `/etc` and
descendants as fixtures. Both origins share parser, descriptor and refusal code;
fixture positives cannot enroll a real host, issuer or namespace. Kernel host/boot/
user/mount enforcement, all-thread checks, durable epoch/refusal, production
preparation and authenticated original start remain separate prerequisites.

## Tests and local gates

Record the Windows policy/artifact/library baseline, then a Linux missing-API
test before implementation. Test valid exact groups, primary inclusion and empty
groups; re-signed malformed/ambiguous records, mapping and whole-file mismatches;
limits and growth; symlink/hardlink/FIFO/directory and ownership/mode refusals;
leaf and directory replacement, in-place rewrite/restoration; revoked trust and
already-refused policy before IO; repeated and independent concurrent fixtures.
Retain seeded valid mutations and all negative results. Use only owned temporary
files and synthetic issuer/context; never modify real account files or enroll a
host. Run affected Windows/Linux check/strict lint, scoped rustfmt, focused tests,
root contracts/evidence/governance, docs/links/book and expected-red require-ship.
Keep qualification inputs, old account/NSS/namespace APIs and live routes exact.

## Implementation and retained local results

Implementation `7d3b276973893f53553528b2dacfb2d67eda5348` adds the Linux child
module, separate fixed/fixture wrappers and parser/descriptor guards, plus the
platform-target registry and root scope check. No policy wire schema changes.
The [local packet](local-runs/diagnostic-worker-files-20261011/manifest.json)
retains the 43-test Windows baseline and final portable regression scope; the
Linux-only target contains zero Windows cases. Final Linux scope passes 283
(223 library, 29 artifacts, 2 manager, 3 account, 13 policy and 13 file tests),
with one existing system-bus case ignored. The 13 new integration cases and one
new positional-read unit case pass three additional Linux repetitions. Seed
`0x7522026` drives 64 valid unrelated-document mutations per repetition: new
matching policy pins accept each fixture, original whole-file pins refuse it.
Exact 1 MiB, 4096-byte line, 16384-record and 32-group positives pass as well as
their over-budget/unsafe/ambiguous counterparts.

Negative evidence remains intact. Before implementation the missing-API check
hit Rust 1.94's diagnostic-renderer ICE while formatting unresolved-import E0432;
a baseline API replay repeated that ICE. The same absent module with Cargo's short
diagnostic format returns ordinary E0432/native exit 101. The first full Linux run
failed the injected same-length-write metadata assertion; the injection now
changes length deterministically and the full scope passes without sleeps or
timestamp-resolution assumptions. The first root gate caught an unregistered
Linux cfg target; registration under the existing CI step fixes that omission,
without changing the workflow, an old gate or a qualification threshold.

Root contracts/evidence/governance pass 129 checks. Windows/Linux affected package
check and strict lint, scoped format, doc/local contract, links and book pass;
require-ship remains expected native exit 1. All actual frozen qualification
contract inputs and product/native/NSS/namespace/live-route code are unchanged.
These are working-tree local safety gates, not full-workspace or release
qualification. No positive production ancestry/context, real issuer enrollment,
host mutation or performance result is established. Next: observe and enforce
actual admitted kernel/host context before composing this reader with credentials.

The subsequent [observed Linux context guard](diagnostic-worker-context-local-design.md)
adds a separate read-only comparison of real machine/boot and typed user/mount
namespace observations against the original policy. This account reader still
accepts assertions; sequentially dropping one wrapper and constructing another
is not an atomic context/account/credential join. That composition is the next
prerequisite, with production preparation and start still closed.
