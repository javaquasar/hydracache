# Local runtime progress and blocking-caller shadow

This follows the [actual MSRV source backport](dbus-msrv-backport-result-20261028.md).
The selected local bridge preserves background progress and original caller
predicates in an in-process synthetic channel. **It is not integrated into the
supervisor.** Root manifests/lock, deny policy, qualification identity and all live
checks/order/frequency, time origins and original deadlines remain unchanged.
No real bus/manager, server, services, clocks, installation, product load or new
performance hypothesis is used.

## First baseline, compatibility regression and final source

| Input | Isolated change / first result |
| --- | --- |
| C | Prior F3 source plus common shadow tests and synthetic p2p prerequisites; tokio+proxy. Outside-entry progress contract fails within the original 250-ms method duration; reentry joins the task with 74 before asserting. |
| M | Only the manifest feature becomes tokio-multithread. All 13 tests pass on 1.94. |
| B | Same M implementation plus one constructor-order regression. The selected test fails after endpoint cleanup: invalid names bypass the nested-entry panic. |
| R | Only the bridge implementation changes to enter async Proxy::new through block_on before parsing arguments. All 14 tests pass on 1.88 and 1.94; first strict Clippy on each refuses the same three test-setup async-yields-async diagnostics. |
| S | Same R bridge, test setup obtains the runtime Handle and spawns explicitly instead of yielding JoinHandle from async. Final 14 tests, check and strict Clippy pass on both 1.88 and 1.94; unchanged scoped dependency policy passes. |

The old zbus 5 blocking wrapper uses a process-lifetime static multi-threaded
Tokio runtime. The selected zbus 6 feature restores that type/lifetime instead of
creating a runtime per observation. The C/M fixture diff contains exactly one
manifest feature substitution and byte-identical locks; no caller predicate or
timeout moves. The first negative control is a completed assertion failure with
task cleanup, not a leaked task or a result inferred from wall-time throughput.

The final wrapper is [bridge_v2.rs](../../../../tools/dbus-runtime-bridge-074/bridge_v2.rs).
The first bridge remains separate as rejected review material. Proxy keeps a
connection clone, returns the original reply/MethodError and drops its async
proxy within runtime context. Builder name/path/interface and address validation
remain immediate. The constructor preserves the source-derived old priority:
nested runtime entry panics before invalid argument parsing. The first prototype
got that order wrong; retaining its regression prevents the supported-call happy
path from hiding the difference. No lint allowance or threshold increase is used.

## Exact caller scope and tests

The complete original diagnostic_manager.rs is independently pinned to SHA-256
`7950deead36bcc6514f1c2f73be3cbce126a6e7a3bff3d1a55d11eac0be674db`.
The shadow selects whole unique_owner/proxy/ReplyError/sender_matches/
classify_error/decode_reply/checked_call bodies. Only one blocking Builder path
and three wire-alias spellings change. An inert inverse projection equals the
selected original bodies. This does not project the worker, boot/policy/epoch,
loaded settings, manager-owner observation or original ReplyState capability.

The final 14-case catalog retains the prior five MSRV/wire cases and adds:

- A released task progresses outside every synchronous entry, then joins.
- Nested entry refuses before polling its future; invalid address refuses before build.
- Original proxy helper returns destination/path/interface errors immediately.
- Actual synthetic calls retain success, reject wrong signature/foreign sender,
  and distinguish original-manager NoSuchUnit from a foreign error.
- Proxy and connection clones keep the same underlying connection usable after
  original handles are dropped; explicit synthetic close follows.
- A request reaches the synthetic peer, which deliberately withholds a reply;
  the original method timeout yields the existing Refused result before peer release.
- Nested Proxy::new preserves constructor panic/argument order.

The in-process Channel is upstream's authenticated_socket test transport; it
deliberately bypasses OS authentication. Names and credentials are synthetic
assertions. There is no real manager authorization, syscall/SCM_RIGHTS transfer or
kernel identity proof. Same-message sender/serial semantics do not grant an
original-owner capability. CI Job retirement is not all-writer production proof.

## Checks, graph and evidence boundary

M passes 13 cases, R passes 14 on each compiler, and S passes 14 on each compiler:
69 selected passing native executions, with 28 on final S. Two selected first
native assertion failures and both first R Clippy refusals remain immutable.
Seven actual compiler images have distinct source/compiler-derived target paths;
Cargo artifact source/path/SHA and complete nonempty native catalogs are bound
before selection. Every owned command retains unchanged source/HEAD/fixture/
overlay/root inputs and retires without error under the original 1200-second,
4 GiB and five-second local review contracts. Three Python catalog controls
reject missing/foreign/duplicate cases or a false summary. Owned Rust formatting
passes. Old 300/60/10 live frames and 780/4800 package/workspace limits are exact.

All registry package identities equal the previous compatible MSRV fixture. Only
the probe's explicit existing UUID constraint and UUID's getrandom edge appear
for the shared p2p test transport; no new registry version is selected. C/M and
all successor fixture locks are byte-identical. Cargo-deny uses unchanged root
policy without a new ignore and passes advisories/bans/licenses/sources in the
standalone graph. This does not replay or clear the unchanged product gate.

The source, first failures and complete per-command streams/Job/native receipts
are retained with the inert composition audit. The article and closure plan
record both the accepted local mechanism and its unclosed integration boundaries;
there is no performance improvement or native API non-regression claim.

The [primary packet](local-runs/dbus-runtime-bridge-20261028/index.json) retains
1,566 logical references and 398 distinct raw objects. Index SHA-256:
`59b7f69b3ee958f2a467af97d1cd80ed0d7f35194e33d9fbb58ed001602e1cda`.
Its predecessor binds the exact MSRV archive packet. Advisory DB
99670cfc0bc574433034b75876a4b791ccaa2efb is observed clean in postflight only,
not sealed per lookup. Native binaries/targets are excluded; artifact receipts
remain. Verification reads data and does not execute archived code.

## Next boundary

Exercise actual property reads and manager-owner observations in the synthetic
peer, port the complete systemd_unit/diagnostic_manager call sites, then check
Unix FD ownership/unknown flags, original sender/reply state, partial/disconnect
and cleanup on GNU/MSRV. System connection/address build and property wrappers
currently have compile coverage only. Root adoption still requires source/effect
and distribution reviews and a fresh complete package/workspace gate. Original
delivery, autonomous writer retirement, reconnect and server admission remain
separate open gates. Chitchat's declined preview stays declined.
