# Offline ReservedLive leaf audit

Read the four sealed packets and seventeen canonical runtime source witnesses
at `e59190ffa431f36d9fc37998e60b368bcc2a1623`. Reconstruct the earlier complete
round facts, then retain kernel and overlapping live views for ReservedLive
alone. Source dispositions are review-risk choices, not leaf timing estimates.

```powershell
python -B tools/offline-reserved-leaf-audit-074/test_audit.py
python -B tools/offline-reserved-leaf-audit-074/audit.py --output docs/testing/performance/0.74/local-runs/reserved-leaf-audit-20261021/facts.json --verify
```

The audit imports the existing pure log readers. It has no process, network,
WSL, host clock or service entry point. Only explicit first publication creates
`facts.json` in its separate packet; it never replaces output or writes inputs.
Verification is read-only. The full local `check.ps1` also runs selected Windows
source canaries, xtask check/lint and structural gates, never a Linux fixture or
profile. Existing check-output directories refuse replacement.

Do not change source pins to accept a runtime candidate without a new audit
identity. No old span count may be multiplied by a new allocation result to
invent post-candidate latency, total savings or a server readiness proof.
