# Structured signer bootstrap context consistency

Baseline `8aa8e208128c0db93084e6fea1fe53a3e84c9258`. Continue local implementation
without actual keys/tokens, server/GitHub operations, workload or qualification.
The runtime join now regenerates the GNU catalog but still accepts opaque
bootstrap context. Test the isolated hypothesis that a stronger opt-in path can
reject inconsistent or unsafe declared job profiles even when the old receipt
signature and opaque hashes match. This is statement consistency, not actual job
protection, launcher acquisition or independent runtime admission.

Add canonical LF/deny-unknown structured context v1, at most 65536 bytes. Bind
repository, exact signer source/tree, binary and runtime-manifest digests to the
already asserted expected build statement. Require an explicit nonzero launcher
digest/size up to the existing 128 MiB binary limit, distinct from signer; it
remains an assertion, not a retained or executed launcher. Require the existing
fixed controller workflow ref, explicit nonzero lowercase 40-hex workflow SHA,
environment/default subject, workflow_dispatch and github-hosted profile. Share
the existing fixed workflow/environment strings with the old OIDC implementation
without changing its verification behavior. No actual workflow SHA is selected.

Require false declarations for build signing/OIDC authority, signer compilation,
dependency installation, checkout and unreviewed inherited environment. These
fields describe the required job, not observed configuration or a sandbox.
Missing/unknown/duplicate/future/noncanonical/type-confused and opaque contexts
refuse. Do not alter the old model or original runtime join's permissive context
semantics; add a stronger separate result with no conversion back to a weaker
guard or into signing/start authority. The portable path verifies the unchanged
receipt and all supplied content before structured context checks. The Linux
path validates receipt/context before acquisition, then reuses original GNU
runtime acquisition and exposes only diagnostic hashes after original rechecks.

Capture old portable build/GNU baselines and Linux runtime-join baseline before
mutation, then a missing ordinary portable API failure. Use synthetic keys and
images only. Cover every context field, correctly re-signed unsafe declarations,
different launcher/workflow assertions, every source/binary/runtime binding,
canonical/unknown/duplicate/type/size limits, legacy opaque acceptance, stronger
opaque refusal, and no deserialization/clone/authority conversion. Seed
`0x0740c078` covers 64 signed byte mutations and four independent models. Extend
the existing private Linux fixture tests to distinguish old and stronger joins,
preserve sticky original-file refusal and publicly reject nonroot fixture input.

Run repeats, old build/GNU/issuer/runtime/root/OIDC regressions, current all-target
check/strict lint, MSRV 1.88, root contracts, scoped format, documentation and
local non-promotable checks. Preserve red results. Commit preregistration,
implementation and evidence separately; update COMPAT/GATES/article and keep
frozen lock/qualification/product/workflow inputs unchanged. Real launcher,
runtime/root/job custody, signer receipt, worker original-start, all writers and
completion/cells 2–4 remain open; no positive synthetic profile may select them.
