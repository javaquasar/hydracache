# First native coordinator phase control

The opt-in coordinator recorder completes its first bounded local Windows
control: sixteen workers and sixty-four ordinary short Git calls, with no
operational or telemetry refusal and no cleanup error. All six phases retain
their raw native clock brackets. This closes enabled-mode compatibility for
this exact diagnostic build, not recorder overhead or the full package gate.

## Source and unchanged execution contract

Source/HEAD is `1030acb263e96d1e023f852dceb021de458ce6f4`. A new offline/locked
Rust 1.94 no-run build precedes the sole attempt; its source inventory and HEAD
stay unchanged through build and native postflight. Worker image SHA256 is
`c0e2478e96d31e1b9d2100d0bce4706720f274b68e8cda698b9dcc6780dba94f`.
The [existing protocol](../../../../tools/xtask-package-attribution-074/coordinator-frames-protocol.toml)
and collector are unchanged from the model stage and retained in the packet.

The exact `--coordinator-frames` gate selects its separate exclusive output
directory. The four alternating blocks keep the original Git argv, working
directory, inherited environment and pipe semantics. The common 120-second
origin, five-second owned cleanup, 4-GiB disk floor, 16-MiB log bound, independent
Git image pins, initial owner checks and stable terminal Job census remain.
Every Job reports nine total members, zero active and zero forced terminations.
Each worker emits four exact-body/status results and one complete footer.
There is no retry, V6 replay, server/service/clock action or product workload.

Both reference and profile arms have coordinator brackets enabled. Their
difference concerns the existing worker's per-call observation, not coordinator
telemetry on versus off. The older untraced packet is a different attempt and
cannot be used as a paired overhead baseline for these new observations.

## Retained phase observations

All 192 probes contain 576 native clock reads in the declared order. The
independent reader checks nonnegative integer endpoints, wall/CPU monotonicity,
exact phase schedule, each bracket delta and containment within the original
whole-frame metrics. Across sixteen frames the observed values are:

| Phase | Sum wall lower to upper bounds ms | Sum process CPU ms | Zero CPU spans |
| --- | ---: | ---: | ---: |
| setup | 23.8718 to 24.3114 | 0 | 16 |
| spawn | 468.0865 to 468.2296 | 234.375 | 3 |
| wait | 10772.1677 to 10772.5186 | 15.625 | 15 |
| terminal reads | 2.0683 to 2.1601 | 0 | 16 |
| decode | 519.6645 to 519.8839 | 0 | 16 |
| close | 3.9700 to 4.0623 | 0 | 16 |

The sum of original worker-frame wall observations is 11793.0502 ms; the sum of
original coordinator process CPU deltas is 250 ms. The phase CPU deltas also sum
to 250 ms in this input. That equality does not establish zero cost in gaps,
export or other zero-delta phases. CPU counters are process-wide observations,
not exact CPU of an API or proof of timer precision.

The largest observed wall upper bracket is wait at block 1/slot 3,
970.1749 ms. Spawn reaches 173.1420 ms at block 0/slot 0; decode reaches
79.7211 ms at block 2/slot 1. Brackets include clock bookends and possible
scheduling, not pure effect-only time. Long wait wall does not mean coordinator
CPU work. Decode has positive wall but zero measured CPU in every frame; neither
zero cost nor a cause such as filesystem, scheduler or security software is
established. The signed four-block original estimator remains unchanged and
`overhead_accepted`, `speedup_established` and `package_pass` stay false.

## Evidence and checks

The [159-member packet](local-runs/coordinator-native-20261026/README.md) retains
the two source-preserving build/run receipts, every worker log/result, raw native
intent/result, exact collector/runtime sources, model predecessor index and the
unchanged qualification manifest. Index SHA256:
`55b802621e147681f9478a164053a04c055ec28d8d35227d68d49ad0b4de380f`.
The image hash is a retained build/run observation; offline replay does not
attest currently mapped executable bytes or require executing the binary.

Six independent integrity controls pass, including corrupt/missing objects,
consistent duplicate-ledger mutations, changed raw clock arithmetic, operational
verdicts, identity, terminal census, source/build pins, deadlines and block sums.
The verifier reads retained bytes only and does not import a retained collector,
invoke Git/Cargo, sample clocks or start a worker. Additional final check receipts
are retained separately without rewriting this first native packet.

All 56 original model/caller/accounting cases pass again. xtask all-target
check, strict all-target/all-feature Clippy, doc-check, verifier syntax and the
local non-promotable performance contract also pass. The
[seven-check ledger](local-runs/coordinator-native-20261026/final-checks/README.md)
has index SHA256
`d5bd23a54f15415f2cc1ea5f3ca39d71e24fe2e9691e0e6f4f7c81d5f71f9e8b`.

Next, use these saved endpoints to narrow remaining transport/process-capture
attribution. Any recorder overhead experiment needs its own preregistered
enabled/disabled comparison; this success is not that comparison. Preserve the
V6 timeout and all earlier refusals. Full package coverage, frozen C74, original
delivery/all-writer lifecycle and production admission remain open. Qualification
manifest SHA256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
