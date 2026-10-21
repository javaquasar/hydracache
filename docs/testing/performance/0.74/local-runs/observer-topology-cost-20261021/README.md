# Observer cost on the recorded aggregate kernel topology

Source `7c47fac18acc0640e62de3af5b1d5239bc931f2d` imports the existing observer
modules unchanged into a standalone local diagnostic. Both compilers pass
19 tests, all-target check and strict all-feature clippy before release builds.
The independent reader passes seven controls. Twelve independently started
cases complete in one ordinary-user WSL session with unchanged boot/source/
configuration/input/images. No manager, kernel live guard or product runs.

## What is held equal

Each repetition replays 46 ordered rounds and all eight steps with exactly
894,608 kernel spans and the same recorded per-step nodes, roots and edges as
the two original output subphases. Counts divide by 46 and edge totals admit
a uniform-arity reconstruction. That is a canonical choice, not proof of the
original per-round/per-occurrence distribution. Temporal/sibling order and
cross-observer nesting were not recorded and remain unknown. The body is an
identical deterministic checksum in all three modes; live/auth observers are
inactive. No real guard body, filesystem/crypto/IPC work or deadline is replayed.

`inactive` calls dormant hooks; `kernel` activates actual kernel spans;
`rounds` additionally activates actual round snapshots/deltas. Six cases use
all six mode permutations, with three repetitions per mode: 18 observations
per mode/compiler. There is no warmup, retry, outlier exclusion or adaptive
duration. Raw counters, graph conservation, checksums, round completion and
serialization byte counts independently verify.

## Results and limits

The following CPU deltas are the ranges of six per-case differences between
three-repetition medians, per replay of the full graph. They are not product
speedups or a GNU/MSRV ranking.

| Synthetic walk CPU delta | GNU 1.94 | MSRV 1.88 |
| --- | ---: | ---: |
| Active kernel minus inactive hooks | +45.548 to +67.690 ms | +44.741 to +59.714 ms |
| Kernel plus rounds minus kernel only | -10.741 to +22.882 ms | -1.815 to +26.242 ms |

Kernel activation has a positive delta in all six cases per compiler. The
round snapshot/delta difference changes sign: its magnitude is unresolved by
this series. Negative differences are retained as noise/ordering observations,
not interpreted as negative instrumentation cost or silently replaced.

Outside the walk, round-mode report aggregation uses 0.182–0.354 ms parent CPU
on GNU and 0.176–4.086 ms on MSRV; JSON serialization uses 0.023–0.041 and
0.025–0.078 ms respectively across all 18 repetitions. The wide MSRV aggregate
observation remains. These timings cover the kernel/round subset report, not
the complete live/auth/phase report. Decoding is only a correctness check and
is excluded. Standalone callsite/optimization, short synthetic bodies and
canonical ordering differ from the earlier live-harness context.

This isolates a reproducible kernel-observer component, not full observer
overhead. Do not subtract these numbers from previous multi-second phases,
infer a healthy admission or claim observer work explains/does not explain
the entire expiration. Live/auth instrumentation and original body costs
remain unisolated; no optimization is accepted. Next separate actual fresh
Boot/Namespace and authority primitives in non-admitting controls, keeping
full-observer limitations and all live gates explicit.

## Evidence checks

Nine mutations on a retained actual report reject changed walker/root/edge
counts or wall, negative CPU, invented temporal order, numeric admission,
changed repetitions, serialized size and mode order. Twenty-three previous
packets verify. Effective review is this directory's `review.ps1 -Mode Review`;
it reconstructs summaries and verifies methods, logs, graph and inventory.

Original live Rust/observer source and 300/60/10 windows remain unchanged.
There is no clock, service, idle-policy, server, installation, supervisor or
product action. Synthetic success does not reopen native/delivery/lifecycle/
all-writer/production enrollment or server admission. The packet is terminal
and cannot be rerun, edited or resealed after retention.
