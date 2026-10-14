# Generate and use the separate worker policy issuer key

The operator authorized one new local Ed25519 key pair on October 14, 2026,
without replacing SSH, controller or builder keys and without installing anything
on the server. This key is reserved for diagnostic worker-policy signing.
Generating a key does not select a host/account policy, its digest or an epoch,
enroll an installed authority or permit execution.

## Generation and storage contract

Use a local nonroot Linux account in WSL with system Python and OpenSSL. Keep the
private key in the Linux filesystem outside Git: a new exclusive directory with
mode 0700 and a single-link regular PKCS8 PEM file with mode 0600. The private
file is unencrypted; these modes do not prove disk encryption or prevent access
by the account, Linux root or an administrator controlling WSL. Do not copy it to
the performance host, repository, clipboard, chat, command arguments or CI logs.
No backup, GitHub secret registration or remote enrollment accompanies generation.

The narrow operator helper will support `generate` and `inspect` only. It walks
trusted ancestors without following links, rejects writable/foreign ancestry
and Git checkouts, and refuses any existing generation target without overwrite
or permission repair. OpenSSL receives retained key descriptors, never private
text in arguments/environment. Subprocess diagnostics are sanitized. Output
contains only public key, SHA-256 of the raw 32-byte public key, storage metadata
and a signature over a fixed selfcheck message from a separate non-policy domain.
Inspect verifies the retained public selection against the actual private key.
Partial generation remains closed; do not rerun generation over uncertain state.

OpenSSL supports [ED25519 generation](https://docs.openssl.org/3.0/man1/openssl-genpkey/).
Its [pure Ed25519 signing](https://docs.openssl.org/3.0/man1/openssl-pkeyutl/)
requires raw input of known length, without an external digest. Public extraction
checks the exact Ed25519 SubjectPublicKeyInfo form described in
[RFC 8410](https://www.rfc-editor.org/rfc/rfc8410.html).

## Local checks before generating the real key

Baseline the existing worker-policy and reviewed-trust tests. First retain a
missing-helper red test, then implement and test generation, roundtrip inspection,
exclusive replay refusal, symlink/hardlink and mode rejection, unsafe/Git ancestry,
incomplete/corrupt selection, key replacement, sanitized subprocess failures and
exact public encoding. Positive filesystem/crypto fixtures use disposable keys
under owned private Linux test directories, never the final issuer key.

Run three helper repeats, portable Windows checks with Linux-only cases reported
as skipped, Python source validation, existing Rust policy regression, public
selfcheck verification with Rust strict Ed25519, documentation/contract checks,
and exact public/raw evidence hashes. Never print private material or a private
key digest in test diagnostics. No worker policy is signed during the selfcheck.

## Using the key after policy approval

First obtain an independently admitted host/account context and approve the exact
nonroot UID, primary GID, supplementary groups, machine/boot identity, user/mount
namespace identities and complete passwd/group hashes. Select the current epoch
through the authority procedure, not a default of 1 or wall-clock time. Additional
receiver PID/time namespace and original authority/fence joins remain required.

Build the strict `diagnostic-worker-policy-074-v1` body from those approved facts.
Use canonical sorted-key JSON with a single final LF and the existing 4096-byte
limit. Its raw SHA-256 is the independent policy-body pin, not the hash of the
issuer selection file or signed envelope. Sign exactly:

```text
ASCII("hydracache-diagnostic-worker-policy-074-v1") || 0x00 || canonical_body_with_LF
```

Pure Ed25519 signs those bytes directly; do not sign only their SHA-256, use
Ed25519ph, reuse the selfcheck signature or add another newline. The separate
`diagnostic-signed-worker-policy-074-v1` envelope contains the body and lowercase
128-hex signature; canonical envelope limit is 8192 bytes. Existing receipt v1
does not gain worker-authority fields.

Construct WorkerPolicyTrust from the independently selected raw public key,
canonical body digest and current epoch, then call the existing
`verify_worker_policy` with independently admitted context. Verification of
supplied assertions alone is not installed-host enrollment. No production signer
CLI, policy generator, automatic issuer loader or epoch update is added here.

## Rotation and recovery

Do not regenerate this path to rotate a key. A new pair means a new public trust
selection, separately reviewed policy/epoch and explicit old-authority revocation.
Do not lower the epoch or make an old policy current after a failed attempt.
Loss of the private key requires reviewed rotation, not recovery from the public
file. Suspected exposure requires closing affected admission and explicit
revocation; local file deletion alone cannot revoke already published signatures.
