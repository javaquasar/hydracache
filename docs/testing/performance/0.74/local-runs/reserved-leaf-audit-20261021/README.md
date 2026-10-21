# Offline ReservedLive leaf audit evidence

This is a separate read-only audit of the retained first refusals and the runtime
tree at `e59190ffa431f36d9fc37998e60b368bcc2a1623`. No new profile is collected.
The raw seals and all members of four earlier packets are checked directly;
the old full round facts are reconstructed before narrowing to ReservedLive.

`facts.json` keeps the ReservedLive-only kernel and overlapping live views,
seventeen canonical source hashes, thirteen leaf witnesses and review
dispositions. Review order is scope/risk, not an exclusive elapsed-cost ranking.
The first publication command refused because this new packet directory did
not exist; it wrote no facts and invoked no runtime. Creating this README then
permits exclusive creation of the derived file. This is not a repeated live run.

`checks/` retains pure Python controls, source-canary checks, affected xtask
check/lint and documentation/registry/local-contract logs. `method/` holds the
auditor and test sources. `inventory.json` seals every member except itself.
No prior member, refusal, seal, window, observer or runtime is replaced.

Recompute without writing any facts:

```powershell
python -B tools/offline-reserved-leaf-audit-074/audit.py --output docs/testing/performance/0.74/local-runs/reserved-leaf-audit-20261021/facts.json --verify
pwsh -NoProfile -File tools/offline-reserved-leaf-audit-074/review.ps1 -Mode Review
```

No allocator, CPU, syscall, memory or post-candidate speedup is measured by this
audit. Full material, cleanup/EOF/fence, delivery, all writers, downstream fault
and server gates remain closed. Original windows remain 300/60/10 seconds.
