# First original owner materialization outcomes

Both first compiler calls refuse on the original Causal time bound during
material revalidation, after production projection and materialization return.
`original_material_gate_passed=false`; no rerun, product, original send or server
admission follows. See the [report](../../held-material-live-20261021.md) and
[preregistered plan](../../held-material-live-plan-20261021.md).

The exact entry source is `1d6d179bd7150ef564560821a897794e20b24b36`.
`metadata.json`, lane before/after records and `postflight.json` are identical.
The raw `.log` and `.exit.json` files retain each first outcome; `summary.json`
is derived without executing anything. Build/image and launch receipts retain
the fresh default-feature source binding. `method/` contains the exact measured
driver bytes; `checks/` preserves both the pre-live PowerShell 5 tooling refusal
and the separate completed PowerShell 7 checks.

From the repository root, run only read-only review:

```powershell
pwsh -NoProfile -File tools/local-profile-session-074/seal_material_live.ps1 -Mode Review
```

Review checks inventory membership and raw hashes, source/driver binding,
original first outcome, phase/step/kernel conservation, boot/origin arithmetic,
stage prefix, missing success footer, checks and scope. It starts no WSL/root
fixture. An absent success footer is retained as incomplete EOF/fence assertions,
not converted into cleanup proof by the empty fixture-residue snapshot.

The `inventory.json` is created once after packet collection and never replaced.
There is no continuation entry point or retry recipe for this packet.
