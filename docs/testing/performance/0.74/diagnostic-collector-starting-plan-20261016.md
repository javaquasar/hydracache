# Durable collector Starting publication

Baseline 4fbc4f3b accepts original kernel start consent in memory but writes no
new record. The next work packet consumes that owner into a first-cell Starting
intent before any output, process or manager invocation. The existing lease,
controller and kernel admission deadlines are fixed, not implementation knobs.

The [publication budget experiment](diagnostic-consent-budget-plan-20261016.md)
now retains twelve refusing local chains, including normal release builds.
The straightforward recursive-check publication path is blocked before any
Starting write. The operator subsequently approved the separate
[validation frame design](diagnostic-validation-frame-design-20261016.md).
Its local candidate changes only outer orchestration and retains the original
live leaf guards, origins and deadlines. It is not an implemented publisher:
the mixed release-budget observations and incomplete live-mutation matrix must
not be interpreted as permission to start a process or acknowledge storage.

## Record and phase ownership

Register a separate opt-in diagnostic-collector-starting-074-v1 record, fixed
name diagnostic-collector-starting-074.json and canonical LF budget 16384 bytes.
Bind the original controller-attempt and collector-Reserved record digests,
exact authenticated signed consent, its complete LF digest and the identical
signed Starting state. Unknown, duplicate, future, noncanonical, partial or
over-budget records refuse. Byte consistency inspection cannot acknowledge,
recover or replay an owner. Old two-document and legacy Starting readers must
refuse, not reinterpret the new history.

Use an explicit private storage phase entered only by consuming the admitted
consent. Preserve original descriptors, names, inode/metadata/byte checks and the
common fence. The default Reserved phase must retain its exact two-file inventory;
it cannot accept an arbitrary third document. Only the consuming new phase may
recognize its one exclusively created Starting document. No caller path, FD,
replacement consent, reconstructed kernel object or second storage owner enters.
No API returns the consumed Reserved/consent owner for another attempt.

Latch the attempt before exclusive creation. Validate the original inputs and
known inventory before/after every create/write/seal/sync/ack boundary. Seal and
sync the original file, sync its directory, then acknowledge in memory. Any
failure retains refused owners and complete or partial context until explicit
drop. Do not overwrite, remove, repair, adopt, retry or publish a refusal as a
substitute for the lost original acknowledgement.

## Baseline and budget prerequisite

The original-consent packet retains a default-debug expiration and a successful
separately declared optimized-crypto correctness lane. It establishes no budget
margin for nine additional write boundaries. First measure the exact unchanged
publication-free admission/check cost and preregister any required proof-work
reuse as a separate candidate, with a functional reference and live time/peer/
trust/epoch/descriptor revocation checks retained. If the first publication
candidate expires, retain that negative result; do not extend windows, re-anchor
old clocks, skip checks or retry it into a green claim.

## Tests before execution

Add a red missing-entry/API test before implementation. Cover seeded record
mutations, exact original projections, two-versus-three-file phase refusal,
exclusive creation, symlink/hardlink/rename/metadata/byte drift, competing owners,
key/epoch/trust/time refusal and partial I/O. Require one consuming attempt and
negative clone/deserialize/retry/start APIs. Preserve old source predicates with
exact narrow static restoration and mutation canaries; register the new artifact
in COMPAT rather than widening old reader compatibility.

Define separate BeforeCreate, Created, CreateDirectorySynced, PrefixWritten,
BytesWritten, Sealed, FileSynced, DirectorySynced and Acknowledging cuts. A
bounded nonroot fixture must construct its own original chain in the child;
copying a live owner across fork is not a new original kernel context. The parent
owns the child's pidfd and temporary root. SIGKILL all nine cuts plus AfterReturn,
run an uninterrupted control and inspect every retained state from fresh
processes. Fresh inspection must preserve bytes/metadata and refuse permission;
even a complete record does not reconstruct successful runtime acknowledgement.

The common flock is an in-memory exclusion, not a global durable replay proof.
Retain the existing negative control that raw legacy campaign primitives do not
consult all fixed history after an owner disappears. Actual server enrollment
must route admission through the fixed-context checks; neither this new record
nor a successful local crash test may claim that bypass routes are closed.

Run current/MSRV focused tests, strict affected-package check/Clippy, unchanged
IPC/lease/fence/host regressions, static/docs/registry/local contract checks and
the expected-red ship guard. Milestone verification, signed installation and
bounded server rehearsal remain ahead. This work packet creates no root output,
process start, watchdog, cleanup authority, completion or later cell; no product
workload, pilot or qualification is admitted.
