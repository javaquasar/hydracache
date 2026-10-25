# Exact source successor step evidence

This packet retains the source-only repair of thirteen artifact compatibility
failures, not another full package or live/server run. The twenty unchanged
artifact tests pass on Rust 1.94 and MSRV 1.88. Five new projection controls and
fourteen inventory controls pass on both compilers. The original first red
package and earlier 7/13 artifact outcome remain in their separate packets.

`index.json` binds immutable raw objects by SHA256 and length. Members include
the new canary's first COMPAT refusal, the first implementation's E0277 compiler
error, subsequent checks, source/HEAD snapshots, final implementation, fixed
producer bodies, per-path history/deltas and local coordinators. Input hashes
identify the first source variants; this packet does not claim to reconstruct
every transient source body from those hashes. No private keys, executables or
product workload outputs are included.

The index has 79 members in 66 distinct raw objects, with SHA256
`fc8a4d25330d0a4b4701ba072ac55bebee67d9f40e9450b0b9a301913cf1c215`.

The raw check wrappers retain the same 1,200-second local step bounds and
four-GiB disk floor. These are not a replacement for the unchanged 780-second
full-package guard. The five-path rules have their own independent
`61d3e1941cdae497600592a49af0487e4cf545573ba3f091add173110b560cda` pin;
rewriting the evidence index cannot approve different source rules.

Replay and mutation controls are offline and do not execute retained source,
Git, Cargo, fixtures, workloads, services or clocks:

```text
python docs/testing/performance/0.74/local-runs/source-successor-view-20261026/verify.py
python docs/testing/performance/0.74/local-runs/source-successor-view-20261026/test_verify.py -v
```

The [result report](../../source-successor-view-result-20261026.md) describes the
exact scope, preservation checks and next package boundary. This packet supplies
no qualification, installation or production admission.
