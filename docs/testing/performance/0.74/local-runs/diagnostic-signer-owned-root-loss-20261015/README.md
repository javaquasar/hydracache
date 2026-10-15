# Ordinary selected-root creator loss: owned stops only

Implementation `1f399e0a7040d2a0733825aed2fd13fc6caf85a4`, tree
`591821cf65dc780ce52cfa00f2d690a146cec995`, follows preregistration
`9fe69db4d262e16d6b3da48410c95899d652936f` and baseline
`72b4ba1e7bbaac5d05230c3e55d2a5e6dfb0ee41`.

## Observations and controls

The unchanged ordinary baseline passes 43 tests. The new-mode red fails 0/1:
the old binary refuses root-loss-initial with exit 2. First development modes
pass 1/1; subsequent groups pass 50 and 51 as credential/pipe/ancillary tests are
added. These are development observations, not final-source repetition claims.

Final current Rust 1.94 groups pass 52/52 three times with default parallelism,
two after source commit: 33 root units, 5 ordinary child, 7 policy, 1 root CLI and 6
process tests. Every final group actually exercises five original-creator kills
at initial-after-options, rooted, first-exec, loaded and installed stops, two
healthy acknowledgements and one matched installed no-EXITKILL survivor control.
Each passing loss signals only the original creator. The original worker must
terminate with SIGKILL and pass WNOWAIT/reap/ECHILD and stream EOF before root
cleanup. The negative survivor's later explicit worker kill is cleanup only.
Test markers classify these outcomes separately; all have admission=false.

The monitor holds original root objects from before its fixture-only creator fork;
there is no fresh creator-CLI exec claim. Kernel per-packet credentials and initial
original sole-child/tracer/pidfd/pipe-FD3 identity are checked before advancement.
The endpoint stays open until creator terminal/reap. An unrelated original pause
child remains alive through loss and worker EOF. Four additional fresh installed
loss monitors run sequentially per group and preserve caller context. This does
not prove loss behavior at every instruction or pre-arm/running/thread-only cuts.

Nine new units cover all non-PID frame-bit mutations, unknown modes/lengths,
scalar/status shapes, packet limits, missing/wrong credentials, CLOEXEC, incorrect
gates, original pipe identity/access, payload and ancillary truncation, and root
preservation without quiescence. Rejected duplicate pipe writers must all close:
the original read side reaches EOF rather than EAGAIN. Only the unit-owned outer
TempDir tears down its deliberately preserved synthetic root.

MSRV Rust 1.88 passes 52 tests. All eight normal root-loss modes explicitly refuse
its extra-libm image under the unchanged four-role graph; that summary is not
runtime admission. Windows current/MSRV portable groups pass 8 each. Linux and
Windows current/MSRV checks pass. Strict current-toolchain Clippy passes on both
systems. MSRV Clippy is **not executed**: the installed toolchain lacks cargo-clippy;
its failed invocation is retained, with no component/toolchain installation.

Old regressions pass 121: rooted/loss 83, first-exec 11, policy parsers 3 and old
owned-probe/GNU/CLI/process 24. The broad rooted group is launched after the final
ordinary root process groups, not overlapped with those five-second outer guards.
Its actual duration is retained, not interpreted as a performance observation.
Static contracts pass 162; gate registry remains 175. Formatting, doc-check and
local performance-contract pass. Require-ship refuses with expected exit 1.
No full release verify/merge/tag/server qualification is claimed.

## Reproduction and consistency limits

From the repository root, on Linux x86_64 with the already installed Rust1.94,
strip/readelf, required namespace/ptrace/pidfd kernel facilities and nonroot user:

```text
cargo +1.94.0 test -p hydracache-long-run-supervisor-074 --bin diagnostic_signer_root_probe --test diagnostic_signer_root_probe --test diagnostic_signer_root_cli --test diagnostic_signer_child_entry --test diagnostic_signer_isolation --locked --offline -- --nocapture
cargo +1.94.0 test -p hydracache-long-run-supervisor-074 --lib signer_root_ --locked --offline -- --nocapture
cargo +1.94.0 test -p hydracache-long-run-supervisor-074 --lib signer_first_exec --locked --offline -- --nocapture
cargo +1.94.0 test -p hydracache-long-run-supervisor-074 --lib diagnostic_signer_isolation::linux --locked --offline -- --nocapture
cargo +1.94.0 test -p hydracache-long-run-supervisor-074 --bin diagnostic_signer_owned_probe --test diagnostic_signer_owned_probe --test diagnostic_signer_owned_cli --test diagnostic_signer_gnu --locked --offline -- --nocapture
```

The actual WSL target directories are visible in each capture. Private test copies
alone are stripped. No policy, graph, timeout or qualification-input exception is
used. Logs normalize to LF plus one final newline; format's empty output is one
newline. Require-ship's wrapper exits0 only after verifying the child exit1.

Run `python -B verify_packet.py` and `python -B -m unittest test_verify_packet -v`
from this packet directory. The verifier pins manifest bytes, historical Git
commit/tree/blobs, 18 unchanged sources, 9 changed source artifacts, frozen input
bytes, the previous overlapping-load negative, exact result counts and separately
classified automatic/healthy/negative markers. It mechanically removes only the
new private loss module/entry to compare the original Owner/raw worker unchanged.
Eight mutation tests reject promotion, relabelled refusal, changed bytes/counts,
incomplete capture and ordinary-owner drift.

This is unsigned historical **consistency**, not proof of execution, kernel
attestation, independent custody or trusted evidence signing. The verifier does
not require a future checkout to equal the historical source tree. Prior failed
overlapping-broad-load evidence stays byte-pinned, not superseded by targeted
success. Product code, credentials, protected handoff, authenticated worker-start,
P2/P3 all-writer cessation, completion/later cells, server/GitHub operations,
qualification, shipping and performance claims remain closed.
