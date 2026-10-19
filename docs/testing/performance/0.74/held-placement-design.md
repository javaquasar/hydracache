# Signed expected placement is not an execution capability

The [preregistered local package](held-placement-plan-20261019.md) implements
only the signed placement part of step four in the attempt-root plan. Source
`11e8a588a3b05b60f5c466e3fce7ca7ecb4ab923` adds a data-only v3 reader; it does
not replace the original v1 manager message or the existing v2 launch reader.

## Why another binding is necessary

The prepared original message still projects a direct worker in system.slice.
The guardians retain another attempt-specific root. The privileged fixtures
explicitly place dummy writers there. Sending the old message would neither
confine the original worker to that root nor hold it before validation. A
successful creator-death fixture is not evidence that this delivery is safe.

The new body binds the complete nested launch claims, an exact HCRG0742 Enroll
packet and a deterministic requirements digest under a separate signature
domain. It checks the packet's attempt digest using the unchanged original
identity tuple, including the same serde byte encoding. An Armed ACK is not an
Enroll request. Device, inode, original-context digest and absolute bound are
all covered; none is interpreted as an independently observed kernel fact.

Expected slice name is system-hc074 followed by the complete lowercase attempt
digest and .slice. Its only hyphen is the one after system, placing the expected
slice directly beneath system.slice. The existing fixed cell-one unit names
its expected child. Callers supply no path, command, FD, PID or launcher location.
The deadline is copied exactly, without refresh. Its equality with the actual
original live bound still requires the future owner join; signing it does not
perform that join.

## Reader boundary

Canonical newline-terminated JSON, strict unknown/duplicate-field refusal,
16384-byte body and 32768-byte envelope limits and a distinct v3 domain prevent
accidental v1/v2 signature reuse. The requirements digest includes the complete
existing v2 signing-message digest, not a claim that its old signature was
verified. The new outer supplied key authenticates the nested claims anew.
Production policy/epoch, issuer selection and enrollment remain separate.

The returned object has no runtime methods. All six admission fields and
renewal are false. It cannot build a complete held unit, arm guardians, access
a clock or descriptor, send, release a barrier, complete an attempt or unlock
the fence. Two compile-fail tests protect the most direct mistaken conversions.

## Verification and next boundary

The [separate evidence packet](local-runs/held-placement-20261019/README.md)
retains the old ordinary-image GNU/MSRV baseline before the missing-module
failure, followed by new tests and compiler receipts. Seeded identity cases,
all 104 packet-byte mutations, schema/domain/key confusion and malformed
documents are byte-reader tests, not a new SIGKILL or all-writer confinement
proof. No preparation-wall, allocation or product-speed improvement is claimed.

Exact additive source guards preserve all 423 previous sources, every original
test body, the external fault harness and old wire sections. Only two module
hooks and the new COMPAT section are removed for that comparison. Missing or
duplicate hooks and unrelated drift refuse; no live predicate is normalized.

Next, join these inspected claims with the same original owned manager and
guardians inside the complete live frame, checking actual pinned root identity,
context, bound and placement. Then project a separate complete held unit with
authenticated executable/stream roles and barrier, never the direct-worker
specification. Delivery uncertainty, autonomous revocation of every writer,
durable lifecycle and reconnect remain subsequent gates. No server operation,
supervisor maintenance or product workload is opened by this local package.
