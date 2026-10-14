# Diagnostic prerequisites before server execution

Server execution is not admitted yet. This checklist follows the approved
[P0 through P5 sequence](diagnostic-server-readiness-intermediate-plan-20261012.md)
and [collector amendment](diagnostic-collector-design.md). Local primitives and
kernel facts are distinct from a complete installed production path.

## Implemented local prerequisites

P0 has retained original-process SIGKILL/lost-ack proof. P1 has original attempt,
Reserved and first Starting retention with signed history inspection and no
recovered acknowledgement. The collector writer requires actual EOF on both
pipes, exact bounded retained bytes/hashes, sync and explicit descriptor close.

P2 implementation `f2dbbe64c0fbf0a0eeca3bf6e8e2177188bc7f67` obtains kernel
per-message credentials and pidfd, retains the live original sender credential
projection, and samples boot/namespace-bound CLOCK_BOOTTIME/CLOCK_REALTIME.
Independent supplied-key signature checks no longer require caller peer/time
arguments on this new path. This does not load or enroll production trust.

## Required operator input for production trust

The reviewed controller and distinct builder public keys already exist in the
[public builder policy](diagnostic-builder-public-policy.json). Its raw SHA-256
is `dfe81cf770088029e03f04faa4da429ef163583c0ba1b7002b8ec5606e766e22`, matching
the [October 9 enrollment](local-runs/diagnostic-builder-enrollment-20261009/manifest.json)
and separately verified hosted-build evidence. The builder seed was retained in
a protected GitHub environment secret, not a local key file. SSH access keys are
separate from these signing authorities; regenerating them is not a prerequisite.

The [reviewed-trust entry points](diagnostic-reviewed-trust-design.md) reuse these
fixed public pins for builder-policy loading and opt-in kernel controller-key
selection. They do not establish current revocation status or installed trust.

The operator-authorized separate worker issuer key has now been generated locally;
see the [public selection](diagnostic-worker-issuer-public.json) and
[custody/use instructions](diagnostic-worker-issuer-key.md). Its fixed non-policy
selfcheck verifies with OpenSSL and Rust strict Ed25519. The private key remains
outside Git in local WSL storage, not on the performance host or in a new CI secret.

Still select the signed production worker policy with its exact canonical body
SHA-256 and current nonzero authority epoch. Supply an
independently reviewed public-config file or approved public values; no private
key or token belongs in chat/evidence.
An envelope cannot choose its own verifier, digest or epoch. Fixture seeds, keys,
namespaces and modeled host facts cannot fill these slots.

The historical worker-policy packet uses synthetic issuer keys. Creating the
separate real issuer now does not retroactively enroll that packet or a real host.
Production trust loading/enrollment cannot pass until the
remaining operator input exists and is joined to trusted host/root ownership.
The fixed controller/build/worker verification primitives are not themselves
proof of independent production selection. Choosing how keys are generated,
stored and controlled is a deployment decision, not an implicit test default.

## Code and local gates still open

The [fixed trust input](diagnostic-worker-trust-input-design.md) now retains the
selected issuer and explicitly configured body pin/epoch from a bounded original
root-owned file, with an owned config/journal/policy join. Its positive filesystem
and signed-policy cases are private nonroot models with synthetic issuers. No
production configuration or worker policy is created, and current host context
still needs independent kernel admission. This does not close P2.

The separate [worker/kernel join](diagnostic-worker-kernel-binding-design.md)
now compares verified policy with actual retained local machine/boot/user/mount
context and the original kernel packet, without a caller host argument. It
retains config/journal/policy, Probe and packet ownership through cross-refusal.
Real production policy/epoch, source-opener/physical-host enrollment and
controller provenance/replay/fence remain open; no execution route follows.

The [controller principal input](diagnostic-controller-trust-input-design.md)
adds the fixed root-owned configuration reader and an original signed-request
guard borrowing the same worker/kernel chain. It verifies the selected signer's
statement and actual peer/freshness, not independent GitHub/build provenance or
nonce consumption. Real allowlists/configuration, worker policy/epoch and source
enrollment remain unselected; durable replay/fence and P3 are still required.

The [controller and builder receipt binding](diagnostic-controller-build-input-design.md)
retains the independently pinned builder policy and signed statement against the
original request identity, with whole-chain refusal. The receipt has no controller
run/actor/workflow identity: independent controller provenance is still required.
Installed executable contents, durable nonce consumption and common fence are
not supplied by this read-only guard.
The [provenance decision](diagnostic-controller-provenance-decision.md) recommends
an additional GitHub OIDC source. The existing campaign signing workflow cannot
produce a diagnostic packet; new trust/configuration and signing-job policy need
operator approval before implementation. P2 remains open.

1. Connect independently selected root-owned trust inputs to kernel admission
   and original authority/fence ownership, including receiver time/PID namespace
   identity, verified controller provenance, replay, epoch revocation and trusted
   host enrollment.
2. Add the separately signed collector start projection and private root output
   constructor. Preserve legacy signed projections; bind original executable,
   environment, pipes, collector, process and manager invocation/cgroup.
3. Implement autonomous watchdog and exact original-cgroup revocation/emptiness.
   Missing EOF, uncertain stop, collector loss and leaked writers retain failure;
   a stopped leader or replacement unit name grants no release.
4. Join authenticated successful output/outcome and writer cessation to owned
   completion. Implement cells two through four without reopening/rebinding the
   immutable first intent, then SIGKILL every new publication boundary and lost
   acknowledgement. Unknown attempts cannot retry or become successful.
5. Pass the complete integrated local lifecycle/fault gate, full milestone
   verification and signed installation/preflight preparation for exact tooling.

The kernel-input package does not close these items. No green local receipt may
stand in for an enrolled start, whole-writer revocation or later-cell protocol.

## Host gates after local completion

P4 needs actual signed bundle/receipt, ownership, UID/GID, current kernel options,
systemd/cgroup and a quiescent host check. Do not chown live campaign layouts or
stop another campaign. Any approved maintenance stop targets only the exact
stable supervisor, with restoration and health verification.

P5 then needs a separately reviewed bounded nonproduct lifecycle/fault rehearsal
and controller overhead observation on that exact host. Reboot needs separate
maintenance approval. Numerical pilot, product candidates, six-hour/24-hour
qualification and release admission remain separate gates. Frozen numerical
inputs and the inherited plan-digest mismatch are not changed here.
