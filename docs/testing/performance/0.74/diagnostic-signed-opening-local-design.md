# Credential status opening inside signed reader context

## Implemented local safety slice

The fixed/fixture constructors now borrow the original account/context and
process generation while privately owning fresh namespace-checked credentials.
The old public namespace constructor body and parser stay unchanged. A separate
crate-private opener pins status without observing its credential projection.
The post-open original account/context check must pass before that first read;
later observations never reopen or replace the object.

The [packet](local-runs/diagnostic-signed-opening-20261011/manifest.json)
retains the 43-test portable baseline, missing-opening-API E0432, 15-test first
green, 340 Linux passes (one existing system-bus ignored), 43 Windows passes and
133 root contract/evidence/governance passes. Fifteen owning reader tests and one
private opener test pass three additional repetitions. Seed `0x7572026` varies
256 modeled failure positions per run. Scoped format/check/strict lint and
documentation checks are retained; ship admission is expected-red.

Actual owned NNP helpers with synthetic signed account fixtures cover revocation,
valid signed UID/GID/exact supplementary mismatch, process exit, restoration/drop
and concurrency. A private opener test confirms no projection before revalidation,
including an unhardened helper that refuses only on reading credentials.
A deterministic private boundary seam mutates the real account fixture after
opening status: the post-open account error stops before the projection stage.
Modeled stage tests cover every first-error position, not exhaustive kernel races.

Construction makes one bounded clone of at most 32 original supplementary GIDs;
subsequent observations do not clone that list. This diagnostic cost is not a
product allocation improvement. No performance or production-enrollment claim
follows. Frozen qualification inputs remain exact. Worker mount/all-thread proof,
kernel opener credentials, real issuer/durable epoch/refusal and authenticated
original start remain independent outstanding authority work.

The standalone [leader mount guard](diagnostic-mount-namespace-local-design.md)
now adds local same-mount consistency. It is not yet composed into this signed
opening reader, so its historical mount/all-thread proof boundaries stay closed.

## Scope and baseline

Baseline `70064e71078c587f79d371fa611c78a2e53b9ff8`. Add opt-in Linux
fixed and owned-fixture readers borrowing the original context/account guard and
retained ProcessRead, and privately owning freshly opened namespace credentials.
No already opened numeric or namespace credential guard, PID, pathname, numeric
policy or replacement process selector enters. Existing public numeric/namespace
constructors and the earlier borrowed binding retain behavior.

## Opening and later order

Require context/account, open status, context/account, credentials, context/account.
The first account observation verifies original external trust/envelope/assertions
and actual reading-thread context before process/status IO. Derive numeric UID,
primary GID and the exact supplementary list only from its original checked policy:
one bounded list clone, at most 32 entries, no primary union or caller numbers.
A crate-private opener establishes original shared user namespace, brackets status
opening with namespace/generation checks and performs no credential projection read.
It owns the new namespace/status objects. The post-open account stage must succeed
before the first credential observation; original numeric parser/generation rules
then pin the projection, which must match original signed mapping exactly.
Original namespace credentials retain namespace/credentials/namespace revalidation.
Later outer order is account/credentials/account without any opener or refresh.

Any first error stops and refuses borrowed account/context plus original policy
and any already-created owned credential/namespace guards. Failed construction
drops owned objects; policy refusal persists. Later refusal persists after restored
files and wrapper drop. Healthy drop preserves the borrowed account/policy and
process remains read-only, never killed/reset. Both origin types are private-field,
neither Send nor Sync, with no guard/FD/numeric extraction or public callbacks.

This is sequential status opening and read consistency, not inspection of kernel
file-opener credentials, atomic/continuous proof, worker mount namespace/all-thread
attestation, initial host authority, durable issuer revocation or production start.
A retained ProcessRead is still a generation observation, not authenticated start.
Owned synthetic account fixtures and NNP test processes do not enroll operator keys
or a valid production account. No wire/receipt schema or cache path changes.

## Tests and gates

Record portable baseline and missing API first. Test exact opening/later order,
all first failing positions and typed errors, prior refusal without IO, nontransfer,
no projection read in the private opener, original namespace/status guards,
derived mapping mismatch, context/account failure before process observation and
after open, failed opener, NNP refusal, original exit, replacement/restoration,
healthy drop, independent concurrency and seeded refusal-stage cases. Actual
process/context positives are unprivileged owned NNP cat helpers under synthetic
issuer policy/files; status-open gap injection remains private modeled coverage.
Never mutate real identity files/namespaces or inspect a valid production account.

Run scoped Linux/Windows supervisor tests/check/strict lint, root contracts/evidence/
governance, formatting, docs/links/book, final registry and expected-red require-ship.
Retain immutable raw logs and negative results, preserving frozen qualification
inputs and all prior packets. No infrastructure, workloads, performance claims,
production preparation, authenticated lifecycle, qualification or full-workspace
milestone is authorized. Rollback removes this optional runtime API by old binary.
