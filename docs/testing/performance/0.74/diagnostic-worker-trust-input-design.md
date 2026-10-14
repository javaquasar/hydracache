# Retain the fixed worker trust source with original authority

Baseline is `a4bb2cb0d34814492ace996d1bda07452e97480d`. The separate
[worker issuer](diagnostic-worker-issuer-public.json) is selected, but production
worker policy/body pin/current epoch and installed trust remain unselected.
This P2 slice prepares a read-only input path; it creates no live configuration,
signs no policy and grants no execution.

## Fixed trusted configuration

Add a Linux-only no-argument constructor for
`/etc/hydracache-perf/worker-trust-074.json`. Require preliminary effective UID/GID
zero, root-owned searchable nonwritable-peer ancestry, a root-owned single-link
non-executable regular file and bounded reads. Retain root, both ancestor
directories and the original file. Reject links, special files, unsafe modes,
replacement, metadata/content drift, short/growing reads and every failure
permanently. Never create, chmod/chown, truncate, repair or reload this source.
Scalar root ownership is not initial-host namespace enrollment.

The separate canonical LF JSON schema `diagnostic-worker-trust-input-074-v1`
has exactly schema_version, repository_id, issuer_key_hex,
worker_policy_body_sha256 and policy_epoch. Budget is 2048 bytes. Require fixed
repository, the separately selected real worker issuer, valid lowercase SHA-256
and nonzero epoch. No envelope, public selfcheck, environment or caller key/hash/
epoch may select these inputs. Unknown/duplicate/future/noncanonical/missing/null
fields refuse. Rotation is an explicit reviewed change, not an automatic fallback.

This task does not populate that file with fake values or default epoch 1. Its
eventual contents must come from independently approved production policy and
authority. Do not reinterpret the issuer selection, worker-policy v1, installed
receipt v1 or existing authority journal as this configuration.

## Original lifetime and authority join

Verify under config/journal/policy/journal/config brackets. Require the same
active authority revision/epoch on both inspections under the retained trust and
asserted host context. Return a non-cloneable original-thread policy input guard
that owns the verified policy and original envelope/context, and retains mutable
borrows of the original config owner and fixed authority journal. Export no
worker policy, trust extractor, FD, serialization, fixture conversion or start.
Revalidation accepts no replacement bytes, context, key, pin or epoch.

Failure permanently refuses the original config and authority runtime owners;
existing policy refusal is also retained. This is an in-memory cross-refusal,
not a durable revocation append or recovered admission. Fresh config loading
alone is not monotonic authority: the original journal join is still required,
and whole-host rollback remains outside the selected model.

Host context is still an asserted input at this layer. Independently admitted
kernel host/namespace and controller provenance/replay/fence joins remain P2
work. Root-file consistency, a signature or a local fixture cannot replace them.

## Preregistered local checks

Retain existing worker-policy, reviewed-public-trust and fixed-authority baseline,
then a missing-API red test. Exercise strict documents, missing pin/epoch, wrong
issuer, metadata/mode/link/type/size boundaries, root/ancestor/file replacement,
same-length byte changes, original refusal and no refresh. Test verified policy/
journal joins, refused/drifted/wrong authority and wrong policy signature/host/
epoch under private synthetic keys and owned nonroot modeled roots; never sign a
fake policy with the actual issuer. Include deterministic short-read/race checks,
seeded document mutations, independent concurrency and compile-fail lifetime/
extractor/thread constraints. Modeled positives do not establish installed root
or host enrollment. Actual public constructor must refuse the nonroot local caller.

Run three focused repeats, portable policy regression, current/MSRV checks,
strict current lint, scoped format, documentation and non-promotable contracts.
Keep qualification digest, workload, thresholds, private keys, existing generic
APIs, provisioning and workflows unchanged. Original start/root collector,
watchdog/writer cessation, completion and cells two through four remain separate.
