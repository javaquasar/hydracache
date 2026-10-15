# Healthy constructor control progress after a retained stall

The first seven-case Rust1.88 pressure run passes six and fails the healthy
five-cut control. `constructor-loss-linux-msrv.log` retains the failure after
original root descriptor checks, with `original loss observer stalled`.
Initial current-toolchain five/seven-case runs passed. No positive terminal
oracle, root check or frozen timeout changes follow from that observation.

The healthy observer currently waits five seconds for the entire original
tracer to finish after releasing a pre-options cut. That includes construction,
root entry, original exec, filter installation, READY, dummy transfer and final
ACK/close. It lacks attribution inside that aggregate interval. This is a
harness hypothesis, not yet evidence that the guard or kernel is at fault.

Before the next implementation, freeze seven credential-checked one-byte
progress records from the original tracer: 1 after successful builder return,
2 after root entry, 3 after original exec, 4 after install, 5 after READY,
6 after the sole dummy transfer and 7 after complete. Each record has no FDs
and must match its exact ordinal. Keep the existing five-second per-gap bound,
30-second outer bound, 8192-byte capture and exact final success/cleanup checks.
The total received count is at most nine packets including setup/report;
loss cases still receive only setup/report and send no dummy. Progress never
admits a process or authorizes a retry. A stalled stage still fails.

Retain the first failed MSRV capture unchanged as development evidence and
rerun all final source gates. No passing rerun can erase that failure or count
it as a positive. The terminal predicates (81, SIGKILL, controls83/84) remain
exact. No qualification duration/threshold, helper, key, profile or production
authority changes. The final evidence must distinguish this confirmed wait
structure from the hypothesized pressure attribution.
