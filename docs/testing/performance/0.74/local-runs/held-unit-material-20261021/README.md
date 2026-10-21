# Held unit materialization local checks

The [implementation report](../../held-unit-material-20261021.md) covers
same-owner materialization and extended inventory, based on `c9cc5a2e`.
Construction takes no substitute descriptor, path, credential, deadline or
payload; it retains the consumed projected holder on success and refusal.
No transport or FD remapping is enabled.

The root cohort records the initial local tests and checks. Review found that
its new final census followed the last fresh absolute-bound check. Despite
passing tests, that candidate was not accepted as complete materialization.
The final candidate adds a fresh original-owner/deadline check after all twenty
FD stamps, plus a mandatory source canary. `final/` retains a separate complete
cohort for that source. Nothing overwrites the initial logs, and no unchanged
workload is retried to seek a pass. The reviewer reconstructs the initial source
by reversing only those two changes; every other source hash must match.

Each compiler lane selects twelve new material controls, ten projection, ten
consuming-holder, twelve reply state/flag/exchange and thirty-eight synthetic
held-build controls: 82 tests per compiler. The final host cohort has thirteen
source canaries (the initial one has twelve), including the 462-file historical
reconstruction. All-target check and strict all-feature clippy cover affected
packages; the host adds format, docs, registry, local non-promotable performance
contract and diff checks.

Kernel positives use real anonymous pipe/socket/memfd/pidfd objects with
synthetic worker/stream/guardian expectations. Bracket/failure cases are models.
The actual original owner API is compile-checked but no new privileged or whole
original-owner fixture is invoked. These are not a live timing pass, all-writer
proof, full SIGKILL/lost-ACK or server admission. Packet injection occurs only
in isolated kernel tests; production release queues remain empty.

`checks.ps1 -Lane 194|188|host -Cohort initial|final` captures commands, exits,
test logs and hashes, with before/after manifests of supervisor/xtask source
and tests, Cargo.lock, COMPAT, registry and qualification. It refuses overwrite
and source drift within a cohort. Captures require their corresponding source;
review never executes an old cohort against a later candidate.

`review.ps1 -Mode Review` is read-only. It verifies both cohorts, the exact source
transition, final source, unchanged qualification, the preceding immutable
forty-file projection packet, report bindings and every packet byte. It neither
rewrites nor bypasses earlier source pins.

No clock, service, host policy, supervisor, installation, server or product
workload changes. Bootstrap grammar and original deadlines stay intact. No
performance claim is measured. Next validate whole original-owner preparation
in its original 300/60/10-second windows, preserving its first result.
