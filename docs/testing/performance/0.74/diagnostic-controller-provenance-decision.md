# Controller provenance decision before replay admission

Status: proposed, awaiting operator approval. Audit source is
`d27c00970b329305ed840a1013950cff44db8ca9`. P2 needs a source independent of
the controller signature to authenticate its GitHub origin. The recommended
extension is GitHub OIDC alongside, not instead of, the current controller
signature. No new trust root, crypto dependency, configuration schema, workflow
permission or server route is enrolled by this proposal.

## Existing protocol boundaries

`diagnostic_ipc.rs` signs a distinct diagnostic request: reserve/heartbeat/status/
cancel, hash-form request ID, nonce, lease identity, fixed observer source/preset
and an authorization window of at most 60 seconds. Its controller tuple contains
repository ID, run ID, attempt and actor. The original controller/build input
retains those bytes and authenticates the reviewed builder's statement against
the request identity. BuildStatement has no controller run, actor or workflow
fields; matching its signature cannot independently verify that tuple.

`performance-long-run-qualification-074.yml` instead constructs campaign
start/attach/seal/abort requests, with a UUID request ID, campaign manifest and
300-second authorization. It uses the campaign build-request CLI, not the
diagnostic signing domain. The nonproduct start-bundle workflow assembles a
different six-file bundle. Neither is a producer for the new diagnostic guard.
The separate builder workflow signs the fixed build receipt. Its artifact name
includes run/attempt, but that name is not part of the signed BuildStatement.
These formats must not be reinterpreted as each other.

## Recommended additional origin evidence

GitHub documents OIDC run/attempt, numeric actor/repository, workflow ref/SHA,
environment and runner claims, plus a configurable audience. Token retrieval
requires `id-token: write`; this proposal scopes it to one dedicated signing job.
These facts make OIDC a suitable additional
origin source; they do not prove completion, a current active run, compilation or
permission to execute. See the [GitHub OIDC reference](https://docs.github.com/en/actions/reference/security/oidc).

The proposed implementation would require both the existing original controller
signature and a separately verified GitHub token. Derive a domain-separated
audience from the canonical original DiagnosticRequest digest; never accept an
audience supplied as an independent assertion. Require token run/attempt/actor/
repository to match that request and fixed principal policy. Select exact
workflow revision/ref, environment, event and GitHub-hosted signing-job policy
independently. The observer source commit is not the controller tooling/workflow
commit; neither can fill the other's pin. Reusable-workflow claims need an
explicit supported policy rather than treating missing claims as defaults.

Retain original controller/build/kernel/config/journal inputs throughout the
check. Use original kernel freshness sampling with exclusive expiry; OIDC time
cannot supply policy epoch or extend the diagnostic request's 60-second window.
Never allow token-controlled key URLs, key material, algorithms or trust pins.
Require bounded strict parsing, duplicate/type/missing-claim refusal, exact
issuer/audience, approved signature algorithm and independently authenticated
verification keys. A claims-only decoder must have no admission conversion.

## Key acquisition and rotation proposal

Prefer a reviewed, bounded public GitHub verification-key snapshot in a separate
root-owned trust configuration, retained with original metadata and bytes. Its
selection, review interval and expiry must be explicit; an incoming token cannot
select its own key set or epoch. Unknown keys, stale selection, substitution and
unsupported algorithms refuse instead of triggering privileged automatic
downloads. Rotation is a reviewed configuration change that refuses old runtime
guards, not replacement arguments on revalidation. Actual current public keys,
digest, expiry and workflow/environment values remain unselected.

This proposal keeps network fetching and GitHub credentials out of the privileged
host execution path. The snapshot's independent authenticated acquisition and
custody still require implementation and review; a caller-provided JWKS file is
not trusted merely because its JSON parses. Crypto-library choice and MSRV/
dependency review belong to the first implementation preregistration after
approval. No raw OIDC token or bearer credential should enter logs or repository
evidence; retain only reviewed redacted metadata, digest and refusal outcomes.

## Alternative and rejected shortcuts

A direct trusted HTTPS query of GitHub's exact run-attempt endpoint is an
alternative. The [workflow-runs API](https://docs.github.com/en/rest/actions/workflow-runs)
exposes run and actor metadata. It would require authenticated TLS/network
ownership, response freshness, bounded failure behavior and credential custody
on the verifier path. A downloaded unsigned JSON file or caller-supplied API
response cannot inherit those properties. This adds an online dependency at
admission and complicates deterministic offline evidence, so it is not the
recommended first implementation.

Reject copying metadata from environment variables, artifact names, unsigned
host observation or a second receipt signed by the same controller as independent
GitHub proof. Reusing the builder key to sign controller assertions would also
expand its reviewed fixed-build purpose. OIDC origin evidence remains separate
from reviewed builder authority and from monotonic worker/host policy authority.

## Verification after approval

Preregister the new bounded token/trust profile and compatibility changes before
code. Capture baseline and missing API refusal. Use synthetic issuer keys only
for modeled positives. Test every required claim, signature, key/algorithm
confusion, audience/request substitution, duplicate JSON and malformed encoding,
time boundaries, rerun identity, original-owner drift, key rotation refusal and
cross-refusal of every original input. Add property/concurrency tests and
compile-fail constraints prohibiting raw claims export, replacement and start.
Keep synthetic positives distinct from a future actual protected-job receipt.

Only a reviewed provenance producer may feed the owned durable replay/common
fence join. Define consumption before acknowledging or executing, preserve
unknown attempts and never renew expired requests on retry. New durable writes
then require owned SIGKILL and lost-ack proof. OIDC token identity alone does not
consume a nonce. Actual production policy/epoch, root/host enrollment, installed
executable verification, original start, collector/watchdog/writer revocation,
completion and later cells remain prerequisites under the existing checklist.

## Approval scope

Approve local implementation of the OIDC option, its additional independently
selected public-key trust source and the dedicated diagnostic signing-job design.
This permits preparing a narrowly scoped future `id-token: write` job, not running
it or changing existing frozen qualification tooling. Actual key/workflow policy
selection, protected GitHub configuration, installation, job dispatch, server
rehearsal and product qualification retain their separate approvals.
