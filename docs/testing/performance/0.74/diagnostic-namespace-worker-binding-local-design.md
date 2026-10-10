# Fixed account binding to namespace checked credentials

## Baseline and local scope

Baseline `8ceb10e08f63151974c73fd66b9812e91fb5130d`. Add an opt-in Linux
binding of the original `WorkerAccountRead` and `NamespaceCredentialRead`.
Keep the numeric-only credential API, old binder and existing namespace-checked
opening/read sequence unchanged. Record baseline, missing API, tests,
implementation and repeated results separately. No process/PID, numeric policy,
account snapshot, pathname, descriptor or namespace selector is accepted.

## Original mapping and refusal

Compare the original reader's retained numeric assertions to the original fixed
account snapshot, reusing the exact existing UID/primary GID and membership
union rule without exporting or allocating a replacement policy. A mismatch or
prior refusal stops before any account helper or process observation. Otherwise
require account, namespace-checked credentials, account, retaining each existing
component's checks. The middle stage remains namespace, credentials, namespace;
status is never reopened. Preserve the first typed account or reader error.

Any error permanently refuses the borrowed account guard and the borrowed
namespace-checked reader, including both its privately owned namespace and
credential guards. Constructor failure and dropping the binding cannot restore
these original inputs. A successful drop does not refuse a healthy input. Do
not expose component extraction, reset or arbitrary read callbacks publicly.
Existing manager pending-helper cleanup remains reserved; do not replace it,
kill it implicitly or retry a failed observation.

The account snapshot remains an NSS observation, not a trusted provider/config
or helper namespace attestation. Joining that snapshot to numbers interpreted
inside the reader's sequential namespace bracket does not establish trusted
host/initial namespace, every thread, atomic/continuous proof, authenticated
original start or launch authority. No helper/IPC mode or live route is added.

## Tests and gates

Test exact outer order, prior refusal/mapping mismatch without callbacks, first
typed failure at each outer stage, and seeded valid snapshot drift. Real owned
NNP cat helpers exercise original readers with synthetic account projections:
repeated success, success drop, first/second account drift, original exit,
mapping mismatch during public construction, pre-refused inputs, pending-helper
reservation and independent concurrent bindings. Add tests inside the reader
to demonstrate binder refusal reaches both privately owned component guards.
Positive fixed-account mappings remain synthetic; retain the real NSS operator's
explicit refusal rather than manufacturing a trusted host fixture.

Run scoped Linux/Windows supervisor tests, affected package check/clippy,
formatting, root contracts/evidence/governance, documentation/link/book gates
and expected-red require-ship. Retain exact raw captures and their hashes,
including negatives. Frozen qualification manifest and old evidence packets
remain unchanged. No host mutation, expensive workload, qualification, full
workspace milestone verification or numerical performance claim is enabled.
