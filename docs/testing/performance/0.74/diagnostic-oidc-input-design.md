# Fixed OIDC trust and original controller origin observation

Baseline is `35210757cc505e159345c58608677aa537c4bdd1`. The operator approved
local implementation of the [OIDC decision](diagnostic-controller-provenance-decision.md),
not actual key/workflow enrollment, remote jobs or host execution.

## Narrow initial profile

Use already locked ring 0.17.14 RS256 verification and base64 0.22.1 decoding as
Linux-only supervisor dependencies. No new package/version is selected; the
current tooling lockfile gains those direct dependency edges only. The frozen
observer source/lock hashes and qualification inputs stay unchanged. Ring verifies
the original compact JWS signing input with RSA PKCS1 SHA256, not reserialized JSON;
see [ring's component verifier](https://docs.rs/ring/0.17.14/ring/signature/struct.RsaPublicKeyComponents.html).

Require three bounded canonical unpadded base64url segments, exact RS256/JWT
header and one configured kid. Reject unknown header fields, duplicate claims
including escaped duplicate names, nested/floating/null claims and malformed
encoding. Required origin/time fields have strict types. Bounded additional
scalar claims confer no authority. Verify fixed issuer/repository, request-derived
audience, run/attempt/actor equality, exact configured subject/workflow SHA and
fixed dedicated workflow/ref/environment/event/hosted-runner profile. Do not
equate controller workflow SHA with the frozen observer source commit. For this
initial non-reusable workflow profile, reusable job claims refuse. No clock
allowance, algorithm/key URL override or networking exists.

## Fixed source and original ownership

Schema `diagnostic-oidc-trust-input-074-v1` lives only at
`/etc/hydracache-perf/oidc-trust-074.json`, with the existing 2048-byte source
budget. Canonical LF fields: schema_version, repository_id, issuer, workflow_ref,
workflow_sha, environment, subject, snapshot_not_before, snapshot_expires_at,
and key. The key contains exactly kid/kty/alg/use/n/e. Select one RSA modulus of
2048 through 4096 bits and exponent 65537, strict base64url integers without
leading zero. Snapshot lifetime is explicit and positive; no default epoch or
actual public key is chosen. Require current kernel wall time inside the snapshot
and token windows; token lifetime is at most 600 seconds, while original request
and packet freshness retain their exclusive 60-second bounds.

The no-argument root-only source constructor retains original no-follow ancestry,
file metadata and bytes using existing private source primitives. Only the
original ControllerBuildInput may join it to token bytes. The result mutably
borrows both originals, owns bounded token bytes and revalidates without any
replacement input. A private kernel-clock sampler must retain sender/namespace/
boot/hold checks and whole-chain refusal. All construction/revalidation errors
latch OIDC source and the original controller/worker/config/journal/packet chain.
Healthy drop preserves owners. No raw token, claims, keys, policy, request or FD
export, clone, Send, start, nonce consumption, fence or completion is added.

Root ownership is not independently authenticated public-key acquisition or
physical host enrollment. Positive tests use synthetic RSA/controller/worker keys
and private nonroot roots with actual local kernel packets. No real private key,
production configuration, token or job is accessed. The dedicated workflow is
specified here but not dispatched or wired into the frozen campaign workflow.

## Preregistered local verification

Capture controller/build baseline and a missing API compile refusal before code.
Generate one explicitly public synthetic RSA test fixture, never a production key.
Exercise valid RS256 on original bytes, noncanonical JSON that remains a valid
JWS, signature/header/key confusion, recomputed audience mismatch, strict claims,
exclusive time/snapshot bounds, all source/owner drift and permanent cross-refusal.
Use seed `0x07552074`, 32 signature mutations and four independent bindings.
Add compile-fail export/start/replacement/thread/borrow constraints. Run three
focused repeats, affected regression, current/MSRV checks, strict current lint,
doctests, format, root contracts/registry/evidence and documentation/local gates.
No real GitHub issuer proof, integrated replay admission, SIGKILL publication,
full release milestone, server readiness or numerical measurement is claimed.
