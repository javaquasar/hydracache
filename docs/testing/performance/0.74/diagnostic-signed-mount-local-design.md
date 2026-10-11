# Original signed credential and leader mount composition

## Scope and baseline

Baseline `b07a655bb42237b3eb80e774b953017e39850dd3`. Add separate fixed/fixture wrappers
borrowing the original signed-opening reader and privately owning a newly opened
mount guard for exactly its retained process. Public binders accept no process,
PID, path, preopened mount reader, numeric or replacement context selector.
Old signed-opening, numeric and standalone mount API behavior stays unchanged.

A crate-private namespace-credential adapter creates the mount guard only from its
own original process reference and refuses either unhealthy credential component.
It returns no process/FD/identity getter. A private mount-policy boolean compares
original device/inode with the original checked signed mount pin. No new policy
or namespace assertion is adopted from an observation.

Construction is signed/open-mount/signed/mount/signed. Each signed stage retains
account/credentials/account including context/files/context and user-namespace
credential checks. Open-mount uses only the original retained process; post-open
signed context must pass before the outer mount stage. Mount stage requires
original signed mapping equality and existing mount-generation observations.
Later order is signed/mount/signed, without reopening.

Prior refusal stops before IO. First typed error stops and latches the borrowed
signed reader, original account/context/policy, both credential components and
any newly owned mount guard. Restoration/drop cannot reset refusal. Healthy drop
keeps original signed reader and process usable. Fixed/fixture wrappers remain
distinct and neither Send nor Sync. No descriptors or public callbacks are exposed.

This is leader mount consistency joined with original signed observer context,
not all-thread inspection, initial-host attestation, mount content immutability,
atomic/continuous proof, kernel file-opener credentials, durable issuer/refusal,
real host enrollment, production preparation or authenticated original start.
No public helper/IPC/live route, receipt schema or cache code changes.

## Tests and gates

Capture portable baseline and missing API first. Cover exact construction/later
order, every typed failing position, prior refusal without IO, seeded stage and
pin drift, negative Send/Sync, healthy owned NNP fixture/drop, original-process
derivation even after the helper dies, pre-refused credentials, post-open fixture
mutation precedence, revocation, mapping mismatch, mount/component substitution
and restoration, independent concurrency and refusal after wrapper drop.
Private injected boundary/pin cases are modeled, not real foreign namespace races.

Run scoped supervisor Linux/Windows suites, all-target check/all-feature strict
lint, root contracts/evidence/governance, format, docs/links/book, registry and
expected-red ship. Retain raw logs/negative results and frozen qualification bytes.
No infrastructure/product workload, qualification or full-workspace milestone is
authorized. Rollback is the earlier binary without this optional composition.
