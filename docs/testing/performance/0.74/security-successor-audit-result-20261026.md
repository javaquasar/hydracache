# Security successor compatibility and package coverage audit

The approved rustls, WebPKI and Tokio updates pass a strict, read-only comparison
against the reviewed V6 dependency inputs. Historical preservation guards have
not yet been adapted: the first exact OIDC lock guard fails, and the old V6
preflight refuses source drift before dispatch. Neither result is relabeled as
a package pass or retried unchanged.

## Exact dependency scope

The [active audit and mutation controls](../../../../tools/security-successor-audit-074/README.md)
pin six complete current bodies independently of historical comparison code.
Only CRLF-to-LF conversion is permitted. The complete lock graph differs from
`fa9e708868b22999d60c9ccd97b694662910a095` in these nodes:

| Locked node | Reviewed V6 | Approved current |
| --- | --- | --- |
| rustls | 0.23.45 | 0.23.46 |
| rustls-webpki | 0.103.15 | 0.103.16 |
| tokio | 1.52.3 | 1.53.3 |

The two declared floors change from rustls 0.23.41 to 0.23.46 and Tokio 1.52.3
to 1.53.3. Package membership, dependency edges, sources, other checksums,
features, MSRV 1.88 and deny policy are unchanged. The manager channel is pinned
to the separately reviewed owned-registration implementation in `2a4a771d`;
this audit does not replace its paired GNU/MSRV ownership evidence.
The four new roots remain unapproved and unapplied. The current graph's nine
findings belong to the preceding pinned-database security result, not a new
advisory scan or a green policy gate.

## First preservation failures

The exact unmodified
`diagnostic_oidc_keeps_configured_origin_separate_from_enrollment_replay_and_start`
case fails at `performance_contract_074.rs:1338`. Its assertion permits only two
old supervisor crypto edges relative to `35210757`; accepted security versions
are outside that old expectation. One selected case fails, with 320 filtered
out. The filter isolates this diagnostic; it does not substitute for the full
package or remove cases from the eventual package command.

The old V6 dispatcher is invoked only in its default preflight mode, without
`--execute`. It refuses `source bytes/path drift` and records
`package_dispatched:false`. Its original full attempt remains 72/145 completed
targets, 448 passed cases and an incomplete case at the original 780-second guard.
The old source-successor rules, preservation assertions, dispatchers and their
pins are unchanged. A narrow historical security-input compatibility bridge is
still required; blindly refreshing hashes would discard the preserved contract.

## Fresh source inventory and remaining effect review

The existing syntax-only child parses fresh pinned files with the reviewed
contextual module resolver. One round closes the module graph over 369 Rust
files and 25,910 syntax sites. No input test body runs, and this is not compiler
or macro expansion. Comparison with V6 identifies the exact default scope:

| Input category | Difference requiring review |
| --- | --- |
| Rust source | `direct_output_cost_074.rs` changed; `git_descendant_cpu_074.rs` added |
| Old Rust rows | 367 unchanged; none removed |
| Default targets | 145 to 146; only `git_descendant_cpu_074` added |
| Existing 45 auxiliary inputs | Cargo.toml, Cargo.lock and `run-with-heartbeat.py` changed |
| Explicit heartbeat companion | `watchdog_linux_group.py` is absent from V6's auxiliary catalog |

The source delta contains a synthetic pipe child's explicit flush/exit and exact
byte/status mutation oracle. The new accounting child accepts only its exact
diagnostic argv; ordinary default invocations return before Git execution. The
heartbeat successor imports a Linux subreaper companion. Its previously tested
CI containment is not production all-writer authority, and the new import must
not inherit approval from a catalog that omitted it. Both explicit heartbeat
companions are now pinned in the audit, without claiming transitive effect
clearance. Current complete source/effect review remains open.

## Controls and preserved setup errors

Twenty-two Python tests pass. They reject stale versions, complete-body changes,
wrong paths/checksums, other graph edges or nodes, root-catalog changes, altered
MSRV/features, duplicate JSON/source/target entries, stale syntax, changed
heartbeat imports/companions and extra execute/filter/timeout CLI options.
Successful observations are not cached. Missing catalog rows are reported, not
turned into clearance. The sealed V6 membership remains 368/45/145.

The initial capture coordinator collided with its outer receipt directory and
failed before executing the parser. The initial audit was then given that outer
failure receipt rather than a syntax document and raised KeyError. Both original
programs and outcomes are retained. A separate payload directory fixes the setup
collision; explicit syntax-envelope validation now rejects the wrong document.
The corrected parser executes once. These changed-setup operations are not a
retry of the old lock case, V6 workload or a performance measurement.

## Evidence and next gate

The [append-only packet](local-runs/security-successor-audit-20261026/index.json)
retains raw first refusals, parser output and build identity, current audit and
controls, exact inputs and both initial coordinators. Its 219-member index is
SHA256 `af9931a32cb6e60a0cd4390845056b8df5eea027fb6fa1ef9d8834573389835c`.
The audit implementation is committed as `49e934356`.
Python compilation/indentation, xtask formatting, doc-check and the local,
non-promotable 0.74 performance-contract pass. Six independent offline packet
controls pass. The first verifier looked for a captured libtest panic on stderr;
libtest wrote it on stdout. The corrected stream oracle leaves every raw byte
and first outcome unchanged. Both verifier versions and first failures are
retained in the [53-member verification receipt](local-runs/security-successor-audit-20261026/verification/index.json),
SHA256 `898b7e76175199294c744a18ef33ff77f838e029584c200e8e4aceff8aff86b7`.
Run the packet's `verify.py` and `test_verify.py` offline with Python 3.11 or
later; they perform no Git or child-process operations. Test-only re-pinning
exercises semantic rejection, not approval of a modified evidence packet.
The first staged Git audit checks 136 packet blobs against their physical bytes
with zero mismatches; its receipt is retained separately. Explicit -text
attributes preserve archived log and source bytes.

All runtime predicates,
their observation frequency and original 300/60/10 origins/windows remain.
The qualification manifest stays SHA256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

Next implement and fault-check only the exact dependency/source compatibility
bridge, then complete current transitive effect review before any new full
dispatch. Preserve the old first failures and the unchanged 780-second/four-thread
package contract. This audit changes no cache/runtime behavior, performs no
server/service/clock operation or qualification, and admits neither a performance
claim nor a release. Original worker release, all-writer cessation, durable
completion/cells 2–4, crash/lost-ACK/reconnect, trust and host admission stay open.
