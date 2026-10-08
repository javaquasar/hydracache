# Rental pilot preparation: inspection is not execution

The preparation implementation is `f0030b50edb9886992dc232d33f7146055fd3d33`.
This is local preparation under the read-only/preparation authorization, not a
running coordinator or a new product hypothesis. No SSH, service operation,
remote build/upload, product fixture, numerical pilot or qualification ran in
this step. The old P0 inputs, floors and qualification manifest remain unchanged.

## Implemented boundary

`scripts/perf/performance_rental_pilot_prepare_074.py` exposes only `plan` and
`verify-build`. For example, from the repository root:

```text
python scripts/perf/performance_rental_pilot_prepare_074.py plan .
python -m unittest scripts/perf/test_performance_rental_pilot_prepare_074.py -v
```

`plan` starts no subprocess. It validates every field of all four strict input
objects, including the original schedule, integer types, surface/depth/order,
corpus and typed workload digests. Unknown/duplicate keys, non-finite JSON and
oversized inputs fail. The output binds raw input and contract hashes and
prints a fixed Linux release build command **as data**, never executing it.
Execution, reservation, admission and promotion remain false.

`verify-build SOURCE_ROOT SEAL BUILD_LOG` inspects an existing artifact on local
Linux only. It requires the exact clean observer source `62114be0`, source tree,
both lockfiles, empty features, no counting allocator, release/target identity,
ELF64 little-endian x86_64 executable mode, bounded binary size/hash, raw build-log
hash and matching Rust/Cargo 1.94.0 identity. Git source/cleanliness is checked
before and after inspection. Seal fields are exact; linked files and symlinked
paths are refused. This does **not** prove that the declared command compiled
those bytes, that lock inputs are compiled into the executable, or that a later
execution will use unchanged bytes. A real Linux release build, trusted build
provenance and execution-time identity checks remain outstanding.

Only fixed Git/version metadata commands are allowed. Git fsmonitor is disabled;
stdin is closed and the child environment is restricted. Metadata has a ten-second
deadline, 64-KiB combined-output bound and one-second leader cleanup deadline.
Output polling can overshoot between checks. These limits are **not** arbitrary
workload-tree termination proof, cgroup ownership, controller-loss recovery or
retained partial workload receipts. The future workload contract still requires
an owned cgroup/child tree, 60-second per-process and 300-second total limits,
16-MiB receipts, retained failures and no retries. That coordinator is not built.

## Reservation blocker found in source

`HostExecutionClaim::recover_active` in
`tools/long-run-supervisor-074/src/host_execution.rs` takes the existing
`.host-execution.lock` before reading the active marker and returns Busy when
another process owns it. The three maintenance methods in `server.rs` propagate
that error; `serve` propagates their errors out of its loop. The installed service
definition uses `Restart=on-failure`.

The same affected paths were inspected via local `git show` at the last observed
installed source `543108f1ccd206ae670803c2617f07e7fa91ae62`. Therefore an external
pilot holding this lock can plausibly terminate/restart the supervisor. This is
a **source-based inference**, not an induced host failure or an observation of
current host health. The inventory's absent markers do not make that operation
safe. No live lock probe was attempted.

External flock, forged active markers and a private lock that existing campaigns
ignore are prohibited by `rental-pilot-coordinator-contract.toml`. Merely swallowing
Busy would not establish mutual exclusion, an authenticated diagnostic lifecycle
or crash-safe cleanup. We do not change that behavior, restart/pause the service
or bypass the existing reservation protocol under preparation-only authorization.

The next decision needs explicit control-plane/service scope: a reviewed,
supervisor-owned diagnostic lease with a fixed allowlisted preset, rejection of
active campaign/fixture contexts, cgroup/deadline ownership and controller-loss
cleanup. This must not become a generic remote shell API or change frozen product
qualification. A controlled maintenance pause is a different, separately
authorized option. Neither is implemented or authorized by this document.

## Evidence and limits

The packet `local-runs/rental-prepare-f0030b50/manifest.json` binds a clean source
before/after capture, all three raw logs/plan outputs and the preparation source
hash. Eleven Python checks passed on Windows and eleven on local WSL/Ubuntu.
The Linux log was captured through PowerShell Tee; raw retained bytes are sealed,
but this is not native Linux stdout-byte provenance. The positive binary-inspection
test uses a **fake ELF, fake build log and mocked Git/toolchain**; no real product
Linux binary was built or inspected. Windows mocks POSIX executable bits only
for this fake fixture; Linux exercises the fixture's actual executable mode.

Tests cover complete input drift, malformed JSON, seal/source/cleanliness drift,
wrong ELF/architecture, corruption, hardlinks, allowlist, metadata failure,
deadline/output overflow cleanup and the unsafe reservation prerequisite. Two
development test failures were corrected before sealing: a misplaced assertion
referenced an out-of-scope mock (`UnboundLocalError`); a mock-Linux test initially
assumed NT chmod supplies POSIX execute bits. Neither was a product workload or
a numerical attempt; no floor or P0 workload changed.

The first strict clippy pass also rejected `filter(...).next_back()` in the new
receipt guard. It was replaced by equivalent `rfind(...)` before the final gate;
no lint exception was introduced.

The root guard `rental_pilot_preparation_cannot_bypass_supervisor_or_authorize_execution`
verifies the packet hashes, all 22 successful preparation checks, config hashes,
closed execution flags and incomplete workload-tree deadline contract. It is
enrolled in the release-evidence registry. This is not full D3, native nonregression,
System allocator active/resident/retained proof, A/A calibration or C74 admission.

Final focused validation passed: 66 `performance_contract_074` tests, 13
`release_evidence` tests, xtask all-target check, strict all-feature clippy,
formatting, doc-check, local non-promotable performance-contract-check, 17
release-governance checks, documentation links and mdbook build. The ship variant
is intentionally red for unresolved C74/qualification; no full workspace/release
verification or performance measurement is claimed by these preparation gates.
