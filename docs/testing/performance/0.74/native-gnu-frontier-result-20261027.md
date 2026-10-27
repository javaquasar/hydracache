# Complete GNU frontier on the independent native checkout

The separate five-target GNU frontier passes all 31 cases in local WSL on
native ext4. Full tree/ref postflight passes. This closes that specific Linux
compatibility boundary; the earlier Windows-mount timeout and red incomplete
full-package result keep their original outcomes.

Runner implementation: `3abf66ac69b6d567cdb16ba364ff7e71ba8f4a6b`.
Tested native source: `636feb166b273b5c3d6fc0b93c4fccf57560cf03`, not the later
documentation HEAD. The [preparation receipt](native-frontier-preparation-result-20261027.md)
defines its complete offline transfer and independent object database.

## Source and environment review

The runner checks the fixed transfer request, exact HEAD, all 53,842 tracked
file bodies/executable identities and 404 resolved ref/object-ID bindings.
No evidence path is pruned. The V9 review supplies a fixed source superset,
not blanket GNU package clearance: all 386 Rust inputs and 56 auxiliary bodies
are compared as data. Of those 442 bodies, 332 are raw-exact and 110 differ only
by the reviewed Windows CRLF to native LF representation. Extensionless
content-addressed objects always require exact raw bytes. No archived code runs.

The actual test caller's Git configuration is checked separately from the
transfer helper's sanitized Git environment. Only the five fixed repository
core settings are present; there are no remotes, borrowed objects, filters,
hooks or lazy-fetch configuration. Inherited Git/build/test fixture overrides
and shadow Cargo configuration are absent. The exact native Cargo alias config
is pinned to its source blob. Rust 1.94.0 targets `x86_64-unknown-linux-gnu`.

The five targets perform fixed read-only historical/current Git operations,
source/AST/string comparisons, in-memory mutations, and owned temporary-file
and symlink refusal controls. Their complete audits retain every old protected
path. Compilation uses the existing owned target directory and the locked
vendored protoc build path; Cargo is offline. Nothing starts a manager fixture,
product workload, SSH operation, service or clock change.

Five pure runner controls pass on Windows and Linux, covering exact argv,
incomplete/duplicate/filtered summaries, source drift, directional EOL handling,
object path/hash substitution and duplicate JSON keys. These preliminary control
captures precede the runner commit. The actual project attempt additionally
binds its committed launcher and helpers before dispatch and records the source
map before execution; no pre-execution binding is invented for the preliminary
untracked controls.

## First distinct native attempt

The unchanged five-target command is:

```text
cargo +1.94.0 test --offline --locked --no-fail-fast \
  --target-dir /home/artur/.cache/hydracache-074-194-check -p xtask \
  --test frontier_current_view_074 --test current_frontier_074 \
  --test current_tree_input_audit_074 --test authority_rust_history_074 \
  --test history_status_readers_074 -- --test-threads=4
```

It retains 900 seconds including compilation and four test threads. Unlike the
old `/mnt/c` attempt, this independent checkout needs no `GIT_DIR` or
`GIT_WORK_TREE` workaround. The unchanged SHA-pinned CI heartbeat/subreaper
contains its original child tree with the five-second retirement budget. Both
native free space and the physical Windows backing volume retain the 4 GiB
floor; the outer coordinator monitors physical capacity during execution.

| Target | Passed | Failed | Ignored or filtered |
| --- | ---: | ---: | ---: |
| authority_rust_history_074 | 10 | 0 | 0 |
| current_frontier_074 | 7 | 0 | 0 |
| current_tree_input_audit_074 | 1 | 0 | 0 |
| frontier_current_view_074 | 9 | 0 | 0 |
| history_status_readers_074 | 4 | 0 | 0 |

All five complete summaries are present in this one attempt. The watchdog
reports success and owned-child retirement, with 87.873 seconds diagnostic
elapsed including compilation. Fresh full-tree/ref postflight and the outer
Windows source/HEAD postflight pass. No retry occurs. That elapsed value is a
bounded test receipt, not a repeated CPU/end-to-end comparison or throughput
claim about ext4, Git or HydraCache.

## Evidence and remaining release work

The [packet](local-runs/native-gnu-frontier-20261027/index.json) retains the
review, commands, exact source witnesses, first outcomes, all five summaries
and postflight receipts. Its index SHA256 is
`be69c7c5e73cec2a0fb6d94da4faceebbcfe25a5532771f50db35f3aba9241fd`.
Neither the old GNU timeout nor V8 is replaced by pooled partial results.

Next review the current full-package coordinator and run its separate unchanged
780-second/four-thread/4-GiB/no-filter/no-retry command. Security policy, original
worker release/all-writer retirement, durable completion/cells 2–4, reconnect,
production trust and compatible-host admission remain separate open gates.
No live predicate, observation frequency/order, 300/60/10 window or threshold
changes. The qualification manifest remains raw SHA256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
