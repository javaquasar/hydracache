# Diagnostic request producer and public key inspection preparation

Baseline is `fc696e72aa33e5a5d41227f9314455293705a9fb`. This local step prepares
the distinct diagnostic producer, not the campaign protocol or a protected-job
token. The [approved OIDC decision](diagnostic-controller-provenance-decision.md)
still requires separate actual trust/job/host enrollment before server admission.

## Preregistered producer boundary

Add a portable private-field PreparedDiagnosticRequest from canonical LF
DiagnosticRequest bytes plus the fixed reviewed builder policy and matching signed
receipt. Validate the existing diagnostic domain/source/preset/window/repository
before touching any signing key. Derive audience from the exact canonical request
digest using the same domain-separated format as the OIDC reader; do not change
any signed schema. Caller run/actor fields remain statements until independently
verified OIDC origin and fixed host admission are joined.

Signing consumes the preparation and requires the already reviewed controller
public key; no caller verifier, pin or key-purpose override. Sample job UTC before
and after signing and refuse future/expired requests without clock allowance.
Job time is not receiver kernel time, epoch authority, durable nonce protection
or host enrollment. Verify the resulting Ed25519 signature under the selected
key before returning canonical bounded signed bytes. No token is minted, parsed
or logged by this producer. No actual signing secret is read in local tests.

Add a separate Linux-only diagnostic_controller CLI with audience and sign
commands. Reuse the protected builder's original no-follow bounded file helpers
internally; secret parent must be caller-owned 0700, secret leaf 0600, single-link
and at most 65 lowercase hex/LF bytes. Clear encoded/decoded key buffers, create
output exclusively, sync output/directory, never overwrite or retry. Public
production entry always selects the reviewed controller pin. Synthetic positives
use private test-only selection, not a public fake-key option. Do not add any
command to the installed supervisor, network listener or server dispatch.

## Public key inspection preparation

Prepare a separate bounded read-only inspection helper for discovery/JWKS inputs.
Accept no arbitrary endpoint, redirect, proxy, bearer token, secret or incoming
token-selected kid. Exact fixed issuer/endpoint and TLS hostname/certificate
verification are required for any future public fetch. An offline JSON inspection
is explicitly not authenticated acquisition. Record bounded key metadata and
raw source digests, never choose actual policy/workflow/expiry or install trust.
Real fetching/selection is not required or claimed by synthetic local tests.

## Baseline regression and verification

The Python builder baseline has six passes and one frozen-input failure: its
test uses current tooling lock bytes. Model the exact frozen observer inputs in
an owned temporary fixture and assert every existing digest. The production
coordinator, old signed receipt, frozen lock pins and workflows stay unchanged.

Capture missing producer API/CLI before implementation. Cover canonical/duplicate/
future/version/repository/source/identity/build mismatches, selected-key purpose,
exact signing bytes and audience, job freshness, secret metadata/links/bounds,
exclusive outputs and non-Linux CLI refusal. Add deterministic seeded mutations
and independent preparation/signing tests using synthetic controller keys only.
Test inspection JSON budgets/duplicates/issuer/endpoint/key confusion and explicit
non-enrollment. Run three focused repeats, affected regression, current/MSRV
checks, strict current lint, format, contracts/registry and documentation gates.
Retain every functional failure. No remote job, actual token, server operation,
new SIGKILL publication proof or performance measurement belongs to this slice.
