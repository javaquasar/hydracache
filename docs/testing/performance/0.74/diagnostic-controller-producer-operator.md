# Diagnostic controller producer local usage

The separate producer prepares diagnostic request bytes and signs only with the
already reviewed controller key. It neither retrieves GitHub tokens nor contacts
the host. These commands are preparation tools, not a deployment or execution gate.
The [design](diagnostic-controller-producer-design.md) and
[contract](diagnostic-controller-producer-contract.toml) define that boundary.

## Build and prepare public inputs

On Linux, build from the reviewed controller tooling revision before making any
secret available:

```sh
cargo +1.94.0 build -p hydracache-long-run-supervisor-074 --bin diagnostic_controller --locked
```

Use absolute input paths through safe no-follow ancestry. The request must be
canonical LF DiagnosticRequest JSON, with the frozen observer source/preset,
reviewed builder receipt digest and matching binary identity. Supply the unchanged
public builder policy and signed receipt, not current tooling lock bytes. Run,
attempt and actor fields are job statements until independently verified origin
is joined at the receiver. The producer does not manufacture boot/lease identity,
request ID, nonce or a production worker policy.

```sh
diagnostic_controller audience /job/request.json /job/builder-policy.json /job/build-receipt.json
```

This prints the domain-separated audience derived from canonical request bytes
without the final LF. Audience preparation does not check current freshness;
signing and original receiver admission do. A prepared digest is not a capability.

## Sign without a public trust override

The signing seed file must contain exactly 64 lowercase hexadecimal characters,
optionally one final LF. Its caller-owned immediate directory must be 0700 and
its single-link regular leaf 0600. An SSH key, builder seed or worker issuer PEM
is not this key. The derived public key must match the existing reviewed
controller pin; there is no alternate verifier or clock option.

```sh
diagnostic_controller sign /job/request.json /job/builder-policy.json /job/build-receipt.json /private/controller-seed.hex /private/packet.json
```

Output is exclusively created, then synced with its directory. Existing output
is never overwritten. Failure after creation may leave an incomplete file;
retain failure and do not adopt that file or retry into the same path. The
producer clears its explicit encoded/decoded seed buffers, but this is not a
claim of complete process-memory erasure or crash confidentiality. Request/key
values are not echoed by refusal messages. Signing samples job UTC before/after
the signature; it cannot replace receiver kernel time or durable epoch authority.
The request expires exclusively, within its existing maximum 60-second window.
Filesystem/transport delay can make signed output stale, which the receiver must
refuse. Recreating preparation does not consume a durable nonce.

## Future protected job integration

For a future in-process Linux coordinator, retain one preparation across the
public audience step and signing:

```rust,ignore
let prepared = linux::prepare_files(request, policy, receipt)?;
let audience = prepared.audience().to_owned();
// Actual OIDC retrieval/verification and signer custody still need implementation.
linux::sign_prepared_files(prepared, &audience, key, output)?;
```

The [retained API](diagnostic-retained-producer-design.md) takes no replacement
public paths and consumes preparation on success or failure. It compares
retained body/build/digest/audience before key IO. Audience text is consistency,
not token verification or a bootstrap attestation. The separate CLI commands
above still read files independently and must not be described as this coordinator.

A separately reviewed job must freeze one request body, derive its audience,
retrieve OIDC for that audience and sign that same body. It must independently
bind exact repository/run/attempt/actor and reviewed workflow SHA/ref,
environment, subject, event and runner profile. Building or checkout after
secret exposure needs its own custody review; do not execute unreviewed branch
code with signing authority. Never print a raw token, bearer credential, seed
or place it in repository evidence, a retained artifact or chat.

The token and signed body still require the original receiver/controller/build/
worker/kernel/config/journal joins, durable replay/common fence and original-start
admission. No such job is added or dispatched here. The campaign workflow and
its longer authorization/domain cannot be used as a diagnostic producer.

## Inspect public sources offline

```sh
python scripts/perf/diagnostic_oidc_source_074.py /public/discovery.json /public/jwks.json
```

The inspector requires the fixed GitHub issuer and JWKS URI, RS256/RSA signing
keys within bounded integer/roster/JSON limits and unique fields. It reports
raw input digests and bounded public metadata. It selects no active key,
workflow pin, epoch, expiry or trust configuration. Certificate metadata is
syntax only, not a chain verification. Its explicit acquisition/enrollment/
admission flags are false; no runtime trust reader accepts this report.

Independent authenticated acquisition is a later step: fixed HTTPS endpoints,
certificate/hostname verification, no redirects or inherited proxy/token/key-log/
CA overrides, bounded responses and explicit custody/review/rotation are required.
The [GitHub discovery document](https://token.actions.githubusercontent.com/.well-known/openid-configuration)
publishes the issuer endpoint; parsing a supplied copy does not authenticate its
origin. The inspector performs no network acquisition or actual key selection.

## Public acquisition tool after separate approval

The [local acquisition implementation](diagnostic-oidc-acquisition-design.md)
adds a separate command; it was tested with modeled responses and synthetic
loopback TLS, not against the real issuer. Only after actual acquisition is
approved, run from reviewed tooling and an audited Python/OpenSSL/CA environment:

```sh
python scripts/perf/diagnostic_oidc_acquisition_074.py acquire-public
```

There is no URL, CA, token, kid or timeout option. Proxy, CA/key-log/OpenSSL,
Python and known GitHub token overrides refuse even when set to empty strings;
use a separately reviewed clean process instead of copying credentials into it.
The owned child receives only a small OS/locale environment and has a 15-second
total deadline, with five-second resource limits and no retry. Do not invoke its
internal `_worker` mode manually; that bypasses the outer child deadline.

The unsigned report retains bounded public bodies, digests, platform CA roster
digest, peer certificate/TLS metadata and job timestamps. Platform trust is not
an independently selected issuer pin, and this report cannot grant a runtime
capability. Review acquisition source/interpreter/CA custody and source hashes
before choosing an actual key, snapshot lifetime or protected workflow policy.
The tool creates no file, signs nothing and installs no trust. Existing receiver
freshness, worker epoch, replay/common fence and original-start gates still apply.
TLS and owned-child timeout use the documented
[Python ssl API](https://docs.python.org/3.12/library/ssl.html) and
[subprocess.run lifecycle](https://docs.python.org/3.12/library/subprocess.html#subprocess.run).

The separately approved [one-attempt observation](local-runs/diagnostic-oidc-public-source-20261014/README.md)
has now received the public sources under inventoried Windows platform TLS.
Use its read-only verifier for reproduction, not another acquisition command.
Its four unselected keys do not fill production membership, subject, workflow
SHA or snapshot lifetime slots. The [bounded v2 reader](diagnostic-oidc-roster-design.md)
preserves v1 and accepts a separately selected roster; no configuration is
generated or installed by observing public sources. See the [job preparation](diagnostic-controller-job-preparation.md)
before any protected environment, bootstrap, key policy or token-producing job.
