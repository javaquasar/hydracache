# Signed local mapping bound to original namespace checked credentials

## Scope and baseline

Baseline `4fa462b2cd339f6e4bab4e7371bcdaa8125dea98`. Add opt-in bindings
borrowing the original context/account reader and original NamespaceCredentialRead.
Fixed and owned-fixture bindings have distinct types. Neither accepts raw policy,
numeric assertions, snapshots, PIDs, paths, replacement process or namespace
handles. Reuse old parsers, credential projection and namespace/status opening
unchanged. Existing standalone readers and NSS binders remain separate.

## Mapping and observation order

Compare the credential reader's retained UID, primary GID and exact supplementary
list with the original signed policy. Do not form a primary-group union, infer
membership, allocate/export new credentials or use observations as trust pins.
Private boolean adapters require the original credential projection and no
refusal. Prior refusal wins over mismatch; both stop before observation. Otherwise
require context/account, namespace-checked credentials, context/account. Each
account stage preserves context/files/context; each credential stage preserves
namespace/credentials/namespace and original process generation checks.

The first typed account/context or credential/namespace/process error stops the
sequence. Any failure, including public construction, mismatch or prior refusal,
latches the borrowed context/account guard and original signed policy plus both
privately owned namespace/credential guards. Restoration or binding drop cannot
reset inputs. Successful drop preserves healthy inputs. No callbacks or guards
are publicly exported. Binding inherits original context's negative Send/Sync.

This binding accepts an already namespace-checked credential reader, whose status
opening was bracketed by its own original namespace reader. It does not attest
that status was opened under signed mount/host policy, the kernel file-opener
credentials, the worker mount namespace or all worker threads. The new bracket
compares retained numeric assertions interpreted through that sequential shared
user namespace with signed mapping and observed reader context. It is not atomic
or continuous enforcement, physical/initial-host attestation or production
enrollment. Existing ProcessRead remains borrowed/read-only, not killed or reset.

## Tests and closed boundaries

Record portable baseline and missing API before implementation. Cover exact order,
prior refusal/mismatch without callbacks, every first failing outer stage, exact
UID/GID/supplementary mapping (including primary presence versus absence), seeded
mapping mutations, non-transferability, healthy drop, constructor drift/refusal,
revocation, changed account files/restoration, original owned NNP process exit
and independent concurrent bindings. Use private test seams only. Real unprivileged
NNP cat helpers and original-process test pins combine with actual local context
and synthetic account files/issuer keys. Fixed production account positives,
real issuer enrollment and foreign namespace operations are excluded.

Run scoped Linux/Windows supervisor suites, check/strict lint, formatting, root
contracts/evidence/governance, docs/links/book and expected-red require-ship.
Retain exact logs with hashes and verify historical packets. Frozen qualification
inputs and manifest, product/cache/native code and production routes remain exact.
No host infrastructure, install, workloads, performance claim, durable epoch/refusal,
production preparation, authenticated original start or qualification follows.
Rollback uses the earlier binary without this new opt-in runtime-only API.

## Implemented binding and local results

Implementation `72322696b422f3621763fb2af1c9535c0f1d9a7c` adds fixed and owned-fixture runtime bindings.
Two private boolean adapters compare original credential assertions to the signed
mapping, requiring an original projection and healthy components. No projection,
policy or original object is exported. Old opening/read/parser behavior stays
unchanged. Refusal reaches original policy/account/context and both namespace/
credential guards; success drop leaves them healthy.

The [packet](local-runs/diagnostic-policy-credentials-20261011/manifest.json)
retains a 43-pass portable baseline, ordinary missing-API E0432 (WSL exit 1),
14-test first green and reviewed final suite. Linux passes 324 (250 library,
29 artifacts, 9 context-files, 2 manager, 3 account, 5 context, 13 files, 13 policy),
with one existing system-bus ignored. Windows passes 43; root contracts/evidence/
governance pass 132 and new root guard passes Linux. All fifteen binding cases
plus the component/projection case pass three further runs. Seed `0x7562026`
varies 256 modeled mapping mutations per run.

Order/first error, no IO after mismatch/refusal, exact primary-list distinction,
non-transferability, healthy drop, constructor refusal, revocation, restored
account files and owned NNP process exit/concurrency pass. Private component tests
refuse missing projection or either refused component. The fixed branch has
negative coverage only; no public fixture conversion or positive production
inspection is introduced. Initial Linux test/check logs retain one redundant-mut
warning; after removing the annotation final tests/check/strict lint pass.

Format, scoped check/lint on both platforms, docs/links/book and final registry
pass; require-ship remains expected native exit 1. Frozen qualification inputs
and existing packets remain exact. This is working-tree local safety, not
full-workspace qualification, real issuer enrollment or performance data. Next:
open status inside the signed-context observation bracket without replacing
original process identity. All-thread/worker mount authority, durable issuer
epoch/refusal, production preparation and authenticated lifecycle remain separate.
