# Local public OIDC acquisition preparation

Baseline is `e007113d52a6031e3254c36fa169f495ddf56e08`. This step implements
the acquisition tool locally under the approved
[OIDC decision](diagnostic-controller-provenance-decision.md), without fetching
actual issuer keys, choosing trust/workflow pins, installing configuration or
dispatching a job. The existing offline inspector and all historical packets
remain unchanged.

## Fixed public transport and process boundary

The only public acquisition entry has no URL, CA, token, key, kid, timeout or
redirect argument. A separately owned Python child requests exactly the fixed
GitHub discovery path and fixed JWKS path over HTTPS on port 443. Direct
http.client connections use TLS client certificate and hostname verification,
SAN-only names, TLS 1.2 minimum, strict full certificate chains and platform
default CA roots. Record the loaded root roster digest, peer certificate digest
and negotiated TLS version as observation metadata, not issuer-policy pins.

Refuse inherited proxy, TLS/key-log/CA/OpenSSL and Python overrides and known
GitHub bearer-token environment names without reading or echoing their values.
The owned child uses the same exact script with Python environment/user-site
isolation and a minimal process environment. The outer 15-second subprocess
deadline bounds DNS, handshake, slow headers and body reads together. Socket
timeouts and a monotonic resource deadline additionally bound normal progress;
no automatic reconnect, alternate origin, redirect, decompression or retry.
Only that child is terminated on timeout; partial output is never accepted.

Accept only HTTP 200 with application/json, unambiguous framing, no Location,
identity encoding and at most 65536 body bytes per source. Retain both raw public
bodies as base64 alongside hashes and the existing strict inspector result.
The complete unsigned acquisition report is bounded to 196608 bytes. Network
receive time is job observation, not monotonic authority epoch, snapshot expiry
or freshness at a future receiver. Exact source and interpreter/runtime custody
remain required; Python helpers and JSON reports are not capability boundaries.

## Enrollment remains a separate decision

Use a new unsigned report schema, never a runtime trust config. Successful fixed
HTTPS observation cannot select an active signing key, review interval, workflow
revision/subject, production epoch, root ownership or actual job policy. A copied
or fabricated report cannot grant any runtime capability. No acquisition occurs
inside the privileged supervisor and no runtime verifier can refresh trust from
the network. Existing request/receipt/domains and qualification inputs are fixed.

## Local verification

Retain the current Python baseline and a missing acquisition API failure before
implementation. Add mock HTTP refusal cases for redirects/status, duplicate
headers, ambiguous or truncated framing, compression, over-budget JSON, deadline,
certificate and source mismatch, plus environment refusal before child launch.
Exercise parent failure/timeout/output bounds and seeded independent modeled
acquisitions. Use a publicly committed synthetic CA/server key pair only in
owned loopback fixtures to prove real TLS accept, hostname mismatch and unknown
CA refusal; no actual GitHub connection or production private key is used.
Do not expose test CA/host/process overrides on the public entry or CLI.

Run three focused repeats on Windows and Linux, affected Python regression,
root contracts, registry, affected check/lint, format and documentation/local
gates. Record actual TLS fixtures separately from modeled acquisition positives.
Retain every negative. No product metrics, qualification, protected job or host
operation is authorized or proved by this packet.
