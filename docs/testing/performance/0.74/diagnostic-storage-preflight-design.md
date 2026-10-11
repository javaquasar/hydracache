# Diagnostic storage server preflight

This step prepares code for a read-only storage preflight on the dedicated host,
not a worker-start pilot. Baseline is `7119644eeb5c4536a9cbcc06254520cef5545923`.
The fixed publisher and original authority/fence/state/runtime composition remain
required before any diagnostic worker execution.

## Ownership amendment before observations

The historical fixed authority contract required every ancestor's root GID.
Existing provisioning deliberately sets the performance parent root UID with
hydracache-perf GID and 0750. This new preregistered contract amends only that
parent's GID requirement: accept its provisioned group when UID stays zero,
group/other write and special bits are absent, and retain/recheck its exact
original GID with device/inode/mode. `/`, `/var`, `/var/lib` and both private leaves
remain root:root. The worker-owned campaign directory is not authority storage;
its owner/mode and the common parent's existing provisioning are unchanged.
No record schema, pin, epoch, rollback assumption or public origin selector changes.

Provisioning adds only empty root:root 0700 worker-authority-074 and
diagnostic-intents-074 leaves to the existing signed tmpfiles input. It neither
initializes journals nor enrolls a key/policy. Do not execute the installer here.
An updated bundle must carry a fresh source receipt/signature before later
installation approval; existing installed receipts cannot cover changed tooling.

## Preflight command and scope

The Linux command `diagnostic-storage-preflight` has no arguments/selectors. It
opens separate original fixed authority-storage and intent-storage guards,
rechecks authority/intent/authority/intent and prints a bounded versioned JSON
result. Opening is root UID/GID gated. No file/directory is created, no flock is
acquired, and neither guard initializes or parses a journal. Unknown arguments,
missing paths, nonroot execution, unsafe permissions or object substitution
refuse with a nonzero exit. Success is explicitly storage ancestry only: journal
verification, authority binding, enrollment, worker launch, qualification and
ship are all false. It does not depend on GitHub, NSS, a helper or a shell.

Model both complete and missing leaves, all ancestry substitutions, benign
growth, original GID drift and sticky refusal using private nonroot temporary
trees. Validate the existing authority engine and intent guard regressions,
test public CLI nonroot/argument refusals, strict lint, MSRV and documentation.
Keep all frozen qualification inputs unchanged and retain the genuine digest
refusal. Actual root-path positives belong to later controlled host checks.

## Requirements for the server test

After separate installation authorization: verify the fresh signed bundle and
source receipt, confirm no active campaign or lifecycle fixture before maintenance,
use only the exact stable supervisor if a stop is required, and restore/verify it.
Run the fixed preflight as root through a separately approved execution path;
do not broaden the runner sudoers. Retain stdout, stderr and exit code with the
installed source identity. A successful result permits no worker start.
Real operator worker/controller issuer pins, initial-host namespace admission,
fixed publisher, production output preparation, watchdog and owned cleanup
remain before a worker-start server rehearsal. No test issuer may be promoted.
