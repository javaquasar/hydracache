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

Select the diagnostic controller public key, distinct builder public key and
worker-policy issuer public key, with exact builder/worker policy SHA-256 and
current nonzero authority epoch. Supply an independently reviewed public-config
file or approved public values; no private key or token belongs in chat/evidence.
An envelope cannot choose its own verifier, digest or epoch. Fixture seeds, keys,
namespaces and modeled host facts cannot fill these slots.

Current independently selected values are not present in the reviewed source.
Production trust loading/enrollment cannot pass until this operator input exists.
The fixed controller/build/worker verification primitives are not themselves
proof of independent production selection. Choosing how keys are generated,
stored and controlled is a deployment decision, not an implicit test default.

## Code and local gates still open

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
