# Generate and inspect the separate signer build issuer

The operator authorized this new local Ed25519 pair on October 15, 2026 for the
signer-build receipt role. It is distinct from SSH, controller, observer builder
and worker-policy keys. Generation is complete; approval of its public fingerprint
and enrollment of production custody remain separate.

## Current public candidate

The [public record](diagnostic-signer-issuer-public.json) contains the raw public
key and a signature over a fixed non-receipt selfcheck. Its SHA256 fingerprint,
computed over the 32 raw public bytes, is:

```text
0c22ba7d6c6d16b334e4b1d4a0ce161117800135a977aa30020906caa70af884
```

This is not a private-file hash, SSH fingerprint, signed build receipt, approved
trust pin or host authority. OpenSSL inspection and independent Rust strict
Ed25519 verification on Windows/Linux agree. The public record deliberately
keeps pin approval, production enrollment, receipt signing, workflow installation
and admission false. It is inventory, not an automatically loaded trust file.

## Storage and commands

The private key remains at
`/home/artur/.local/share/hydracache-signer-build-issuer-074-20261015/signer-build-private.pem`
in WSL Ubuntu, user artur. The directory is 0700, private file 0600, public file 0400;
UID/GID are 1000 and both files have one link. The PKCS8 PEM is unencrypted.
These permissions do not prevent access by the account, root or a WSL
administrator, nor establish disk encryption or production custody. No backup,
remote copy or GitHub secret registration was performed. Do not open, paste,
hash, log or commit the private contents.

The generation command was executed once, with an isolated system Python and
clean environment. **Do not rerun it against this existing destination.**

```powershell
wsl -d Ubuntu -u artur --cd /mnt/c/Workspace/prj/jq/cashe/hydracache-074 --exec /usr/bin/env -i PATH=/usr/bin:/bin LANG=C /usr/bin/python3 -I scripts/perf/diagnostic_signer_key_074.py generate /home/artur/.local/share/hydracache-signer-build-issuer-074-20261015
```

For a later public inspection, use:

```powershell
wsl -d Ubuntu -u artur --cd /mnt/c/Workspace/prj/jq/cashe/hydracache-074 --exec /usr/bin/env -i PATH=/usr/bin:/bin LANG=C /usr/bin/python3 -I scripts/perf/diagnostic_signer_key_074.py inspect /home/artur/.local/share/hydracache-signer-build-issuer-074-20261015
cargo run -p hydracache-long-run-supervisor-074 --example verify_signer_issuer_public --locked -- docs/testing/performance/0.74/diagnostic-signer-issuer-public.json
```

Inspection uses the retained private key only inside OpenSSL to derive public
data and sign the fixed selfcheck, never arbitrary input. It may update access
time. Different filenames and a public-role check before private open prevent
the new helper from adopting the existing worker key directory. The trusted-home
walk rejects links, writable/foreign ancestry and Git checkouts. Existing,
incomplete or uncertain generation state refuses without overwrite or repair.

## Future receipt use and rotation

After explicit fingerprint and custody approval, independently approve the exact
SignerBuildStatement and its original prebuilt inputs. The existing receipt
model uses pure Ed25519 over:

```text
ASCII("hydracache-diagnostic-signer-build-074-v1") || 0x00 || canonical_statement_JSON_without_LF
```

The signed envelope is canonical JSON with one final LF. Do not substitute the
selfcheck, hash-only signing, a worker-policy domain, an incoming bundle's own
expected pins, or an invented epoch. The current helper intentionally has no
receipt-signing operation. Actual signing must wait for the reviewed prebuilt
launcher/runtime/job path described in the [custody design](diagnostic-signer-custody-design.md).

Rotation uses a new exclusive destination, independently reviewed public
selection and explicit revocation of old authority. Losing a private key cannot
be repaired from its public record; deleting a file cannot revoke signatures.
An exposed key requires closing affected admission and a reviewed rotation.

OpenSSL documents [Ed25519 generation](https://docs.openssl.org/3.0/man1/openssl-genpkey/)
and [pure Ed25519 raw signing](https://docs.openssl.org/3.0/man1/openssl-pkeyutl/).
