# Windows/workspace preparation and post-cleanup storage attribution

The Windows/workspace execution gate is **not dispatched**. Read-only preparation
closes the current package source census and environment checks, but observes
only 5,219,930,112 bytes free. This is below the **existing** whole-verify initial
16 GiB prerequisite. The GNU package remains green; Windows/full-workspace and
release admission remain open. No system setting, clock, service, foreign
process, VM, server, product workload or qualification run is changed.

Tool commit: `3d335a17c31af6c2660014b36a1445414ef2b17b`.
The preparations read original HEAD `d68d2e9743171cda365058bae827807f4d87c28b`
with their own new tools explicitly visible as untracked inputs. They do not
pretend this was a clean execution candidate. Subsequent staged-byte checks bind
the exact five controlled tool bodies and the one added LF text policy to the
tool commit. No Rust implementation, dependency or qualification identity changes.

## Where the space is, and what the observations cannot reconstruct

The [raw packet](local-runs/windows-workspace-preparation-20261027/index.json)
retains UTC-stamped volume/file/CIM readings, separate raw probe streams, complete
metadata scans, first outcomes and exact source versions. No archived code is
executed to verify it. The scans finish without read errors or traversed reparse
entries; totals below are **logical file bytes**, not allocated-volume deltas.

| Scoped current consumer | Bytes | Approximate GiB |
| --- | ---: | ---: |
| `hydracache-074/target`, all named build/diagnostic outputs | 87,424,805,907 | 81.42 |
| Its `debug` subtree, included above | 69,555,509,336 | 64.78 |
| Its `security-patch-msrv188` subtree, included above | 4,919,343,776 | 4.58 |
| Its `tests` subtree, included above | 4,263,708,755 | 3.97 |
| Its old `xtask-verify-39960` subtree, included above | 1,786,532,000 | 1.66 |
| Shared Git objects, a separate tree | 387,454,702 | 0.36 |
| Registered Ubuntu `ext4.vhdx`, allocated bytes after scan | 153,795,690,496 | 143.23 |
| Registered diagnostic `HydraCache074Native` VHDX, allocated bytes | 643,825,664 | 0.60 |

The main Ubuntu disk is under the user's Ubuntu package `LocalState`, **not**
the similarly named small `hydracache-074-wsl-native` directory. Registry-derived
locations and two independent read-only allocation metadata APIs identify them.
Do not add the Linux filesystem's used bytes to its hosting VHDX a second time.

All four manually removed locations remain absent: `t067-rescope` and the
`target/debug` trees in `hydra-moka-release`, `moka-post-removal-observer-pr` and
`moka-upstream-prep`. This task performs no deletion, compaction or cache reset.
The remaining target contains evidence, scripts, bundles, receipts and fixtures;
neither the whole directory nor its diagnostic subtrees are disposable caches.

Since the GNU attempt's 13:09:20 UTC start, currently surviving target files
created in that interval total 863,309,432 bytes (about 0.80 GiB); currently
surviving modified files total 1,486,362,951 bytes. These overlapping metadata
sets are not a net-growth measurement: replacements, deleted files and old
allocation sizes are unknown. They do not establish that Windows build products
consumed the historical roughly 24 GiB loss of headroom.

During the new 14:47:39–14:51:18 UTC observation bracket, free volume bytes fall
from 5,236,404,224 to 5,190,668,288 (43.62 MiB net). Ubuntu VHDX allocation rises
by 33,554,432 bytes (32 MiB); the Docker and diagnostic VHDX allocations stay
exact. This identifies real current disk growth, **not** the cause of the much
earlier large drop. Other allocations and releases can overlap these samples.

System memory pressure is independently visible. CIM reports free physical RAM
of 214,812 then 523,008 **KiB**, and pagefile `AllocatedBaseSize` of 81,987 then
84,066 **MiB**, with usage 26,084 then 26,832 MiB. Pagefile logical length rises
from 85,969,817,600 to 88,149,598,208 bytes. Its true allocated-file metadata
query returns sharing violation **32** through both attempted read-only APIs;
that refusal is retained without elevation, privilege changes or bypass.
Consequently its logical length/CIM counters are not substituted for a physical
allocation delta. The roughly 2 GiB logical increase is plainly not the same
quantity as this bracket's 43.62 MiB net volume change.

The raw process snapshot records Explorer at about 13.3 GB private commit,
WSL around 6.0 GB, and several browser processes around 3–4 GB each. Private
commit is not resident RAM and does not assign pagefile bytes to a process.
This is evidence of competing system memory demand, not a proved memory leak,
product-retention result or authority to terminate another application.

There is no pre-drop per-file allocation baseline. The earlier resource packet
already marks its separate pagefile observations as non-simultaneous manual
transcriptions. An exact retrospective balance or a single causal owner for the
original roughly 24 GiB cannot be proved from these data. The concrete result
is a scoped current inventory, a measured small VHDX increment, sustained memory
pressure, and explicit retention of the missing-baseline boundary.

## Prepared gate, without weakened checks

The [preflight tool](../../../../tools/windows-workspace-preflight-074/README.md)
has no execution, filter, retry or timeout override. It checks the fixed whole
source/syntax closure: **394 Rust bodies, 58 auxiliary inputs, 26,832 sites and
156 default targets**, including complete module-edge closure. Existing Windows
environment checks also pass: no inherited fixture/build overrides, shadow Cargo
configuration, executable/lazy-fetch Git configuration, reparse target/scratch,
or unexpectedly present negative-control executable. All **36** root workspace
member manifests are recorded. This source-only review is not compiler expansion,
a whole-workspace effects proof or Windows execution admission.

The package contract remains exactly 780 seconds including compilation, four
case threads, a physical 4 GiB sampled floor, the complete command and no retries.
The complete old V9 timeout stays red/incomplete; no suffix or GNU pass fills it.

The separate whole-workspace entry remains `cargo xtask verify`, with the actual
default 1.94.0 toolchain required before any future dispatch. Its exact current
`verify.rs` body is pinned. Preserve the complete allocator/Clippy matrix,
dependency policy, fast simulation/fault budgets, feature/contract checks,
rustdoc warnings, original console rule, Windows split test commands and isolated
`target/xtask-verify-<verify-process-id>` for the original two xtask test stages.

The original October 24 milestone's actual receipt records a **4800-second**
outer budget, **16 GiB initial free space**, **4 GiB running floor** and
**five-second capacity polling**. The preparation now retains these independently
of the package's 780 seconds; the expired October 24 wall-clock schedule is not
reused as a new launch schedule. A bounded owned execution envelope still needs
review before dispatch. No original deadline or predicate is enlarged or skipped.

The current observation leaves only 924,962,816 bytes above the running floor,
and fails the original workspace starting prerequisite. A sampled floor does
not reserve future compiler, isolated-target or pagefile growth. Both launch
and release admission therefore remain false.

## Checks and first outcomes

Eleven focused Python controls pass, including the real temporary-file metadata
API, missing-file refusal, source pin/path mutations, manifest-only inventory,
no reparse traversal, exclusive outputs and preservation of preflight refusal.
All four Python modules parse successfully. The existing SHA-pinned ordinary
xtask binary passes doc-check; the tool/source bytes remain exact around these
checks. No Cargo build, full package rerun, Linux live test or server call occurs.

The first control command is retained: nine cases pass and one fixture errors
because its temporary tree lacks an earlier unrelated required pin. The corrected
fixture narrows the pin dictionary to the independently tested body; the production
pin guard is not weakened. Initial and final sources/streams remain separate.
The initial preparation likewise stays distinct: the subsequently recovered
4800-second/16-GiB workspace envelope is not retroactively inserted into its receipt.
An initial `safecrlf` staging refusal is documented as a manual tool-response
transcription; a local LF attribute fixes text preservation without changing Git
settings. Exact staged source bytes are independently verified before commit.

## Next boundary

First recover resource headroom without touching clocks, settings or somebody
else's processes: the owner can save work and close unnecessary browser/IDE
sessions, and separately authorize a scoped cleanup of proven rebuildable files.
Recheck the original 16 GiB starting prerequisite **and** actual build/paging
headroom; reaching a single threshold alone is not a reservation. Do not compact
an active VHDX, delete a pagefile or blanket-delete target evidence.

Then review the owned Windows execution envelope and clean source/environment,
run one distinct full Windows package command under its original contract, and
run the complete workspace milestone. Preserve first outcomes even on timeout or
security failure. This does not promise the known Windows preservation-loop cost
will fit 780 seconds, waive security, freeze C74 or admit original execution,
all-writer retirement, recovery, enrollment or a server run. No new performance
hypothesis is opened.
