# Windows/workspace preparation and read-only storage attribution

The original storage/preparation tools do not run Cargo. The separately guarded
`dispatch.py --execute` connects only the fixed local Cargo commands below.
No tool runs WSL, a VM, service, original-owner path or server. They do not stop foreign processes, change
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

## Owned Windows CI mechanism prototype

`owned.py` has no CLI and does not connect the prepared contracts to Cargo.
Its private fixture-tested mechanism loads only the original SHA-pinned CI
Windows Job helper; that helper is unchanged. The worker inherits already
separated streams, without global stdio/environment replacement. A successful
leader with live descendants is not success. Timeout, floor, cancellation and
setup/cleanup failures retire only the held Job/original child handles, with no
PID reopen, `taskkill`, privilege changes or access to foreign processes. The
five-second cleanup budget and capacity polling stay explicit. CI containment
does not establish production all-writer cessation or release authority.

Run `test_*.py` to include the mechanism's models and three tiny native Python
fixtures. Fixtures retain their own temporary stdout/stderr/result even on failure;
their deliberately small synthetic bounds are not gate deadline overrides.
The existing six-case native CI Job cohort separately checks creator loss before
resume and during a live descendant, setup/unsupported-Job refusal and stdio/CWD.
No service, original-owner product path, WSL or server fixture is involved.

The separate fixed dispatcher below requires clean source/environment, the exact
controlled worker/envelope bytes, current capacity and complete target accounting.
The old timeouts are not retried, and prototype controls alone are not admission.

## Fixed Cargo dispatcher

`dispatch.py <absolute-new-output> --mode package|workspace` defaults to preflight
only. `--execute` is the only execution switch; there is no free-form command,
filter, deadline, thread or retry option. The existing preparation-only API and
its old false admission flags remain unchanged.

Execution requires the exact complete green `controls-final` receipt, clean
original branch, fresh source/module and original environment checks, all workspace
manifests, Git HEAD/refs/effective-config digests and actual toolchain/executable
identity. Configuration/environment values are hashed, not published. A complete
matching postflight is required. The original capacity prerequisites remain.

The coordinator writes exclusive separate stdout/stderr before preflight and
passes a SHA-bound request to `worker.py`. Helper bodies are verified before
imports. Startup/frame validation consumes the SAME absolute 780/4800-second
budget, never an added execution window. `_owned` accepts only an absolute
deadline that cannot extend its original bound; its old default behavior stays
unchanged. Retirement remains five seconds. Parent fallback uses only its original
worker handle; lost/missing retirement receipts fail closed, never declare success.

A nonblocking worker-owned Windows file lock prevents concurrent runner-owned
Cargo commands without stopping any external process. A per-input/mode exclusive
claim is retained even after failure/cancellation: another output label is not a
retry mechanism. Output directories and existing receipts are never overwritten.

Package accounting uses the unchanged SHA-pinned strict 156-target parser.
Compiler/runner/resource refusals additionally retain partial counts and unstarted
targets, but can never pass. Workspace accounting lists all 21 original verify
stages, Cargo test/doctest targets and complete/partial case summaries. Original
nightly ignored cases and the original Node/npm absence rule remain visible.
Rustdoc can print multiple groups per Cargo target; those groups are all counted
without inventing a case-to-target mapping. Whole-workspace default discovery
authority is its fixed Cargo argv and pinned manifests, not a fabricated static
feature-resolution catalog. Missing stages/summaries or unknown framing refuse.

The full gate result additionally requires natural successful exit, empty original
Job, successful bounded cleanup and byte-exact source/ref/config/environment
postflight. Parser errors retain raw streams and an explicit unassessed accounting
record. No partial suffix, GNU result or metadata verifier substitutes for a gate.

The [2026-10-28 security successor](../../docs/testing/performance/0.74/security-review-20261028.md)
adds exactly one reviewed zstd-sys version/checksum change. `security_lock.py`
checks the entire current lock and reconstructs the entire previous lock before
the inherited source closure runs with that single new auxiliary digest. Other
catalog entries, Rust syntax inventory and historical receipts stay exact. This
does not establish dependency-policy success or a new historical package pass.
Execution now requires the separate complete security-successor controls receipt
at `target/security-review-20261028/controls`; old green controls cannot authorize
new helper bodies. No deadline, capacity or live-check frequency changes.

The subsequent [Swagger migration](../../docs/testing/performance/0.74/swagger-security-result-20261028.md)
uses `swagger_inputs.py` and SHA-pinned `swagger-inputs.json`. Complete current
lock/manifest bytes reconstruct the complete preceding pair; the preceding sys
inverse still runs. Only those two inherited auxiliary digests change. The
original sys tests now use its immutable packet input, keeping that versioned API
and all its refusal assertions intact. Current-input tests separately check both
whole predecessors, ordering, schema drift, catalog mutation and build overrides.
The original inherited LF normalization applies to both CRLF manifests; no
deadline or live check is replaced. Current execution requires the complete
`target/swagger-security-20261028/runner-controls` receipt for these exact helpers.
Inherited `SWAGGER_UI_*` overrides refuse before Cargo because they are outside
the reviewed registry-vendored asset path. The older receipts stay historical.

The subsequent YAML successor uses `yaml_inputs.py` and SHA-pinned
`yaml-inputs.json` to reconstruct both complete preceding lock/xtask manifest
bodies. Both original Swagger and sys inverse proofs still execute; the root
manifest's Swagger proof also stays mandatory. Only these three named inherited
auxiliary entries change, with all Rust sources, target/module inventories and
live checks retained. Historical Swagger API tests use its immutable packet;
new controls separately validate the current YAML bodies and refusal paths.
Execution requires the distinct complete
`target/yaml-security-20261028/runner-controls` receipt for the current helpers.
