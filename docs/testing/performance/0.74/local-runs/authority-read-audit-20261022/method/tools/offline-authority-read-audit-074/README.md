# Fresh authority read ownership audit

Read-only reconstruction of sixteen sealed packets and the current complete
authority read sources. This tool neither loads a live authority registry nor
runs a kernel, File/Read benchmark, test image, WSL, clock or service command.
Its only write mode exclusively publishes a separate derived facts file.

Retained Leaf.raw and the original File/stamp remain owned evidence. Fresh
revalidation independently opens and reads the named object and keeps that new
File alive through both comparisons. Only its transient byte storage is a next
hypothesis, pending a separate reference baseline for bounded read/EOF/error/
ownership behavior. This audit supplies no measured request or time savings.

Read-only commands after publication:

```powershell
python -B tools/offline-authority-read-audit-074/audit_read.py --output docs/testing/performance/0.74/local-runs/authority-read-audit-20261022/facts.json --verify
python -B tools/offline-authority-read-audit-074/test_audit_read.py
```

The fixed first check recipe also runs the previous offline readers and Windows
source canaries, xtask check/strict clippy/format and local structural gates.
It cannot replace its first output directory. Source mutations are not new
kernel fault evidence; whole-file pins and complete expression order are checked
in addition to named witnesses. All original deadlines/refusals and closed
material/fault/reconnect/server gates remain.
