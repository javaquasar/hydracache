# Retained body signing before token coordination

The local signing API now consumes one original opaque preparation and checks
the audience associated with its retained body before reading any signing key.
It removes a public-file reopening seam, not the independent signer custody,
token verification or receiver authority boundaries. The
[preregistration](diagnostic-retained-producer-plan-20261014.md) fixes the scope
against `e30740fe1bc746bf66d30368be0995538e3c3bf9`.

## One retained preparation

prepare_request/prepare_files already own the validated DiagnosticRequest and
fixed verified builder identity. On Linux, sign_prepared_files consumes that
object, the observed audience and the existing key/output paths. It takes no
replacement request, policy or receipt. Recompute the canonical body SHA, require
its retained digest/build identity and canonical domain-separated audience, and
compare the observed audience exactly. Reject mismatch before secret file IO.
Request freshness, fixed reviewed controller pin, private key metadata/encoding,
explicit buffer clearing, clock brackets and exclusive synced output retain the
existing implementation. Errors consume the Rust owner; they do not consume a
durable nonce or prevent constructing another preparation.

Local tests change or remove all original public files after preparation. The
signature still binds the original body and its earlier audience. A separately
valid replacement body paired with that earlier audience refuses before even
opening an absent key. An audience assertion is not a received OIDC token and
does not prove that any token was requested for it. A trusted future coordinator
must retain this same object through actual retrieval and verification, rather
than calling the old file-signing path again.

The legacy file-signing wrapper delegates after its existing preparation. The
CLI's separate audience/sign commands remain separate reads. No new command,
stdin handshake, wire schema, network helper, receipt or server route is added.
Compile-fail examples prohibit owner reuse/clone and supplying a request path
where the retained API expects the earlier audience. The existing signed domain
and packet shape are unchanged; no migration or old-binary upgrade is required.

## Bootstrap remains an independent custody decision

The current reviewed builder key attests only the frozen observer binary. Its
statement has neither a controller-signer binary role nor a reviewed bootstrap
context. Using it for the signer would change its purpose. The controller key
cannot independently attest the executable that will read that same private key.

Before activating a token-bearing job, select a reviewed executable/runtime
artifact through independently authenticated custody. A locally computed digest,
artifact name, caller-provided checksum or the executable's self-report is only
inventory. The job must not compile, install dependencies or checkout unreviewed
code after exposing signing/token authority. A new signer-build receipt/domain/
key role, or actual out-of-band artifact/pin enrollment, needs a separately
reviewed decision. No such root or purpose is selected by this retained-body API.

The subsequently approved [local signer build model](diagnostic-signer-build-design.md)
implements only the separate receipt/domain/role using synthetic keys. Its
explicitly asserted selection and supplied-byte consistency result cannot
authorize this producer, authenticate original runtime custody or choose a
production root. The retained signing API remains unchanged.

## Verification and remaining integration

Synthetic positives cover original bytes/signature/audience after file and
caller-buffer changes, valid-body substitution, exclusive output, selected-key
refusal and unsafe key metadata. Portable cases reject body/hash/audience drift.
Freshness and malformed audiences refuse before secret IO. Seed `0x07403b21`
exercises 32 retained bodies and four independent preparations. Tests use only
owned temporary files and the already public synthetic controller key.

Actual issuer/workflow/subject/snapshot policy, bounded credential transport,
OIDC verification of the retained body, protected-job receipt and authenticated
handoff remain before original replay/common fence, authenticated start,
all-writer cessation and server/pilot/qualification admission. No raw token,
actual signing key, installed trust, host action, product workload or numerical
claim follows from the local consistency proof.
