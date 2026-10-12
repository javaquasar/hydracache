# Initial Reserved publication local evidence

The fixed-only library publisher consumes the original empty reservation fence
without releasing its common lock, exclusively borrows original fixed signed
authority/intent and creates only a new initial Reserved state. File/directory
sync and exact original-inode, canonical bytes, inventory and intent checks
precede runtime acknowledgement. No service entry or worker start is enabled.

Baseline: `d7b46bbb44f0ed1231b238fc361594bb5806f662`.
Preregistered [contract](../../diagnostic-reservation-publication-local-contract.toml)
and [design](../../diagnostic-reservation-publication-local-design.md):
`de3827b0526ba5b2ef7cfd90abc18163dc0c1209`.
Implementation: `c5da3217ac8c03f239a5ccc0ad06e04ed41dff40`,
tree `8bba8f806c251059b7ebfff0c173d6c65438bf6b`.
Checks ran locally on 2026-10-12 (client calendar, Europe/Malta), using nonroot
WSL Ubuntu and native Windows. Fixed-dispatch positives use private synthetic
authority and temporary ancestry/locks, not physical root enrollment.

The baseline passes 14 original empty-fence/intent cases. The MSRV 1.88 absent-API
probe fails with E0432. Initial test compilation has an ambiguous stage import
and missing safe Debug implementation; the corrected preliminary suite passes
three cases, then the expanded suite passes 13. Provisional full tests also pass,
but Linux warnings-denied Clippy rejects a large owned error and unused journal
variable. Those logs are retained as development history, not final-source proof.

The final error boxes its private fence/storage container only on failure.
The success path keeps its original value; no descriptor or lock is reopened.
The corrected final source passes all 15 focused cases in each of three repeats
with logical interruption seed `0x74b`. Eight stages cover pre-create, created
empty record, partial/full bytes, file sync, directory sync, document binding
and final acknowledgement. Tests also cover late same-name O_EXCL refusal,
valid later Reserved snapshots, unsafe mode/links/name replacement, bound
signature/state/peer/worker refusal, unsafe/advanced and partially published
authority, sticky original refusal and independent concurrent chains.

Final scoped Linux regression passes 583 cases with one existing real systemd-bus
test ignored because that bus is unavailable. Thirty-four compile-fail doctests
pass. Native Windows passes 196; its Linux-gated zero-test targets are not Linux
coverage. Native root contract/evidence/governance tests pass 159. Current 1.94
and MSRV 1.88 checks, warnings-denied Clippy, format, rustdoc, doc-check and mdBook
pass. This is scoped verification, not a full workspace release gate.

The tests verify real local creation, canonical bytes, sync calls and original
lock/inode retention with logical injected interruptions. They do not prove
machine power-loss behavior or installed-host durability. Created/partial/
unacknowledged state remains until fixture teardown and blocks fresh empty-fence
acquisition; the publisher never deletes it or automatically retries.
An existing complete state is never adopted as a new acknowledgement.

Qualification dry-run still refuses the inherited main-plan digest mismatch;
ship still refuses incomplete candidate identity and release qualification.
All eight frozen qualification inputs and manifest digest remain unchanged.
No server, ownership, installer, service, product workload, thresholds or
qualification dispatch changed. There are no product performance measurements.

The [manifest](manifest.json) binds exact raw captures, commands/exits,
test counts, immutable Git source/tree identities, W11 mappings and closed
permissions. Run `C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-reservation-publication-20261012/verify-packet.py`
from the repository. The [verifier](verify-packet.py) is read-only. Historical
empty-fence/intent evidence is verified against its recorded source, not today.

Next specify exact intent-consumption publication and restart ambiguity,
then authenticate peer/clock and review coordinated trusted ownership/binary
rollout. Original output/start/watchdog/owned cleanup still precedes a host pilot.
