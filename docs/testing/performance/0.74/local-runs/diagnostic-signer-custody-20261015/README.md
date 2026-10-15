# Local signer issuer and custody preparation

A distinct local Ed25519 signer-build issuer candidate was generated once after
disposable fixtures and pre-generation checks. Its public selfcheck verifies
with OpenSSL and independent Rust strict Ed25519 on Windows/Linux. The private
key stays outside Git in the WSL Linux home; only the public record and public
selfcheck output are retained. Existing worker/controller/observer keys were
not opened or changed. No build receipt, worker policy or OIDC token was signed
or requested, and no GitHub/server operation was performed.

## Source and actual generation

Source is `2b90c66593e6fabbec407f1aea743d4987915e38`, baseline
`0b6eff00cad0c8497987523a0016e0cc42ed3a76`, preregistration
`d408c294f26d08922d914fdeab69153bcd197c6a`. The manifest pins nine source files,
25 unchanged inputs and 17 raw captures. Prior packets and frozen qualification
remain unchanged. Generation and inspection JSON must match the published
public record byte for byte. The fingerprint is SHA256 of its 32 public bytes:

```text
0c22ba7d6c6d16b334e4b1d4a0ce161117800135a977aa30020906caa70af884
```

**Public pin approval and production enrollment remain false.** Generating a
key and proving its fixed selfcheck do not authenticate a build, immutable
runtime, protected job or host authority. The key is unencrypted PKCS8 PEM,
with 0700 directory, 0600 single-link private file and 0400 public file, UID/GID 1000.
The local account/root/WSL administrator remain trusted; no encryption or backup
claim follows. See the [operator procedure](../../diagnostic-signer-issuer-key.md).
Do not rerun generation at the existing destination or read private contents.

## Checks and retained negatives

The missing-helper capture is the expected ImportError before implementation.
Initial 22-case helper repeats precede the disabled-draft guard; the final helper
has 23 tests and passed three nonroot Linux repeats. Windows runs 23 with 14 explicit
Linux-only skips, not 23 filesystem positives. The unchanged worker helper's 18
Linux tests and existing signer-model 9/reviewed-trust 8 regressions pass.

The Rust public-inventory example has three tests: exact synthetic selfcheck,
each field/all 64 signature bytes, and malformed/unknown/duplicate/noncanonical
or oversized reports. Current 1.94 and MSRV 1.88 tests/checks pass on Linux and
Windows, with strict current clippy, scoped format and Python syntax checks.
149 source contracts, doc-check, 169 registered gates and the local non-promotable
contract pass; require-ship remains expected red. The actual generated public
record then passed the Rust example on both platforms. These are correctness
results, not cache performance measurements or production qualification.

The role-fixed helper preserves exclusive creation, retained descriptor/inode
checks, trusted-home/no-Git ancestry and sanitized crypto errors. The new role
uses different filenames/domain, rejects old public keys and validates public
role before private open. Disposable tests exercise incomplete state, symlinks,
hardlinks, mode drift, replacement, concurrency, duplicates and wrong role.

## Job preparation and reproduction

The [job draft](../../diagnostic-signer-job-074.yml.disabled) is outside
`.github/workflows`, with two false guards and two unconditional failures.
No signing secret or executable launcher route exists. It records separation
of future build/dependency work from the token-bearing prebuilt signing lifetime.
Exact runtime, root ownership, workflow SHA/ref/subject, environment protection,
issuer snapshot validity and authority policy must be reviewed before activation.
No guessed values or auto-enrollment are supplied.

Run `python verify_packet.py` and `python test_verify_packet.py` here. The
read-only verifier checks source/baseline Git objects, immutable inputs, exact
raw capture hashes, suite counts, public-record equality and closed admission.
Nine mutation tests cover drift, rehashed zero suites/lost public proof,
promoted public approval, admission/source forgery, missing captures, duplicate
JSON, attributes and exact committed bytes. `--committed <evidence-SHA>` also
compares all packet/public bytes with Git without line-ending normalization.
All nine mutation tests pass on Windows and Linux.

Next approve the public fingerprint, then implement admitted original prebuilt
launcher/runtime/job custody and retained-body credential coordination. Worker
original-start, every P2/P3 writer's cessation, completion cells 2–4 and all
server/product/qualification admissions remain separate and closed.
