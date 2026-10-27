# Fixed Windows Job Cargo execution: package green, workspace policy red

The fixed commands are now connected to the original Windows Job and executed
locally. One complete Windows package command passes **156/156 targets and 1,351
cases**, with zero failures/ignored cases. The separate complete workspace entry
stops naturally at **dependency policy**, with **11 advisories**. Neither command
times out, retries, changes a threshold, stops a foreign process or admits a
release/server/product campaign.

| Actual command | Tested source | Result |
| --- | --- | --- |
| Fixed `cargo +1.94.0 test --offline --locked --no-fail-fast -p xtask --lib --tests -- --test-threads=4` | `f62436aa6271793e157272b2a02957576b84c92e` | 156 complete targets, 1,351 passed, 0 failed, 0 ignored |
| Original `cargo xtask verify` | `2fe613bf866f31c1800addb10790d350656b1790` | Exit 1 at dependency policy; 11 prior stages complete, 9 later stages unstarted |

These are separate actual receipts, not a merged exact-SHA C74 qualification.
The second source changes only the workspace accounting of Cargo's exact outer
launch line, its regression tests and the controlled-receipt location. Product
Rust source, package command/parser, dependencies and original verify stages
remain unchanged. The package is not rerun to obtain a favorable result.

## Implemented execution and custody

`tools/windows-workspace-preflight-074/dispatch.py` accepts only package/workspace
modes and defaults to preflight. Explicit `--execute` is required. There is no
free-form command, test filter, thread, timeout or retry option. Every output
directory is exclusive, including on refusal. Per-input/mode claims remain consumed
after an attempted execution; a new output label cannot replay the same inputs.

Before dispatch, the coordinator requires exact complete green controls, a clean
source/configuration scope in the original worktree/branch, the unchanged complete
394-body/58-auxiliary source closure and 156-target catalog, original Windows
environment guards, 36 workspace manifests and actual 1.94.0 toolchain identity.
HEAD, refs, effective Git configuration, selected tracked source, helper bodies,
executable identities and inherited environment are bound in a fresh frame.
Configuration/environment contents are hashed, not published. Source selection
retains the prior wrapper's scope, extends script custody and separately checks
the complete existing package closure; it is not a whole-host or compiler-expansion
proof. All of that frame must match after retirement.

Separate stdout/stderr files exist before preflight. The coordinator sends only
a SHA-bound fixed-mode request to `worker.py`; the worker validates helper bytes
before imports and independently rejoins the original frame. A worker-owned
nonblocking Windows file lock prevents concurrent commands through this runner,
without stopping external builds or changing their settings. The original
SHA-pinned Job helper is unchanged.

The original **780/4800-second** absolute budgets start before worker creation.
Worker startup and validation consume those budgets, never extend them. The
private owned mechanism's new optional absolute deadline cannot exceed its
original bound; existing default calls retain their prior behavior. The package
still has four case threads; workspace keeps its original split `-j 1` commands,
allocator lanes and per-process isolated xtask test target. Initial prerequisites
remain 4 GiB for the package and 16 GiB for workspace, with the original 4 GiB
running floor and five-second capacity polling. These are samples, not reservations.

Only original Job/child/worker handles can be retired. A successful leader with
live descendants is not success. Lost worker/receipt, malformed identity, frame
drift, cleanup error or incomplete accounting cannot turn green. Retirement
retains five seconds. Parent fallback addresses only its original worker handle;
missing retirement evidence is runner loss, not proof of production all-writer
cessation. No PID reopen, `taskkill`, privilege, system setting, clock, service,
WSL, server or product operation is introduced.

## Complete accounting and actual outcomes

Package accounting imports the unchanged SHA-pinned strict parser after checking
its body. It checks the complete scheduled catalog, every declared case/verdict,
summary, compilation framing, zero filters and original limit. Compiler/resource/
runner failures retain explicit partial/unstarted coverage without passing.
The actual package has all 156 scheduled and completed targets, natural exit 0,
empty original Job, no cleanup errors and matching complete postflight. The lowest
sampled physical free space during that owned command is 71,686,627,328 bytes.

Workspace accounting is separate: all **21** ordered source-pinned `verify` stages,
the exact outer Cargo launch, Cargo test/doctest targets, case groups, partial
summaries and unstarted stages are recorded. Original nightly ignored cases and
the original Node/npm availability rule are not weakened. Rustdoc can emit more
than one case group per Cargo target; every group is counted, but an ambiguous
case-to-target assignment is explicitly not invented. Cargo's fixed unfiltered
argv and pinned manifests remain default-discovery authority; no fabricated
static workspace feature-resolution catalog is claimed.

The actual workspace completes doc-check, management-center 0.72, performance
contract 0.73, release-feature leak, formatting, all-feature workspace Clippy,
default workspace Clippy, common/system/mimalloc HydraCache Clippy and dependency
bans. Dependency policy then returns failure naturally. The later DST/soak/raft
budgets, additional Windows allocator lane, workspace tests, isolated xtask tests,
rustdoc, performance-budget contract and management console are **unstarted**.
There are no workspace Rust case summaries yet; zero reported case failures is
not a test pass. The ordinary package's green result does not fill that suffix.

The workspace's original Job is empty, cleanup has no errors and its complete
postflight matches. Its lowest sampled physical free space during the owned command
is 76,304,347,136 bytes. The gate is red because of policy, not resource-floor,
cleanup, framing or timeout failure. No unchanged full command is automatically
replayed, and no detached qualification worktree is modified.

## Current security refusal, not a stale seven-advisory claim

The actual stderr identifies these 11 policy errors:

| Locked dependency | Advisory IDs |
| --- | --- |
| `unsafe-libyaml 0.2.11` | RUSTSEC-2024-0449 |
| `zbus_names 4.3.4`, `zvariant 5.15.0`, `zvariant_derive 5.15.0` | RUSTSEC-2026-0371, RUSTSEC-2026-0370, RUSTSEC-2026-0372 |
| `zip 3.0.0` | RUSTSEC-2026-0367, RUSTSEC-2026-0368, RUSTSEC-2026-0369 |
| `zstd 0.13.3` | RUSTSEC-2026-0391 |
| `zstd-safe 7.2.4` | RUSTSEC-2026-0398, RUSTSEC-2026-0403 |
| `zstd-sys 2.0.16+zstd.1.5.7` | RUSTSEC-2026-0410 |

This is seven vulnerability and four unmaintained errors in the unchanged policy,
not an assertion that every issue is reachable on Windows or in product runtime.
The lock remains SHA256
`e385c45d47050299da7e79a5133d5ec71ca3ea77f6bafbc36744968dff2d28b5`.
Read-only postflight retains clean public advisory DB commit
`99670cfc0bc574433034b75876a4b791ccaa2efb` and exact advisory bytes/hashes. There
was **no preflight database pin** for this command, so that postflight observation
is not promoted to proof of the exact database state at each lookup. Older
seven/15-advisory receipts remain historical data, not rewritten results.

No dependency update, deny exception, target exclusion or MSRV increase is applied.
Qualification identity stays
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`;
all live predicates, observation frequency/order and 300/60/10 origins/windows
remain unchanged. No throughput/allocation improvement is inferred from test
completion or sample free-space differences.

## Controls, first refusals and evidence

The integration initially passes 41 complete local controls; the workspace
accounting successor passes 42. Controls cover both fixed command paths, partial
and missing coverage, worker/receipt/frame drift, replay refusal, exclusive
locking, separate native streams and before-Cargo request mutation. All old
preparation and Job controls remain in the cohort. Python syntax checks pass.

Before workspace execution, an offline replay of the original October 24 log as
**data** exposes `unrecognized Cargo target`: Cargo's outer
`Running target\\debug\\xtask.exe verify` line is not a test harness. Only that
exact entry is recognized separately; duplicate, wrong or missing successful
entry still refuses. Its regression control rechecks original index/object hashes
and retains the old policy failure, never reruns archived code. The first loader
also errs because the old packet uses `files`, not `members`; its exact script,
empty output and labelled manual tool-response transcription remain evidence.

The first advisory-metadata reader refuses CRLF framing. The successor changes
only in-memory TOML-header newline parsing, retaining original database bytes
unchanged in a distinct output. The initial script/empty output and labelled
tool-response transcription remain. Neither correction repeats a Cargo command,
changes a recorded outcome or weakens a source/identity check.

The [content-addressed packet](local-runs/windows-fixed-cargo-20261027/index.json)
retains raw streams, claims, controlled helper versions, native fixture results,
actual Job receipts, complete before/after frames, accounting and postflight
security data. Archived code is inert. Package/workspace verdicts and release
admission are separate booleans; the record explicitly keeps release admission false.

## Next work

The dispatcher/accounting gap is closed. Next perform a fresh security source/
reachability/MSRV review of all 11 entries, including the four newly observed
zstd-family advisories. Apply only approved compatible updates with their own
semantic/MSRV gates; retain separate migration decisions for YAML, ZIP and zbus.
Do not add deny exceptions or change thresholds. After a reviewed changed-input
candidate, run the complete workspace entry again; do not append the unstarted
suffix to this red receipt. Original execution, all-writer retirement, lifecycle,
reconnect, production enrollment and compatible host/server admission remain
separate open release boundaries. There is still no frozen admitted C74.
