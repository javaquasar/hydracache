# Offline ReservedLive candidate reconciliation

Verify eleven sealed input packets, reconstruct the old refused ReservedLive
facts, and join five retained allocation candidates to nineteen current source
pins at `1e5f31d9d443bf6c9b1d901781986501425bf9e9`. Eight other owners remain
unresolved. No old reviewer or evidence is changed to accept new source.

```powershell
python -B tools/offline-reserved-reconciliation-074/test_reconcile.py
python -B tools/offline-reserved-reconciliation-074/reconcile.py --output docs/testing/performance/0.74/local-runs/reserved-reconciliation-20261022/facts.json --verify
```

`--packet-sha256` additionally checks the final packet's raw inventory and exact
membership against the supplied digest; it requires `--verify`. First publication
uses exclusive creation of facts.json in the separate pre-created output
directory. It refuses replacement, linked outputs and publication into old packets.
Verification is read-only. `check.ps1` retains its first Windows-only checks in
a new directory, refusing an existing one.

Metrics stay within each sealed corpus and source pair. Requested bytes are not
RSS/retention, nested old wall is not current timing, and old call counts cannot
turn allocation differences into admission headroom. There is no combined
allocation total, speedup, new profile, runtime change or readiness promotion.
The imported tools are compared with their sealed method snapshots before use;
none of their process/profile entry points is called.
