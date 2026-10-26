# Real Git launcher and native CPU scopes are distinct

One limited local control confirms that the directly created Git process is a
launcher with a separate native child. A CPU counter from the launcher alone
therefore excludes native Git work in this control. This closes an attribution
boundary, not the cause of V6's timeout or a performance improvement. The old
pipe profile already labels its counters as directly-created-process-only;
its values and first outcomes remain unchanged.

## Fixed implementation and first native result

Source/protocol commit `3807c4749873d9a3e9495c755bbf3ef1f73f9c7b` precedes the
sole actual control. The [protocol and design](../../../../tools/xtask-package-attribution-074/git-boundary-design.md)
fix both image paths and independent SHA256 values, original environment/effect
preflight and `git --no-optional-locks cat-file --batch`. Keeping stdin open
permits a live observation; sending EOF releases it. No Git object is requested.
The batch command is diagnostic only, not a persistent source-loader candidate.

The observer borrows the original Popen root handle. The existing HeldLineage
reader opens only a query/synchronize handle to the exact native child, with two
matching snapshots, full-path/creation checks and live brackets on both nodes.
One discovery snapshot locates the shape; the reader's separate two-snapshot
attachment succeeds once. Both original objects become terminal, root exits
zero, stdout/stderr are empty and query/root-handle cleanup has no error.

| Held node in this control | Kernel PID | Image |
| --- | ---: | --- |
| Original directly created root | 35512 | `C:\Program Files\Git\cmd\git.exe` |
| Held native child | 20648 | `C:\Program Files\Git\mingw64\bin\git.exe` |

Creation identities match the same retained objects at terminal collection.
Native lifetime CPU is already nonzero at attachment and does not advance while
waiting for EOF in the saved observations. Subtracting that late initial sample
would erase the native work already completed. Use whole-lifetime terminal
counters for that accounting scope, not an interval beginning at attachment.
This is not per-operation CPU evidence or an estimate of removable startup cost.

Image hashes before/after match the precommitted independent pins. That is an
on-disk observation, not mapped-image attestation or protection against an
unobserved A–B–A replacement. Local process lineage is not authenticated
production ownership, and two observed processes are not a census of every
possible Git descendant. This control does not retroactively establish any old
cohort's topology.

## Preserved bounds and first fault

The new diagnostic has a ten-second run window from before spawn, five-second
discovery within it, at most 100 discovery snapshots, one attachment attempt and
a separate five-second cleanup bound. Late discovery, attachment, exit and
cleanup witnesses refuse. An absent shape is bounded discovery; a refused
attachment cannot retry or choose another image/PID. EOF, not replacement or a
kill request, is the healthy release path.

Review adds a late-close fault before the native attempt. The 13-case model
command produces twelve passes/one failure because an otherwise healthy close
can finish after cleanup's window and still report success. The exact pre-fix
source is retained. Adding a fresh deadline check after handle close makes the
identical thirteen tests pass without extending the window. Further controls
cover late snapshots before attachment, the fixed protocol, first-refusal
retention during cleanup failure and postflight image drift without replay.

All seventeen new models and 43 unchanged observer/lineage controls pass in one
60-case precommit cohort. All-target xtask check, strict all-target/all-feature
Clippy, formatting and Python syntax/indentation pass. No Rust, source-reader,
dependency or product implementation changes. The actual native control runs
only once, on the committed source, with unchanged source/HEAD receipts.

## Evidence and remaining work

The [75-member raw packet](local-runs/git-process-boundary-20261026/README.md)
retains actual source copies, first model failure, first native result, independent
pins and lifetime counters. Its offline verifier and four integrity controls
pass without executing a retained collector or launching Git. Index SHA256:
`c59c64b3f666377ad1098c1795ceead8583f0f1a0d96f03efda1d5f41ad741c5`.
Final doc-check and the local non-promotable performance-contract check also
pass; their raw receipts and exact verifier sources have a separate sealed
final-checks index linked from the packet.

The next attribution must cover native work during the actual fresh-command
helper calls without stdin holds, process replacement or a changed conditional
read graph. A late process-only attachment can miss short-lived children;
launcher-only counters and pipe wait are insufficient. Any proposed observer
must distinguish its own overhead and preserve terminal lifetime coverage before
it can select a validation-cost candidate. Directly invoking the native image,
installing a cache/batch or repeating V6 is not authorized by this control.

The [V6 package result](package-closure-v6-result-20261026.md) remains red:
72/145 targets complete, 448 passed/zero failed completed cases, one incomplete
target and 72 unstarted. No new product hypothesis is opened and no speedup is
claimed. Existing live checks/frequencies, 300/60/10 origins and 780-second guard
are unchanged. No server, service, clock, installation, product load or expensive
qualification action occurs. C74 and production admission remain unresolved;
frozen qualification manifest SHA256 is unchanged:
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
