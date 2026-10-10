# Credential status opening inside signed reader context

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
