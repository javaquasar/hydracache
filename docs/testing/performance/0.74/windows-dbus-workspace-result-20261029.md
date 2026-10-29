# Windows workspace continuation after D-Bus root adoption

This milestone remains **refused**, with no merge/tag/server/release admission.
The first source preflight and actual whole-workspace failure are preserved;
neither is replaced by later focused successes.

## Fixed source successor and first execution

The original YAML input guard correctly refused the changed root Cargo.lock before
Cargo. The D-Bus successor verifies two complete current bodies and reverses
exactly 38 lock edits and two manifest edits, then invokes the unchanged historical
YAML, Swagger and sys checks. The old versioned APIs continue to reject newer
inputs. The original source catalog remains 394 Rust inputs, 58 auxiliary inputs,
26,832 sites and 156 xtask targets. No extra Git lookup enters these inverse guards.
Seventy-seven inert controls passed before dispatch; helpers were committed at
`7d0f28851` before the whole-workspace command.

The one actual `cargo xtask verify` used default Windows Rust/Cargo 1.94 and the
original Windows Job: 4,800 seconds including startup, 16 GiB initial capacity,
4 GiB running floor, five-second capacity polling and cleanup. Its own process
exited 1; retirement succeeded with no cleanup error. The minimum sampled free
space was 25,218,928,640 bytes. It did not stop on capacity or a timeout.

Eleven stages completed: documentation, management/performance contracts, release
feature-leak check, formatting, all preceding Clippy/allocator variants, and
dependency bans. Stage 12, dependency policy, refused:

- RUSTSEC-2026-0391: zstd 0.13.3;
- RUSTSEC-2026-0398 and RUSTSEC-2026-0403: zstd-safe 7.2.4;
- newly observed RUSTSEC-2026-0414: pulldown-cmark 0.13.4.

Nine later stages were never started. No workspace test pass is inferred from
the successful Clippy stages. Bans/licenses/sources passed; advisory findings were
not ignored. The prior three D-Bus advisory findings no longer appeared.

Advisory DB HEAD changed from `99670cfc0bc574433034b75876a4b791ccaa2efb`
before execution to `40f5be6fb12ed93fc591bf4ab23b7d793b3f2239` afterward.
Both observations were clean. These are observations around cargo-deny's own
fetch, not proof of an individually sealed DB revision for every advisory lookup.

## Source coverage audit and isolated correction

The observed source/ref/configuration/toolchain/environment frames were identical
before and after this failed execution. A subsequent member audit found that the
3,688-file source frame omitted 40 tracked `fuzz` files and two files from
`tests/post-publish-consumer`, despite recognizing all 36 workspace members.
Consequently this first frame is explicitly **incomplete**. The already refused
milestone cannot become qualified retroactively.

The successor adds both original member prefixes to source/status custody and a
coverage predicate deriving every member's required tracked files from the
already obtained observations. Missing files in a future member also refuse.
Original selection order, node_modules exclusion and Git observation count/order
are retained. Seventy-nine inert controls pass, including omitted-member and
no-extra-Git cases. This correction alone does not authorize a repeat of the same
failed dependency graph.

The exact first framed source bytes, 77/79-control receipts, all raw first streams,
source audits and fixed producers are retained in
[`local-runs/windows-dbus-workspace-20261029/index.json`](local-runs/windows-dbus-workspace-20261029/index.json):
3,781 logical members / 3,696 objects; index SHA-256
`c7dade0ea66bb2adec6784c50872b112e07d13e007676c3c9e3c857c305b083f`.
No binaries are retained. Object bytes are marked non-text to preserve original
working-file CRLF as well as original streams.

## Security continuation boundary

The Markdown finding has a compatible upstream candidate, 0.13.5. Review it
separately with actual consumer options, first baseline refusal, GNU/MSRV and
consumer code generation/tests before accepting a root successor. The remaining
three Zstd findings still require the previously recorded Chitchat compatibility
decision. The declined preview candidate S is not silently adopted to turn policy
green. No exception is added to deny.toml.

No runtime live checks, original attempt windows, test deadlines, clocks, services,
server state, supervisor or product workload changed. This milestone supplies no
native/RESP performance claim and closes no original delivery, all-writer
retirement, lifecycle/reconnect, enrollment or server rehearsal gate.
