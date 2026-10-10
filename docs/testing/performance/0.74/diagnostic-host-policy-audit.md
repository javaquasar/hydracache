# Worker identity authority before production preparation

## Decision required

The existing provisioning and host observation receipts do not enroll a trusted
diagnostic worker identity. The namespace-checked account binder proves agreement
between retained observations, not their authority. Production preparation and
authenticated start must remain closed until the identity authority and admitted
host context are explicitly selected. This is an architecture proposal, not an
implemented policy or permission to mutate the host.

Audit baseline is `8e16117b94ab6a3b60a182ef5f33e2c5aaeb6893`. Retain the seven
existing portable host-receipt tests before adding regression coverage. Test that
neither existing receipt format accepts an extra worker-authority field, even
when an extended host observation has canonical bytes and a matching digest.
These tests protect existing rejection behavior; no missing API or product
implementation is proposed in this slice.

## What the existing artifacts establish

The [installer](../../../../scripts/perf/long-run-supervisor-074/provision-host-074.sh)
verifies a signed bundle against the installed verification key, runs the fixed
sysusers configuration and records installed artifact hashes, source commit,
machine/boot hashes and a `supervisor_uid` observed through getent. The sysusers
file requests allocation with `-`, not a preselected numeric worker mapping.
Its digest identifies the requested configuration, not the resulting UID, primary
GID and complete membership policy. The receipt has no NSS policy, account-file
identity or helper user/mount namespace fields. The actual supervisor service
is configured as root, distinct from the account named `hydracache-perf`.

The [installed source parser](../../../../tools/long-run-supervisor-074/src/host_receipt.rs)
accepts the exact 24-field provisioning-v1 inventory and projects source commit
and fixture binary digest. It is not a worker enrollment verifier. The separate
host observation records machine, boot, kernel, backing mount, CPU/tunables and
supervisor binary. Its backing-mount identity deliberately ignores the mount ID
local to a namespace. Matching that identity is therefore not evidence that the
NSS helper runs in a particular user or mount namespace. Its strict decoder
rejects unknown fields rather than silently admitting a new authority schema.

The [fixed account observer](diagnostic-worker-account-local-design.md) uses the
helper's NSS context. GNU libc describes separate services and per-database
lookup rules in its [NSS manual](https://raw.githubusercontent.com/bminor/glibc/glibc-2.39/manual/nss.texi).
Thus our inference is that pinning only nsswitch.conf would not establish trust
in provider code, provider data or the helper's filesystem context. Root
ownership of one configuration file cannot substitute for a reviewed authority
model. These are repository/source observations, not a fresh server inspection.

## Alternatives and recommendation

One option is to make the existing libc NSS result authoritative. That requires
an admitted provider set and its configuration, code/data dependencies, helper
context and failure/drift rules. Supporting remote or extensible providers adds
policy and lifecycle obligations unrelated to the fixed diagnostic account.
The current receipts and guards do not implement these obligations.

Recommend instead an opt-in local enrollment for the fixed diagnostic worker.
An authorized operator would approve the exact nonroot UID, primary GID and
bounded supplementary-group set, with a separately pinned canonical policy
digest and an admitted host/boot context. A new verifier would inspect only the
fixed local account files through retained bounded no-follow reads, compare the
fixed account's forward/reverse mapping and membership to that approved policy,
and refuse ambiguity, unsafe ownership, replacement, drift or refusal. It would
not consult general NSS to establish authority or change system-wide NSS rules.
The existing NSS observer and binders remain separate consistency tools.

This recommendation needs approval because it selects local files rather than
general NSS as the worker account source. It does not assert that files alone
authenticate the host. The enrollment issuer, key/policy trust pin, admitted
user/mount context, reboot revocation and helper context enforcement must be
specified before positive production use. Comparing with the current reader or
PID 1 alone must not be called proof of the initial host namespace.

## Implementation sequence after approval

First define the separate enrollment schema and external trust pin without
extending either receipt v1 implicitly. Keep enrollment verification local and
opt-in; no install or start route accompanies the parser. Cover malformed,
duplicate/future fields, wrong signatures/pins, wrong host/boot, zero/sentinel
IDs, group bounds and canonicalization, replay/revocation and first-error refusal.
Then add fixed-file inspection and the namespace/context join with substitution,
fragmented reads, membership drift and concurrent independent guards. Positive
temporary fixtures establish parser/reader behavior, not real host admission.

Only after authority verification, all-thread/start identity requirements and
fenced production output preparation are complete should a separately reviewed
non-product lifecycle rehearsal exercise the installed implementation. Watchdog
and uncertain-intent recovery remain gates before qualification. Approval of the
local policy design alone does not authorize installation, workloads or changing
frozen qualification identities and thresholds.
