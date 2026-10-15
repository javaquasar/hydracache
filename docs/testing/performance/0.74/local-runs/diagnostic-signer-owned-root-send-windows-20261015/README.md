# Original send window local evidence

This packet covers two fixed creator-side stops, not arbitrary-instruction or
in-syscall loss. Six modes separate positive creator-only SIGKILL, healthy
one-shot continuation and untraced no-EXITKILL counterexamples. See the
[design](../../diagnostic-signer-owned-root-send-windows-design.md) and
[preregistration](../../diagnostic-signer-owned-root-send-windows-preregistration.md).

## Captures and reproducibility

The baseline contains the complete original 63-test group. The new-mode red
executes the new scenario against unchanged implementation and receives exit2.
The standalone development matrix covers all six modes. A development targeted
group passes 67 tests before a final source-only restoration of the old
channel.attempt() local binding for its existing source guard. That capture is
development evidence, not execution of the final source commit. Final repeated
groups are captured separately after the implementation commit.

The transport feasibility check creates only two local nonblocking socket
pairs. It observes ECONNRESET then zero/HUP with unread input and only zero/HUP
without it. It is not creator/worker process-loss evidence. Real Rust units and
process controls separately require the exact new terminal prefix and unchanged
EOF guard. No record/timeout/graph/policy threshold was widened.

All process cases run under the existing five-second outer guard and default
parallel harness. New six cases follow old25 and old14 in their existing test;
the red-only standalone test becomes a helper rather than another concurrent
fixture. Run under installed Rust1.94 WSL/nonroot from this worktree:

```sh
cargo +1.94.0 test -p hydracache-long-run-supervisor-074 --bin diagnostic_signer_root_probe --test diagnostic_signer_root_probe --test diagnostic_signer_root_cli --test diagnostic_signer_child_entry --test diagnostic_signer_isolation --locked --offline -- --nocapture
```

MSRV substitutes +1.88.0 and must refuse all new six modes for extra-libm. Old
rooted83/first-exec11/policy3/owned-GNU-CLI-process24 groups run after the ordinary
process groups, not concurrently with them. Checks/lint use affected packages
hydracache-long-run-supervisor-074 and xtask, all targets/features, locked/offline;
current strict Clippy adds -D warnings. No serial harness flag is used.

Implementation source is `541c752b072a755774493e93bbecb85b4f1746b5`, tree
`30b0361b702a39f1990627bfa23f12351441cb8b`, after preregistration `acc1fa37`.
Two final current groups pass 67 tests each after that source commit:
47 root units, five child-entry, seven policy, one CLI and seven process tests.
Each executes the old25, old14 and new6 process controls with default harness
parallelism. MSRV passes 67 but explicitly refuses the new six extra-libm modes;
it does not establish supported-runtime admission. Current/MSRV Windows portable
groups pass eight each, current/MSRV checks pass on both platforms, current strict
Clippy is green on Linux/Windows, static contracts pass 165 and registry retains 175 gates.
Formatting/doc-check and local non-promotable performance contract pass;
require-ship remains an expected exit1 refusal. Windows MSRV strict Clippy again
fails on seven existing macro errors; the Linux component is absent.

Historical comparison verifies 42 original function bodies, including the old
drain, raw worker, frame codec, kernel credential channel/binding and outer guard.
Sticky attempt/dispatch and exact reset-prefix ordering have separate source
checks; none is signed execution or credential attestation.

The old regressions pass 121: 83 rooted/loss, 11 first-exec, three policy parsers and
24 old owned/GNU/CLI/process checks. They start only after both final ordinary process
groups finish. Frozen qualification inputs and all prior negative raw captures
remain unchanged. There are no product performance measurements in this packet.

The manifest seals 32 complete `.log` captures. The later unsigned verifier
transcripts `verification-windows.txt` and `verification-linux.txt` are outside
that capture manifest: verification and eight mutation/unit tests pass on each
platform. They do not attest execution of the Rust tests.

## Limits

The manifest pins historical source/tree, normalized source digests, raw capture
bytes/results and frozen qualification inputs. `python -B verify_packet.py` and
`python -B -m unittest discover -p test_verify_packet.py` check unsigned consistency,
not execution, clean-checkout or credential attestation. Old parallel-fixture
timeouts and MSRV lint failures are retained, never promoted by targeted success.
MSRV Linux Clippy is absent; Windows MSRV macro lint remains a separate failed
gate. Unrelated macros are unchanged. No full release gate, product workload,
server/GitHub mutation, performance claim, permission restart/recovery, trusted
custody, real credentials, authenticated original-start, all-writer cessation or
later cells are admitted.
