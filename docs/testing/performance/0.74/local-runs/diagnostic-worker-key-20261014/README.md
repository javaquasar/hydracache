# Separate local worker issuer key evidence

The operator authorized a new separate local Ed25519 key and a generation/use
note, without replacing existing authorities or installing anything on the server.
The [note](../../diagnostic-worker-issuer-key.md) records custody, exact commands,
future policy-signing bytes and rotation. The
[public selection](../../diagnostic-worker-issuer-public.json) is the only retained
key data; the private PEM remains in local WSL storage outside Git.

## Source and implementation

Baseline is `47c59d40a19f2d04a63f044c0e62703c760ba1c8`. Preregistration is
`f5350ea96e5a41489c5e5835e72af23edcbd2e7a`; helper implementation is
`b1c53e422c26e5646c0bfffb2d8f3493d950b888`; public selection and strict Rust
selfcheck are `69f156804dad006c96c5350a5e210ee31b2336e1`.
Final fixture portability is `bd6f9014252996c527bfccc1fd137a9596706a5c`:
owned disposable test directories no longer depend on a preexisting .local/share,
and root explicitly refuses the operator helper. The generated real key and
helper implementation are unchanged.
No Rust production source, Cargo input, frozen qualification input or existing
SSH/controller/builder authority changes.

The operator helper allows only generate/inspect, uses fixed system OpenSSL
with a sanitized environment and never prints its private output/errors.
No-follow trusted ancestry and exact original objects refuse unsafe ownership,
links, Git checkouts, key replacement and incomplete attempts. Creation is
exclusive; two concurrent creators yield one winner. Files are synced before
public completion. These are local custody checks, not production enrollment
or a power-loss proof. Atime changes are excluded from metadata comparisons;
identity, permissions and nanosecond mutation timestamps remain checked.

## Actual key observations

One real issuer was generated under
`/home/artur/.local/share/hydracache-worker-issuer-074-20261014` in local WSL Ubuntu.
Directory/private/public modes are 0700/0600/0400, owner UID/GID 1000/1000.
The private key is a 119-byte, single-link unencrypted PKCS8 PEM. Permissions do
not protect against the account, Linux root or an administrator controlling WSL;
disk encryption/backup protection is not attested. This task creates no backup,
remote secret or server copy.

The raw public-key SHA-256 fingerprint is
`c714e1038a081ed841dd183e2814ce647c85e6bafaf43943bffb1e2378786cad`.
OpenSSL 3.0.13 verifies the fixed non-policy selfcheck and rejects changed bytes.
Separate inspection commands reproduce the exact retained public selection.
Rust ed25519-dalek strict verification accepts that proof, rejects the
worker-policy domain and controller key, checks nonweak/distinct public keys,
and confirms that policy body pin and epoch remain null. No worker policy is signed.

## Local gates

The existing broad worker-policy baseline passes 244 checks with nine manual
cases ignored, plus seven reviewed-trust checks. The first helper red capture
refuses the missing module. Initial helper tests pass 14; reviewed helper source
passes 18 in each of three repeats and Python compilation. Windows passes seven
portable checks and explicitly skips eleven Linux custody cases.

The final Rust selfcheck target passes eight in each of three Linux repeats and
eight on Windows. Current all-target/all-feature check and strict Clippy pass on
both; MSRV check passes on both. Linux MSRV Clippy remains unavailable and is not
claimed. Root registry/performance/evidence suites pass 6/124/13 checks. Scoped
format, doc-check, local performance contract, links and book build pass.
Require-ship returns expected native exit 1. No full release gate, manual crash
suite, server rehearsal or numerical product measurement ran.

## Integrity and remaining authority

The [manifest](manifest.json) binds source/public/frozen inputs and raw captures.
The [verifier](verify_packet.py) checks retained bytes, public key/fingerprint and
generation/inspection consistency without opening the private key:

```powershell
C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-worker-key-20261014/verify_packet.py
```

It is an integrity check, not another cryptographic implementation or current
host authority. Independent Rust signature verification is retained separately.
Issuer selection is now available; signed production host/account policy,
canonical body pin, current epoch, trusted installed ownership and P1/P2/P3
original-start/watchdog/writer-cessation/completion joins remain open. No local
selfcheck permits a workload or advances cells two through four.
