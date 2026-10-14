# Actual public OIDC source observation

One normal acquire-public invocation received GitHub discovery and JWKS on
2026-10-14. Source is `709d1165f3dc8bfc8aed5630caa58a5a06a14d83`; the
[one-attempt plan](../../diagnostic-oidc-public-source-plan-20261014.md) was
committed as `d0cca7594391cbd9c262cafc80f6b6703e8ed9b9` before the read.
No acquisition/runtime/parser source or original qualification input changed.

## Received public data

Both fixed endpoints returned HTTP 200 over TLS 1.3 with the same observed peer
certificate SHA-256 `2a6aa56347230d695e4ab7a8e5b6f8672d18c2bf98790d06d54dd0706388e6a1`.
The 1217-byte discovery body and 5547-byte JWKS body are retained byte-exactly as
base64 in the [canonical report](acquisition-report.json). Their SHA-256 values
are `c3fba014c6fc3be9f3950c3ce883ceb2670d586dd2efbfec9342d49bef33516b`
and `5f8164c9696c27837ad18c5fe7c7022a433bd5390183e8ff15c74d82233270f4`.
The 11115-byte report SHA-256 is
`77cd206c0302245203caba3f6d7b90ec82a88c2d8ba32841a9d929470093f0bb`.

The roster contains four distinct 2048-bit RSA/RS256 signing keys. Their IDs and
projection digests are inspectable in the report; no active key was inferred
from ordering, certificate metadata or a token. Discovery advertises the origin,
run/attempt/actor/workflow/environment claims used by the configured verifier;
that advertisement is not a received or verified protected-job token.

## Runtime and environment boundary

The normal inherited environment contains CURL_CA_BUNDLE. Its guard refused
before network and did not read or print the value. A separately owned clean
Windows process received only the documented OS/temp/locale environment. The
parent environment, CA stores, proxy settings and TLS guards were not changed.
Its ordinary entry retained the owned worker/resource deadlines and no retry.
See the [status](acquisition-status.json); stdout/stderr and native outcome are
retained without a token, signing secret, installed config or workflow dispatch.

Python 3.13.7/OpenSSL 3.0.16 and source/runtime inventory match around the read.
Both inventories and the transport report share the same 69-root CA roster
digest `dc4dfefbd722e067db82e22d9849be097b6701c51979ffe5c7813b86aac24887`.
The synthetic test CA is absent from platform roots. Matching file hashes and
public CA inventory are not original mapped-binary, physical host or production
custody enrollment; those claims remain false.

## Verification without another request

Independent base64 decoding and offline inspection reproduce both source digests,
the key roster and every closed selection/config/job/admission flag. The verifier
also checks exact committed source, frozen locks, public artifact/capture hashes,
CA and runtime consistency and the one-invocation status. It performs no refetch,
key selection or installation. Six copied-packet cases cover an unmodified
control, raw mutation and rehashed admission, future-schema, foreign-origin and
zero-versus-false refusals; tests never edit this retained packet.

```sh
python -E -s docs/testing/performance/0.74/local-runs/diagnostic-oidc-public-source-20261014/verify_packet.py
python -E -s docs/testing/performance/0.74/local-runs/diagnostic-oidc-public-source-20261014/test_verify_packet.py
```

Windows-managed worktree metadata needs the exact translated GIT_DIR for WSL
read-only Git operations. The ordinary 25-case Python suite is regression only:
its TLS/network positives remain local synthetic fixtures and do not refetch
these public sources. No numerical benchmark result follows from this packet.

The six copied-packet cases and 25 ordinary regression cases passed on Windows
and Linux. Eleven raw captures retain those outcomes and documentation gates.
Doc-check, the 169-gate registry, local performance contract, local links, book
build, formatting and diff checks passed. The separate require-ship check still
refuses with the expected exit 1; no full release gate or Rust runtime test was
rerun for this documentation/evidence-only step.

## Next job and selection boundary

The [dedicated job preparation](../../diagnostic-controller-job-preparation.md)
records the existing fixed profile, separate build/sign authority, immutable body
and token handoff gaps. V1 selects one issuer key while this JWKS contains four;
an arbitrary first key is not an active-key policy. A versioned bounded trusted
roster is proposed for separate review, not selected or implemented here.

This is actual public HTTPS observation under the inventoried platform trust,
not production issuer/workflow/subject/expiry selection or root installation.
Protected-job custody/receipt, worker policy/epoch, trusted host/root, original
replay/common fence, authenticated start and all-writer cessation remain before
completion, later cells, server/pilot or qualification admission.
