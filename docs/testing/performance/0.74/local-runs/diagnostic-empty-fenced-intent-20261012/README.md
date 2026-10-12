# Signed intent under the original empty reservation fence local evidence

The original signed intent and empty reservation fence now have distinct fixed
and fixture read-only joins. Both guards remain exclusively borrowed under the
caller's retained common lock; a bound failure blocks both original chains.
This is neither durable reservation nor production enrollment or worker start.

Baseline: `90527b85aec704dfcfe3b41f2b4142022b7f976a`.
Preregistered [contract](../../diagnostic-empty-fenced-intent-local-contract.toml)
and [design](../../diagnostic-empty-fenced-intent-local-design.md):
`78a873e17a9b4ffde9dead4212dba0d8386596e0`.
Implementation: `63196d54927901797d50f7d8d977f42d8cd88686`,
tree `fee6ab5b33af81e318fe079faeae7331e4d5ad4a`.
Checks ran locally on 2026-10-12 (client calendar, Europe/Malta), using
nonroot WSL Ubuntu and native Windows tools. Private fixed-dispatch models
use synthetic keys and temporary ancestry/locks; they are not enrollment.

The healthy baseline passes 11 existing empty-fence cases. The MSRV 1.88
public-API probe fails with E0432 before implementation. The initial
implementation compile attempt has a nine-versus-ten lifetime argument error
and a Rust 1.94 diagnostic-renderer internal error; that log is retained as
development failure, not negative-API, semantic or performance evidence.

The corrected focused suite passes 14 cases, and the final source passes the
same 14 in each of three repeats, with logical late-context seed `0x74a`.
Coverage includes every first-error bracket position, canonical invalid
signature, expiry/state/peer/worker drift, retained leaf and campaign contexts,
late context, partial Policy publication, unsafe/advanced original authority,
sticky refusal, healthy drop, origin distinction and independent concurrency.
Intent refusal is published once; fence refusal uses only independently
exact-safe original authority, preserving first error and typed uncertainty.

Linux scoped regression passes 567 cases; one existing real systemd-bus test
is ignored because the local bus is unavailable. Thirty compile-fail doctests
pass. Native Windows passes 196 cases; Linux-gated tests there run zero and are
not counted as Linux coverage. Native root contracts, evidence registry and
governance pass 158 cases. Current Rust 1.94 and MSRV 1.88 checks,
warnings-denied Clippy, format, rustdoc, doc-check and mdBook pass.
This is scoped coverage, not a full workspace verification.

Qualification dry-run still refuses the inherited main-plan digest mismatch;
ship still refuses incomplete candidate identity and release qualification.
All eight frozen qualification inputs, manifest digest and product candidate
remain unchanged. No thresholds, server, ownership, service, installer,
performance workload or qualification dispatch changed.

The [manifest](manifest.json) binds source/tree identities, exact raw log bytes,
commands/exits, test counts, registered W11 checks and closed permissions.
Run `C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-empty-fenced-intent-20261012/verify-packet.py`
from the repository. The [verifier](verify-packet.py) reads only. Historical
empty-fence evidence is checked against its recorded source, not today’s source.

Next preregister no-overwrite state publication as a transition retaining
this same original lock and the exact published object. Partial/crash
uncertainty, authenticated peer/clock and coordinated trusted ownership/binary
rollout remain before output/start/watchdog/owned cleanup and a host pilot.
There are no product measurements or numerical speedup claims in this packet.
