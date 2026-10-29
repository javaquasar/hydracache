# Integration preparation while 0.74 is unfinished

This tranche implements the predecessor-independent work packages below. It inherits R-1 through
R-11 in `docs/RULES.md`. W0 remains open, production remains `disabled_fail_closed`, and the release
registry still records 0.75 as planned. No production wire/durable identity is allocated here.

| Package | Implementation and concrete checks |
| --- | --- |
| 1. Retained-history CI | Both evidence jobs use a complete unfiltered checkout. `imap_ci_075` verifies the policy; its real shallow-clone regression demonstrates missing/recovered retained objects. |
| 2. Process qualification | Existing Linux process-group/address-space controls are exercised remotely. The observed-process API also checks probe refusal/deadline/cleanup on Windows; it retains the same platform limits and no Linux process-count promise. |
| 3. Current dispatch | `hydracache-server/tests/imap_dispatch_characterization_075.rs` runs the real existing HC1 codec and verified dispatch through binary/empty data, tenant/namespace isolation, TTL/persist/CAS, configured roster refusal, future versions and oversized values. |
| 4. Backend conformance boundary | `imap_backend_contract_075` has a closed 40-command catalog, explicit scopes, immutable prepared requests and binding consistency checks. Every operation executes through the existing reference owner. Production construction refuses explicitly; no runtime dependency on the testkit is allowed. |
| 5. Existing storage | `imap_storage_probe_075` performs real v1 flush/reopen, budget refusal, tombstone/stale-merge, checksum/truncation, future-format and invalid-path probes. Its result separately lists missing IMap atomic-record guarantees. |
| 6. Daemon observer | `imap_daemon_probe_075` observes an actual current local daemon over HTTP/RESP with managed process cleanup, bounded logs/responses and pre/post binary hashes. Eleven ordered request/response observations cover readiness, auth refusal, cross-surface binary/empty values, isolation, zero-deadline refusal, preservation, metrics and drain. Existing `hc2_daemon_process` supplies real HC1/HC2 mTLS coexistence and startup-refusal tests. |
| 7. SDK development seam | Rust and Java development ports preserve owned payloads, identities, tokens and cancellation/replay semantics. A standalone Rust workspace and a separate Maven consumer compile/use these APIs. The published SDK remains a distinct narrower surface. |
| 8. Listener/security/operations integration | The machine contract names required production epoch/ACK, listener gap/resync, authenticated member, readiness, drain/reclamation and lane/fairness observations. Dependency-graph tests prevent direct/indirect normal/build edges into testkit/xtask. Native listener loss and roster spoofing have real boundary tests. Existing clustered reference mechanisms are reused. |
| 9. Real Hazelcast preparation | A separate pinned 5.5.0 official-client adapter executes ten native operation families and consumes actual native events. Tests compare eternal sequential outcomes/events/digests to the independent oracle, exercise native expiry/eviction/gaps/overflow and startup cleanup. The packaged probe retains raw diagnostic GET samples and JVM heap/nonheap/thread/GC observations. |
| 10. Artifacts and replay | Two Java JARs use a fixed output timestamp and are rebuilt/tested from source in a second clean directory, then compared byte-for-byte. The external Rust lock may use only identities/checksums from the source lock. A separate preparation receipt binds source, tooling/runtime, inputs, reproduction and raw observations; verification reexecutes trusted probes and package reproduction. |

The Hazelcast diagnostic profile is a single owned member/client JVM, BINARY storage, no backups,
no TLS, exclusive sequential access and eternal values. It has no retained mutation identity retry
guarantee. It cannot establish S1/S2 equivalence, a concurrent history, a product latency comparison
or a production resource limit. Native finite TTL/logical-time scenarios are explicitly outside
the semantic comparison profile. Raw diagnostic timing and GC/heap samples are not compared as
deterministic correctness observations and cannot make a performance claim pass.

Current HC1 dispatch evaluates its existing envelope deadline against logical origin zero. The
characterization records positive/zero behavior explicitly; the daemon probe claims only the
zero-deadline refusal. A general current-wall-clock deadline claim is not introduced. Durable v1
does not atomically store IMap TTL, incarnation/generation and retained invocation outcomes.

## Commands and step gates

```powershell
cargo test -p hydracache-cluster-testkit --test imap_backend_contract_075 --locked
cargo test -p hydracache-server --test imap_dispatch_characterization_075 --test hc2_daemon_process --locked
cargo test -p xtask --test imap_ci_075 --test imap_pre074_075 --test imap_daemon_probe_075 --test imap_storage_probe_075 --test imap_proof_process_075 --locked
cargo xtask imap-pre074-tools --release 0.75
cargo xtask imap-daemon-probe --release 0.75
```

`imap-pre074-tools` builds the current daemon, runs the server suites, compiles/tests a standalone
Rust consumer, installs the provisional Java contract JAR, tests/packages the separate Hazelcast
consumer, copies its pinned GA JAR/POM, and reproduces both Java packages in a clean second
directory. Build commands inherit the enclosing CI job deadline; each runtime probe separately
uses the managed execution/output/memory/cleanup envelope. This controller is preparation tooling,
not a final-candidate 6-hour/24-hour soak controller.

With exact clean source, execute and strictly replay the preparation receipt:

```powershell
cargo xtask imap-pre074-evidence --release 0.75 --output target/imap-075/preparation-new
cargo xtask imap-pre074-evidence-check --release 0.75 --receipt target/imap-075/preparation-new/receipt.json
```

The producer executes its build/test steps before retaining observations. Receipts do not select
commands, binaries or classpaths. Replay checks fixed input bindings, raw request/response
semantics, native Java results/events, external Rust output, actual disk behavior, workspace
dependency metadata and package reproduction. Source/build writers must remain stable; this is
not sealing or attestation against hostile replacement of a trusted compiler or tool. Current
trusted artifacts and the bound runtime are needed for replay. Workspace package metadata and
the pinned dependency inventory prepare provenance; they are not a complete release SBOM.

Every implementation step uses focused tests plus affected-package `cargo check` and strict
Clippy. The common IMap gate includes the Rust command boundary, and the local CI matrix includes
the preparation producer/replay plus Java 17/21 and Rust 1.88/1.94 execution. No skipped test or
reference/model receipt can satisfy a missing production verifier.

## Handoff after 0.74 publication

The retained blockers are W0 closure, production backend extraction/owner routing, final wire and
durable identities, production SDK generation, accepted 0.74 non-regression evidence, and actual
distributed S2/release admission. The next product changes follow W10's D1-D16 gates, using these
characterization guards and consumers. A qualified accepted zero-improvement 0.74 baseline is
valid; a positive throughput improvement is not a predecessor requirement.
