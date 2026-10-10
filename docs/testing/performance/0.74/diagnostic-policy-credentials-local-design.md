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
