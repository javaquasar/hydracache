# Separate signer issuer and isolated signing job

This stage creates a new local Ed25519 signer-build issuer candidate and prepares
the signing lifetime. The public fingerprint still needs operator approval;
generation is not production issuer enrollment or build provenance. The
[preregistration](diagnostic-signer-custody-plan-20261015.md) fixes the local scope.

## Local key boundary

The role-fixed `diagnostic_signer_key_074.py` exposes only generate and inspect.
It uses different private/public filenames from the existing worker helper,
rejects the reviewed worker/controller/observer keys, and never accepts a message
to sign. The only signature is a fixed public selfcheck from a non-receipt domain.
Inspection validates the exact canonical public role before opening the private
file. Its public record always says pin unapproved, production unenrolled,
receipt unsigned, workflow absent and admission closed. Changing any such field
makes inspection refuse, rather than installing a trust policy.

Creation requires absent state under trusted local-home ancestry, 0700 directory,
0600 single-link private PKCS8 PEM and 0400 public selection. Original descriptors
and metadata are checked through OpenSSL operations; identity drift, links,
writable peers, Git ancestry, partial state and existing targets refuse without
repair. A failed attempt stays retained. Sanitized failures never echo tool
output, private contents or a private-file digest. The key is unencrypted;
local account, root and WSL administrator access remain in the trust base. File
modes do not establish storage encryption or production custody.

The separate Rust example `verify_signer_issuer_public` reads only a supplied
bounded public record. It checks canonical encoding, every closed field,
SHA256 of the raw public key and a strict Ed25519 signature under that key.
It outputs public inventory only, without a library trust loader or capability.
This independently checks the generated public selfcheck, not its owner, current
authority, approved artifact or protected GitHub origin.

## Isolated job draft

The [disabled draft](diagnostic-signer-job-074.yml.disabled) is outside
`.github/workflows`. Both jobs have a false guard and an unconditional failure;
there is no executable signing implementation or secret reference. Its future
build job has no signing/OIDC authority. Its future signer job accepts only an
independently admitted prebuilt launcher/signer/runtime, with no compilation or
dependency installation. The draft OIDC permission illustrates job-wide scope,
not an installed permission. Existing workflows remain byte-identical.

Before activation, independently select and review all of these inputs:

- The new issuer fingerprint and exact signer-build statement/receipt digest;
  never derive expected pins from the incoming bundle being authenticated.
- Original prebuilt launcher, signer, GNU interpreter/libraries, runtime root,
  loader configuration, environment, inherited descriptors, later-load policy
  and TLS trust sources. Refuse unsupported runtime shapes.
- Exact workflow revision/ref, repository/actor/event/runner policy, protected
  environment and expected subject form, public issuer snapshot membership and
  expiry. No default or guessed authority epoch is introduced.
- Kernel and runner-administration trust, protected root/ancestor ownership,
  absence of untrusted same-UID mutation/debug access, and no credential-bearing
  processes left from compilation. Inventory or hashes alone prove none of these.

GitHub documents [job-scoped OIDC authority](https://docs.github.com/en/actions/reference/security/oidc#workflow-permissions-for-the-requesting-the-oidc-token).
All steps in that future signing job belong to the trust base before its token
permission exists. A local handoff or environment cleanup cannot retrospectively
make an untrusted job safe. Exact SHA/subject/runtime values remain unresolved
until their artifacts and custody have been reviewed; this draft cannot dispatch.

## Following implementation

After public pin approval, implement authenticated prebuilt input acquisition
and original launcher/runtime ownership under the selected profile. Join actual
retained-body token coordination and receiver replay/common fence only afterward.
Worker original-start and revocation of every P2/P3 writer still precede completion
publication, cells 2 through 4 and the corresponding SIGKILL/lost-ack matrix.
Installation, nonproduct server rehearsal, product pilot and release qualification
remain separate admissions. No actual build receipt or worker policy is signed here.
