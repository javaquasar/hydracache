# Original controller request retention

The opt-in `FencedControllerPreflight::retain_request` consumes the original
preflight and returns a runtime acknowledgement of one durable request attempt.
It is not a lifecycle publisher or execution capability. Its
[preregistration](diagnostic-controller-attempt-plan-20261015.md) and
[contract](diagnostic-controller-attempt-contract.toml) follow the approved P2
sequence; no server or workflow consumes this result.

## Original ownership and acknowledgement

The result retains the exclusive original OIDC/build/controller/worker/kernel/
configuration/authority borrow and moves the already held common fence into the
existing private attempt engine. No lock is released or reacquired, and no
second journal owner is opened. Public methods accept no replacement request,
token, credentials, timestamp, policy, path or descriptor.

Before mutation, the original five-check preflight must pass. The request must
be Reserve at expected revision zero. A private kernel-worker capture rechecks
the original authority under its original policy/context pins and retains the
policy digest, epoch and authority revision. The record also binds the exact
canonical request/hash, token digest, reviewed builder-policy/receipt digests and
all three original public-configuration digests. The signed request is not
reconstructed from a caller assertion.

The new exclusive filename is `diagnostic-controller-attempt-074.json` in the
original fixed intent leaf. Its canonical LF JSON schema is
`diagnostic-controller-attempt-074-v1`, bounded at 16384 bytes and registered in
[COMPAT](../../../COMPAT.md). A token digest is retained, never the bearer token
or a private key. The document is unsigned: its standalone validator checks
strict byte consistency, not authenticity or execution rights.

The reused storage engine creates only its own file with O_EXCL/no-follow,
retains its descriptor, checks exact inventory and original identity/metadata/
bytes, seals 0600 to 0400, syncs the file and original directory, and only then
acknowledges. At BeforeCreate, Created, PrefixWritten, BytesWritten, Sealed,
FileSynced, DirectorySynced and Acknowledging it rechecks the original controller
and exact captured binding. The final storage/controller/storage/controller/
storage bracket runs before return. These are sequential checks, not an atomic
snapshot or an additional total-operation deadline.

## Refusal and retained context

The first error stops progression and permanently refuses both original chains,
including authority and kernel packet. The failure owns the original storage
and fence until drop, retaining partial or complete bytes. A later refused
recheck returns before I/O; restoring paths or bytes cannot heal the guard.
This runtime latch does not append a durable authority-refusal record.

Healthy drop releases the original lock without removing the attempt. A fresh
empty-fence reservation refuses any retained leaf entry, regardless of request
or nonce. BeforeCreate remains unadmitted. Neither complete bytes nor a lost
acknowledgement can reconstruct a runtime guard. This conservative leaf exclusion
is not a global nonce ledger or protection against whole-host rollback.

The storage engine now has two private fixed-name constructors; legacy attempt
bytes and naming remain unchanged. Its legacy publication transfer explicitly
rejects controller documents. There is no conversion to fixed intent, Reserved,
Starting, completion or later-cell guards, and no automatic retry, repair,
adoption, cleanup or rollback migration.

## Local tests and next boundary

Private nonroot fixtures combine synthetic issuer/controller/OIDC statements
with actual local kernel message/context observations. Logical write-boundary
faults use seed `0x0740c094`. Tests cover canonical retention, lock lifetime,
unsupported signed operations, all eight interruption and trust-drift cuts,
late source/object/inventory drift, no-I/O refusal, strict reader mutations,
legacy conversion refusal and independently created concurrent chains.

The joined SIGKILL/lost-ack matrix remains a separate required process-harness
stage. Existing storage process-loss proofs do not establish the new composition.
No real issuer/job/host enrollment, original start, all-writer cessation, fixed
cells two through four or server admission follows from these local callbacks.

Runtime implementation `865dab48` and final source `f459aff1` have a
[local packet](local-runs/controller-attempt-20261015/README.md) with three
ten-case repeats, MSRV, the 67-case original-input and 124-case legacy-intent
regressions, nine separate existing process-loss proofs and eleven fence cases.
Those existing process proofs do not close the new joined boundary above.
