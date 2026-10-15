# Local signer issuer generation and custody preparation

Baseline is `0b6eff00cad0c8497987523a0016e0cc42ed3a76`. The operator's continuation
authorizes one new local signer-build issuer key and preparation of an isolated
signing-job/runtime design. It does not approve the generated public fingerprint,
install a trust pin, upload a secret, modify GitHub protection, execute a job or
operate a server. Existing worker/controller/observer keys stay untouched.

## Key helper and one generation

Add a separate role-fixed operator helper, without changing the existing worker
key helper. Preserve its descriptor-relative trusted-home ancestry checks,
exclusive creation, 0700 directory, 0600 single-link private PKCS8 PEM, 0400 public
selection, retained identity/metadata checks, fsync and bounded sanitized OpenSSL
subprocesses. No arbitrary signing operation or secret output is exposed.

Use different filenames, schema, purpose and a fixed non-receipt selfcheck domain.
Reject the three old reviewed public key roles. Inspection must check canonical
public role/shape before opening any private key. Unknown fields, duplicates,
role confusion, unsafe ancestry, links, mode drift, replacement or partial state
refuse without overwrite, repair, retry or fallback. Local same-UID/root/WSL
administrator access and unencrypted storage are not production custody proof.

First retain baseline worker/helper and signer-model tests; then a missing-helper
red test. Run disposable synthetic filesystem/crypto fixtures three times on
nonroot Linux, portable Windows tests with explicit Linux skips, source checks
and existing Rust regressions before touching the real destination. Preserve
negative results. Generate once as WSL Ubuntu user artur into the previously
absent `/home/artur/.local/share/hydracache-signer-build-issuer-074-20261015`.
Use only system Python/OpenSSL with clean tool environment; never read, print,
hash or copy private key contents into evidence. Retain public canonical output,
raw-public SHA256 and a fixed selfcheck signature. A second inspection must match
the public result exactly. An uncertain/partial outcome stops real generation.

## Job and runtime preparation

Keep a non-executable job draft outside `.github/workflows`, disabled explicitly.
It separates compilation/dependency acquisition (no signing/OIDC authority) from
the dedicated signing lifetime. The signing draft has no compiler/package install,
no arbitrary executable, and remains disabled until exact prebuilt launcher,
signer/runtime root, receipt, workflow SHA, subject, environment protection,
issuer snapshot validity and authority pins have been independently reviewed.
An artifact name or runtime hash alone does not establish that custody.

Document trusted kernel/runner administration and the absence of untrusted
same-UID writers/debuggers. Read-only GNU runtime includes actual interpreter,
libraries, loader configuration/environment, later loads and TLS trust inputs.
Select a supported closed runtime profile or refuse, rather than inferring it
from the GNU target. Do not grant job-wide token authority before reviewing all
steps in that job. A launcher guard cannot retroactively sanitize its parent job.

## Exit gate and remaining authority

Add public schema/domain/role and disabled-draft drift guards. Verify the generated
public selfcheck independently with Rust strict Ed25519 without adding a default
trust loader. Run Python checks, affected Rust check/clippy/MSRV, documentation,
gated registry and local non-promotable performance contract. Commit source
separately from actual public generation/evidence/article, then verify Git bytes
before the feature push. Frozen qualification inputs and prior packets remain
unchanged. Production trust enrollment, actual build-receipt signing/token
coordination, authenticated worker start, P2/P3 all-writer cessation, completion
cells 2 through 4 and server admission stay closed.
