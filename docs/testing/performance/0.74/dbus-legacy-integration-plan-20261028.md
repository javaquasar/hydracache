# Join the remaining production D-Bus helpers

Continuation of the approved security backport/caller migration, without a new
performance hypothesis. The preceding full Message graph used zbus 6 for fifteen
original callers but left diagnostic_manager.rs, diagnostic_loaded.rs and
systemd_unit.rs on zbus 5. Passing isolated helper probes did not join these
production callers or remove the old production edge.

The new exclusive source copy pins each complete original file and reuses the
qualified bridge_v2.rs. It changes import/API spelling and selects the preserved
multi-threaded blocking bridge; it retains all old caller functions/tests,
manager read/error/sender order, cache selection, connection method timeout,
Operation origins, all original frame/kernel predicates and 300/60/10 windows.
No unreviewed per-call runtime, dummy effect, lint exemption or hidden exclusion.

The old zbus production requirement moves to an explicit Linux dev-dependency
because the existing ignored manual negotiated transport/credential adapter still
requires it. All complete lock bytes stay exact. This intermediate step must not
be called complete security remediation: the old branch remains in the test/lock
graph. Porting that real backend, source/distribution review and eventual root
adoption remain independent gates. The root product dependency/source graph is
not changed by this temporary integration candidate.

The first inert preparation is retained. Source review found the inherited outer
owner's writer binding was absent before dispatch; a distinct owner-bound context
adds that original binding and keeps the same existing CI owner. No command was
executed against the earlier preparation. New fixed contexts are
target/dbus-legacy-integration-20261028/owner-bound and the native 20261028-a2.

Three inert controls require all original tests and operation/timeout markers,
whole-body drift refusal, the qualified v2 wrapper and multi-threaded feature.
The actual next command is library check and strict all-feature Clippy on GNU
Rust 1.94/1.88 under the exact original Windows Job/Linux pidfd-subreeper envelope:
1200/1100-second origins, 4 GiB floors, five-second polling/retirement. Preserve
the first result of every command and never extend deadlines or retry unchanged
input. Compilation does not execute a manager, bus, service, clock, server or
product workload. Full unit execution and dependency-policy adoption follow
separately after actual source compatibility, not from this plan alone.
