# Owner and property calls: local protocol controls

This continues the [runtime/caller shadow](dbus-runtime-bridge-result-20261028.md).
The local graph and selected bridge are unchanged. The full original manager_owner
function is now compiled and executed against a scripted in-process peer, and
the existing property wrapper gets real protocol-call coverage. No root caller,
dependency, policy, qualification identity or live frame changes. No real bus/
manager, server, services, clocks, installation, product workload or performance
experiment is used. Original checks, order, time origins and limits remain.

## Original source and exact read ordering

Preparation verifies the previous packet, its frozen source receipt, every
fixture/overlay member and protected root input. Original diagnostic_manager.rs
remains pinned to SHA-256
`7950deead36bcc6514f1c2f73be3cbce126a6e7a3bff3d1a55d11eac0be674db`.
Copy manager_owner as a whole, with its original DESTINATION. Its three calls,
error mappings and final owner/UID/PID predicate are unchanged. The preceding
sender/type/error helpers and final bridge remain byte-identical to prior S.
Only one fixture Rust member gains that function/constant and a test module;
all manifests, source packages and lock are unchanged.

Seven new test functions contain fifteen scripted scenarios per compiler image:

| Scenario | Required result / observed call scope |
| --- | --- |
| Valid synthetic owner | GetNameOwner → GetConnectionUnixUser → GetConnectionUnixProcessID; return :1.1. |
| Non-unique owner, nonzero UID, PID other than 1 | All three original reads happen before foreign-manager refusal. |
| Wrong sender at each of the three reads | Original owner/user/pid-unavailable result; no later read. |
| NameHasNoOwner or wrong UID/PID body type | Original error mapping; no later read. |
| String/u32 properties and repeated ActiveState | Three actual Properties.Get calls; active → failed and MainPID=74, with no cache. |
| Wrong Variant type, non-Variant body, UnknownProperty | Property getter returns error after the scripted request. |
| Property request received but unanswered | Original 250-ms method timeout returns error before explicit peer release. |

The peer asserts destination, interface, path, member and decoded arguments on
every message, then compares the complete trace with the script. Each image
receives 31 requests in those scenarios. Error-path truncation is explicit:
changing a predicate or issuing a later read cannot produce the expected original
result/trace. Bad identity deliberately preserves the old three-read order;
this work does not move validation earlier to reduce observation count.

## What those identities mean

Channel/authenticated_socket is an in-process test transport without OS
authentication. All names/UIDs/PIDs are synthetic assertions. UID 0/PID 1 do not
establish host trust. Native process credentials, manager channel ownership,
policy/epoch/boot, original ReplyState, FD exchange and writer termination remain
outside these tests. The full inspect_in_worker does not run, and its before/
after manager-owner comparison across unit reads is not yet qualified here.

Property cases use the old checked helper's CacheProperties::No. They exercise
the bridge's actual get_property calls and String/u32 conversion rather than
only compiling a generic wrapper. They do not qualify systemd_unit's default
lazy-cache behavior or extra property types. No new sender guarantee is claimed
for the direct property getter; sender refusal is proved in the selected
original checked_call/manager_owner path. The real bus's routing/authentication
cannot be replaced by a fake peer's chosen header.

## Checks and retention

The first complete local commands pass **21/21 on Rust 1.94 and 21/21 on Rust
1.88**. The old 14 cases remain in the actual catalog alongside the seven new
names. Both all-target/all-feature probe checks and strict Clippy pass; unchanged
standalone cargo-deny passes advisories/bans/licenses/sources. No lint/deny
exception, retry, source correction, corpus reduction or native failure occurs
in this continuation. Previous refusals remain immutable via the predecessor.

Two source/compiler-specific target roots bind actual Cargo compiler-artifact
source/path, executable SHA and complete native list before execution. Native
identities, counts, ignored/filtered totals, exits and postflight all agree.
Every owned Job retires without errors; whole source/HEAD/fixture/overlay/root
guards hold. Original local envelope 1200 seconds, 4 GiB floor, five-second
capacity/retirement and 250-ms method duration remain. Live 300/60/10 and
package/workspace 780/4800 contracts are unchanged. Three Python controls and
owned Rust formatting pass. These are scoped probe gates, not a full product gate.

Source, source projection, scripts, first streams, native lists/accounting and
owned execution receipts are retained. The article and closure plan record the
new local coverage and limits. No numerical performance/non-regression claim,
production all-writer proof or release/server admission follows.

The [primary packet](local-runs/dbus-owner-properties-20261028/index.json)
contains 365 logical references and 329 distinct raw objects. Index SHA-256:
`6dcc7caeafe086771c60a717c27bfe3ab2091177c1866520fcc1edc25b846dfb`.
Inert composition checks recover the exact previous caller projection after
removing the one new function/constant/test route, verify the original owner body,
and compare all fifteen traces byte-for-byte between compilers. Advisory DB
99670cfc0bc574433034b75876a4b791ccaa2efb is observed clean in postflight only,
not sealed per lookup. Native image path/size/SHA receipts remain; binaries and
target trees are excluded. Verification never executes archived source.

## Next work

Port complete read paths in systemd_unit and diagnostic_manager, including
lazy-cache property behavior, before/after owner change, loaded settings and
exact error mapping. Then GNU/MSRV, Unix FD/original reply state, disconnect/
partial I/O and cleanup controls, source/effect/distribution and fresh complete
package/workspace gates precede root adoption. Real original delivery,
autonomous writer retirement and reconnect remain separate open work.
