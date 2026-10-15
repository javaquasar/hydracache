# Structured signer bootstrap context

The stronger opt-in verifier checks the structure and internal bindings of the
previously opaque bootstrap-context slot. It rejects a correctly signed unsafe
declared profile without changing the old build model or original GNU join.
These are requirements asserted by a selected statement, not observed job
protection or independently admitted launcher/runtime custody.

## Context and receipt binding

`diagnostic-signer-bootstrap-context-074-v1` is canonical sorted-key JSON plus one
LF, at most 65536 bytes, with strict unknown/duplicate/missing/type refusal. Its
repository, nonzero lowercase source commit/tree, binary and runtime-manifest
digests must equal the already expected `SignerBuildStatement`. The context's
exact SHA256/length must match that statement's bootstrap-context slot.

The separate launcher assertion requires a nonzero SHA256, length 1..128 MiB and
a hash distinct from the signer. No launcher file is opened, selected, executed
or certified by that declaration. An explicit nonzero lowercase 40-hex workflow
SHA is required alongside the existing fixed controller workflow ref,
`performance-diagnostic-controller-074` environment and default environment
subject. Only `workflow_dispatch` and `github-hosted` are supported by this
declared profile. Workflow/environment strings are shared with the old OIDC
verifier; its accepted policy, signature/claim checks and wire bytes do not change.
Its existing configurable subject behavior is not narrowed by this new reader.

Six declarations must be false: build signing authority, build OIDC authority,
signer compilation, dependency installation, checkout and unreviewed inherited
environment. A signed false field cannot prove that a job obeys it. Actual steps,
runner state, root/filesystem, environment, later loads and TLS inputs still need
their own admission and enforcement; this schema is not a sandbox specification.

`verify_supplied_with_bootstrap_context` first calls the existing receipt/content
verifier, then checks the retained expected context. It returns a private-field
diagnostic model with receipt/context hashes, no Clone/Deserialize and no
conversion into the old checked model or signing/start authority. Correctly
re-signing an unsafe context makes the old opaque model pass but the stronger
one refuse. Changing well-formed launcher/workflow assertions requires a new
full expected statement; explicitly reselecting it can pass syntax. That test
demonstrates why this API cannot independently enroll pins.

## Original GNU file composition

Linux `retain_asserted_bootstrap_runtime` checks the exact expected receipt and
structured context before file acquisition, then delegates unchanged original
GNU acquisition, catalog regeneration and all-artifact verification. The returned
`AssertedBootstrapRuntimeRead` owns the old reader without extracting or converting
it. Original rechecks and sticky all-input refusal remain delegated to that same
owner. Legacy runtime construction still accepts its old opaque context.

Both stronger results remain assertions/observations. No producer, CLI, server,
token request, signing, replay, fence or original-start consumer accepts them.
Construction publishes nothing, adds no process or durable write boundary and
changes neither a workflow nor actual trust configuration. Actual launcher/root/
job custody and real selected-issuer receipt still precede bootstrap admission;
worker original-start, all writers and completion/cells 2–4 remain open.

## Local falsification

Seven portable cases cover structured positives with deliberately non-ELF and
opaque-runtime bytes, every context field, unsafe correctly re-signed profiles,
alternate assertions, wrong signature/key/content, canonical/schema/type/size
faults, 64 signed schema mutations from seed `0x0740c078` and four independent
models. Portable positives must not be reported as runtime validation. The Linux
composition corpus adds a ninth case: old opaque success, stronger opaque
refusal before IO, valid synthetic composition, public rejection of the nonroot
fixture and sticky original-file failure. Three portable and two Linux
compile-fail cases prohibit cloning/deserializing, weak-result extraction and
worker start. The [preregistration](diagnostic-signer-bootstrap-context-plan-20261015.md)
and [contract](diagnostic-signer-bootstrap-context-contract.toml) retain the scope.
