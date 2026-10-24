# Rustls security update and session redaction regression

Commit `e02448617c7073e499c1d6ddbf7d3330518db0ae` raises the workspace rustls
minimum to 0.23.46 and updates only that resolved package from 0.23.45. The
first changed Windows MSVC checks pass, including Rust 1.88. Dependency policy
still refuses on eleven other advisories; release admission remains closed.

## Baseline and isolated change

The preceding [security plan](dependency-security-plan-20261024.md) retained the
fifteen-advisory full-milestone refusal. The separate rustls patch addresses the
four upstream findings patched by 0.23.46:
[session Debug secrets](https://rustsec.org/advisories/RUSTSEC-2026-0346.html),
[ECH buffered data](https://rustsec.org/advisories/RUSTSEC-2026-0347.html),
[Brotli allocation](https://rustsec.org/advisories/RUSTSEC-2026-0348.html), and
[unauthenticated key-exchange hints](https://rustsec.org/advisories/RUSTSEC-2026-0349.html).
The targeted regression below exercises session redaction, not all four exploit
paths. The other three fixes are supplied by the selected upstream release;
their applicability depends on configuration and must not be conflated.

`Cargo.toml` changes only the rustls minimum from 0.23.41 to 0.23.46.
`Cargo.lock` changes only rustls's version and registry checksum; every other
resolved package record remains equal. No dependency exception, crypto-provider
selection, MSRV, tenant policy, expiry rule, wire format, original live check or
deadline changes. This dependency delta requires a new eventual candidate/build
identity; old performance, signed build or qualification receipts are not reused.
The new working Cargo.lock SHA256 is
`552f8c1fbb9fb06aadefca3fffb4f90cc1b78f4aed3e2c001cdf399b9b6bf0b1`;
the earlier baseline lock bytes and their hash remain in the first-result packet.

## Public API regression and first fixture refusal

The new ordinary test target is
`crates/hydracache-redis-compat/tests/tls_session_security_074.rs`. It installs a
bounded custom `ClientSessionStore` around the normal memory cache, then exercises
actual in-memory TLS 1.2 and TLS 1.3 full and resumed handshakes. Both directions
of handshake identity are checked, and binary application bytes are delivered
unchanged. Session insertions inspect both compact and pretty Debug; only boolean
evidence/counters survive. Session strings and fixture secrets are not logged.
Independent oracle controls reject raw secret payloads, repeated unsafe fields
and trailing data after an apparent redaction marker.

The first fixture's four-ticket cache rounds to one server slot. Its upstream
insertion/eviction behavior removes the only server entry, so both resumption
assertions fail before the secrecy oracle. That first outcome and exact test
source are retained. A separate fixture uses a bounded thirty-two-ticket cache;
it changes no production setting or release threshold.

With that corrected fixture on unchanged rustls 0.23.45, both full/resumed
handshakes and binary exchanges succeed, then TLS 1.2 and TLS 1.3 fail exactly
at the secret-field oracle (two and four unsafe insertion observations).
The baseline and candidate oracle source bytes are identical. On 0.23.46, all
three tests pass on Windows MSVC Rust 1.94 and MSRV 1.88. This is a functional
security regression, not a CPU, allocation or throughput experiment.

## First changed Windows gates

| Gate | First changed result |
| --- | --- |
| Session secrecy, resumption and oracle controls, Rust 1.94 | 3 passed, no ignored/filtered cases |
| Same test source, MSRV 1.88, separate build directory | 3 passed, no ignored/filtered cases |
| Complete ordinary RESP package | 181 passed, 23 unchanged ignored cases, no filtered cases; 15 successful target/doc verdicts |
| Production RESP mTLS integration | 11 passed, including invalid clients, foreign CA/name, AUTH/tenant boundaries, fragmented frames, incomplete handshake timeout, connection cap and shutdown |
| Real daemon HC1/HC2 lifecycle | 4 passed, including shared dispatch, readiness failure and drain/exit |
| RESP and server all-targets check | Passed on Rust 1.94 |
| RESP and server all-targets/all-features strict Clippy | Passed on Rust 1.94 |
| RESP package format check and diff whitespace check | Passed |

These counts describe distinct scopes; they are not summed into a complete
workspace gate. A separate local Ubuntu WSL GNU Rust 1.94 control also passes
the same three tests with no ignored/filtered cases. Its own target directory
and native 900-second timeout/10-second kill grace are separate from all original
launch deadlines. It neither reruns the old-version Linux baseline nor attempts
original-owner preparation. No service, clock or dedicated-server action occurs.

## Dependency policy remains red

The full `cargo deny --locked check --disable-fetch` uses the same clean advisory
database commit `84d943a6deeb8cc6c6f632531aaf4f1c62275239`. It reports advisories
FAILED, bans/licenses/sources OK. The four rustls findings disappear; eleven
remain: WebPKI, two trust-root packages, Tokio, YAML, three zbus wire/name/derive
packages and three ZIP findings. No warning is hidden and no target/dev dependency
is excluded. This focused policy check is not a new complete `cargo xtask verify`.

## Retained evidence and remaining boundaries

The [66-member Windows packet](local-runs/rustls-security-20261024/index.json)
has 63 objects and index SHA256
`ed6fd42ee402c6f59f2b803097bed32ce9fa94b6d4550aa90fe47f6efa82a296`.
It retains first fixture/baseline/candidate logs, complete commands and source
hashes, raw baseline/candidate test files, lock/manifest inputs and changed strict,
MSRV, mTLS, daemon and policy outcomes. It records the final coordinator variant;
the earlier pre-lock-mutation coordinator variant was not separately captured.

The [22-member Linux and independent-verification follow-up](local-runs/rustls-linux-20261024/index.json)
has 20 objects, index SHA256
`a28c7865d085a3873637551543def6ee5b4e3b44143ebf0707f7d4fd1dc77124`.
It retains the actual GNU compiler metadata, separate first changed result and
independent complete padded-advisory-ID set comparison with four rejection
controls. An earlier metadata-only command labelled rustls-linux ran Windows
Cargo's version command; its exact intent is retained and is not Linux evidence.

Subsequent `cargo +1.94.0 xtask doc-check` and local, non-promotable
`performance-contract-check --release 0.74` pass. Their
[eleven-member structural packet](local-runs/rustls-governance-20261024/index.json)
has eleven objects, index SHA256
`2b924c1f870af563701f405b3cf2b9729dae79181bce06e6f0a4faab4476d5af`.
Code commit: `e0244861`. The preceding Windows/Linux packets are committed
separately in `ec044bd0`; no prior archive or release receipt is rewritten.

Next isolate WebPKI/trust-root updates from Tokio and major YAML/ZIP/D-Bus
migrations. The zbus 6/Rust 1.89 conflict remains an explicit decision; the project
stays on Rust 1.88. Exact-source contract failures, complete preparation fit,
original manager delivery, execution/watchdog/all-writer retirement, completion,
crash/reconnect and real enrollment remain in the
[closure audit](release-closure-audit-20261024.md).

The frozen qualification manifest keeps SHA256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
The published/frozen 0.73 worktree and historical evidence are untouched. No
release install, existing-service/supervisor maintenance, clock change, dedicated
server action, product performance workload or expensive qualification follows
from this security patch.
