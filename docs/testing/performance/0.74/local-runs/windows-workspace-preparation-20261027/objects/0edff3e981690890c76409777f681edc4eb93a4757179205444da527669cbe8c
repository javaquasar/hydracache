# Windows/workspace preparation and read-only storage attribution

These tools do not run Cargo, a package/workspace gate, WSL, a VM, a service,
the original-owner path or a server. They do not stop foreign processes, change
settings, resize pagefiles, compact disks or delete build/evidence files.

`storage.py <new-output-directory>` records Windows volume, pagefile/CIM memory
and registered distro disk observations before and after metadata-only scans of
074 `target` and the shared Git object directory. It does not follow reparse
entries. Raw PowerShell streams are opened before CIM probes. Sharing violations
are retained, not bypassed. File logical size, allocated VHDX bytes, CIM MiB,
physical-memory KiB and process private commit/working set remain distinct.
Creation/modification totals are **not** measured allocation deltas. Missing
historical per-file baselines prevent an exact retrospective physical balance.

`prepare.py <new-output-directory>` writes intent/contract before preflight,
checks the fixed complete package source/syntax closure as data, uses the original
Windows environment checks, records manifests for all workspace members and
retains the current source status. It does not import archived code. An output
directory is exclusive even after failure. There is deliberately no `--execute`,
retry, filter or deadline override. Source review is not compiler expansion or
Windows/workspace execution admission.

The distinct package remains 780 seconds **including compilation**, four case
threads, physical 4 GiB sampled floor, all default targets and no automatic
retry. The first V9 timeout and every earlier refusal remain unchanged. GNU's
156-target pass does not replace Windows. A sampled floor is not a build or
pagefile growth reservation; the preparation always reports `launch_ready=false`.

Workspace `cargo xtask verify` remains a separate complete gate, with an actual
default-toolchain 1.94.0 precondition rather than a silent toolchain switch. Its exact
current `verify.rs` body is pinned, including the full Clippy/allocator matrix,
dependency policy, simulation/fault budgets, docs, feature/contract checks,
Windows split tests, original Node/npm absence rule and per-process isolated
xtask test target. The package's 780 seconds is not invented as a whole-verify
deadline. The previous actual milestone receipt had a 4,800-second outer bound,
16 GiB initial free-space prerequisite, 4 GiB running floor and five-second
capacity polling. Those obligations are retained, not derived from the package
budget. The old coordinator's expired October 24 absolute schedule is not reused.
An owned outer execution envelope and sufficient resources must be reviewed
before running it; this tool neither implements nor waives them.

```text
python -B -m unittest discover -s tools/windows-workspace-preflight-074 -p test_prepare.py -v
python -B tools/windows-workspace-preflight-074/prepare.py target/windows-workspace-preflight-20261027/prepare-first
```

All live predicates, frequency/order and 300/60/10 origins/windows stay exact.
These are local diagnostics and preparation, not a performance improvement,
full workspace/security pass, product-memory claim or release/server admission.
