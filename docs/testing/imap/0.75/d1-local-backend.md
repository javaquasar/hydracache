# D1 production local backend extraction

Work resumed on 25 November 2026 after merging main
`3453c2e7b1b2b1e7806d7b689a08b3f51275f614` into the 0.75 branch at
`87309fb4483c19dfa701882446453dabbc02d1d4`. Main contributes the Rust 1.88-compatible
`kube-client` OpenSSL CA backport and its CI check. The previous four W12 receipts at
`42fdea87c5e1d0207498786bdf667ee94cb977f2` remain evidence for that exact historical source.

## Production ownership

`LocalClientDataBackend` now owns the existing local map, expiry clock/cursor and retained
idempotency outcomes. `ClientSurfaceEdgeState` owns identity admission, bounds, audit,
instrumentation, subscriptions and the separate CP lock service. `ClientSurfaceState` remains
a local compatibility/diagnostic facade holding references to those same owners. It creates
no second map. The memory ownership registry accounts the extracted owners separately.

HC/1 routes, HC/2 invocations and RESP execution plans await one shared
`ClientSurfaceDispatcher`. The dispatcher validates identity and protocol before constructing
`VerifiedClientRequest`; external implementations cannot manufacture that token. Backends
return a domain outcome, while the edge retains correlation and protocol metadata. An external
backend constructor creates no spare local data owner and cannot grant distributed readiness.
Backend failure is returned explicitly, without a local-data fallback.

There are exactly two local dispatch candidates: static enum dispatch and trait-object dispatch
with a boxed future. Both use the same local implementation. Existing synchronous client-surface
helpers are restricted to static local dispatch and reject asynchronous modes before effects.
The RESP synchronous compatibility helper polls its explicitly local backend once; network
listeners use asynchronous execution. Ordered RESP followups and HC/2 batch items remain serial.

Subscription operations and all five CP lock operations stay on the edge instead of entering
the data backend. Local expiry maintenance remains in place before those controls. HC/2 drops
a pending backend future when its response stream closes; an RAII guard releases invocation
accounting on success, error and cancellation.

No wire format, durable identity, protocol version or distributed-capability flag changes here.
This request token is a local execution boundary, not the frozen distributed command/receipt
format required by D2-D5.

## Verification

The existing client-surface, protocol goldens, quota, retention, mutation-event and conformance
tests are retained. The boxed candidate runs the same fourteen conformance assertions. The new
production `imap_backend_contract_075` suite checks pending execution, cancellation, correlation,
tenant/protocol/deadline refusal, control isolation, synchronous refusal and HC/1 wire dispatch.
RESP tests exercise both candidates with binary values, expiry, tags and pipelined ordering.
HC/2 tests exercise pending completion, disconnect and explicit future cancellation.

The final Linux instrumented run passed 784 tests with zero failures; 29 existing gated or
scheduled checks stayed ignored. Package `check --all-targets` and
`clippy --all-targets --all-features -- -D warnings` passed for all three changed packages.
Rust 1.88 `check --all-targets` passed, as did the three vendored `kube-client` OpenSSL CA tests.
On Windows, transport tests with all features, the new binary/TTL/tag RESP test, package
checks and strict Clippy passed. Documentation and memory ownership checks passed as well.

LLVM reports 278/278 executable lines and 26/26 functions covered in the new `backend.rs`
dispatcher. The mechanically moved legacy owners are not at 100% line coverage:
`edge_state.rs` is 434/466 and `local_backend.rs` is 1,066/1,175. These are measured file totals,
not a claim of complete branch coverage. Raw profiles, exported coverage and 73 frozen
content-addressed executable objects are retained on AX42; no existing coverage threshold
or baseline was lowered.

Working logs are retained locally under `target/imap-075/w0-d1-20261125/` and on AX42 under
`/home/hydracache-admin/hydracache-075-qualification/development/w0-d1-evidence/`.
These development checks do not constitute a new W12 receipt. The existing W12 changed-line
scope does not cover these newly changed production transport files.

The Linux worker requires a process-local soft `RLIMIT_NOFILE` of at least 65,536 for the
unchanged HC/2 test with 1,000 simultaneous clients. AX42's inherited soft limit of 1,024
was exhausted by client and server sockets. That diagnostic attempt was retained and its
owned test process stopped; the repeat raises only its own limit, without changing host
configuration or reducing test cardinality.

## AX42 placement confirmation

The 26 November placement check used the original B74 and static-candidate build records from
the `89ec8970c66df1ae03b7a58bca3d6a7be573cf93` comparison. On the new AX42 profile the Tokio
worker ran on physical core 3 and the main thread on physical core 4. Each of the two affected
cells got twenty new B74 A/A pairs, equally split between the two orders. Raw embedded `GET`
measured sixteen million operations per attempt and typed embedded `PUT` measured six million.

All 80 attempts stayed below the 5 ms run-queue limit. The typed embedded `PUT` control passed
all four bands. Raw embedded `GET` p99 had a 95% A/A interval of -1.39% to +3.04% against the
frozen ±3% band. The preregistered stop rule ended the campaign before A/B. The previous
configuration's comparison receipts and the newer calibration receipts remain available.

The verified archive is `target/imap-075/d1-put-clock-20261125/d1-isolated-confirm-20261126.zip`
with SHA-256 `fc7fd124b126bdd093d7cb8b9b060d9566828fe99586618837d68672b2cd6461`.

## AX42 placement recalibration

The next preregistered campaign retained the binaries, the split-core placement and the measurement
lengths. Raw embedded `GET` used forty fresh pairs, split evenly between both orders, and typed
embedded `PUT` used twenty. Raw embedded `GET` p99 cleared the frozen ±3% A/A band with a 95%
interval of -2.15% to +0.50%. Both cells passed all four controls without run-queue flags.

The full ten-cell A/B matrix ran on the same split-core profile. Independent verification checked
540 receipts against the B74 and `89ec8970c` build records. All ten A/A controls passed and nine
cells cleared all four A/B bounds. Raw embedded `GET` p99 passed with a 95% interval of -2.30%
to +0.12%. The remaining raw embedded `PUT` intervals crossed the goodput and CPU/op limits.
Goodput was -1.90% with an interval of -3.06% to -1.37% against a -2% floor. CPU/op was +1.93%
with an interval of +1.39% to +3.19% against a +3% ceiling. Raw `PUT` p99 and allocation passed.

The verified archive is `target/imap-075/d1-put-clock-20261125/d1-isolated-recalibration-20261126.zip`
with SHA-256 `71a49123a4eb37584a5202dfcb40b9fc64a490f53ead2d1f4ee79e86140c1812`.

## Open admission gates

The owner explicitly chose to leave W0 public scope open on 25 November 2026. OD-6 and OD-9
are not frozen by this extraction; the remaining numerical and delivery decisions also stay open.

D1 remains open until matched measurements against the exact B74 source admit a dispatch
candidate for embedded/local, direct-native and RESP GET/SET at pipeline 1 and the registered
pipelined depth. The [B74 intake](d1-b74-intake.md) authenticates the predecessor; it does not
replace those measurements. No throughput, latency or CPU advantage is claimed here. D2-D16
and production distributed-map qualification remain separate work.
