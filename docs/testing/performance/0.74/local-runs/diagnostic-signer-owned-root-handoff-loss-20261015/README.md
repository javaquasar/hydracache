# Ordinary dummy transfer creator loss local evidence

This packet covers six stopped transfer cuts, six healthy continuations and two
matched no-EXITKILL survivor controls. It uses public dummy bytes only. See the
[design](../../diagnostic-signer-owned-root-handoff-loss-design.md) and
[preregistration with scheduling correction](../../diagnostic-signer-owned-root-handoff-loss-preregistration.md).

Source and final capture identities are pinned in `manifest.json` and checked
by `python -B verify_packet.py`; mutation tests use
`python -B -m unittest discover -p test_verify_packet.py`. The verifier compares
historical Git blobs and original function bodies, frozen inputs, actual log
bytes/counts and classified controls. This is unsigned consistency evidence,
not execution attestation or a clean-checkout/release gate receipt.

## Preserved failures and capture limits

The pre-change targeted baseline passed 61 tests. `baseline-process-tail.log`
contains only the captured process tail, explicitly incomplete; the initial
41/5/7/1 group output was observed in the tool response but not saved to the
packet. Do not treat that partial capture as a complete execution transcript.

The manifest seals 32 `.log` captures. Final `verification-windows.txt` and
`verification-linux.txt` are later unsigned verifier transcripts outside that
capture manifest: both verification and seven mutation/unit tests pass on each
platform. These outputs do not attest execution of the underlying Rust tests.

`red-new-mode.log` is the unchanged binary's expected exit2 refusal under the
new test. `development-standalone.log` records all 14 new modes passing alone.
`development-parallel-timeout.log` records the old handoff guard expiring in
the full group with an additional concurrent root fixture: the new matrix
passed, the old handoff test failed. No scheduler trace attributes the failure.
The preregistered correction runs the 14 cases after the old 25 within their
existing process test. It preserves default harness parallelism, the baseline
number of competing fixtures and the exact five-second guard; it does not
establish overlapping-load robustness. A development compilation rejected
FixedProbe equality because that shared enum lacks PartialEq; pattern matching
fixed the compile error without modifying the enum. That diagnostic was observed
in the tool response, not retained as a raw capture.

Older overlapping-broad-load and constructor-race negative packets remain
unchanged and pinned. Isolation/targeted success does not waive either result.

## Admission boundary

Implementation source is `97d9304f162a548ba0181cb4334960598b5f1cac`, tree
`85a1857c1a307d05150440279b5b67c9ef5cc717`. Preregistration is `22b92daf`,
with the scheduling correction in `4887964a`. Two corrected current groups pass
63 tests each, the second after source commit: 43/5/7/1/7 across root units,
child-entry, policy, CLI and process targets. Both exercise the old 25 handoff
cases followed by all 14 new transfer controls. MSRV also passes 63, with all
14 new modes explicitly refused for extra-libm. Windows portable groups pass
eight each on current and MSRV. Current strict Clippy/checks pass on Linux and
Windows; MSRV checks pass on both. Static contracts pass 164, registry 175,
formatting/doc-check and the local non-promotable contract pass; require-ship
remains an expected exit1 refusal. MSRV lint is qualified separately below.

Separate old regressions pass 121 tests after the post-commit ordinary group
finishes: 83 rooted/loss, 11 first-exec, three policy parsers and 24 old
owned-probe/GNU/CLI/process checks. They were not overlapped with the ordinary
five-second process guards. Historical comparison also verifies 22 original
function bodies, including the raw worker, owned waits, channel parser, old pipe
binding/group-stop, handoff child/startup and outer guard, are unchanged.

The main Linux command, from this worktree under the installed Rust1.94 WSL
runtime, is:

```sh
cargo +1.94.0 test -p hydracache-long-run-supervisor-074 --bin diagnostic_signer_root_probe --test diagnostic_signer_root_probe --test diagnostic_signer_root_cli --test diagnostic_signer_child_entry --test diagnostic_signer_isolation --locked --offline -- --nocapture
```

Substitute `+1.88.0` for the MSRV refusal group. Run the old groups only after
that command completes, using respectively `--lib signer_root_`,
`--lib signer_first_exec`, `--lib diagnostic_signer_isolation::linux`, and
`--bin diagnostic_signer_owned_probe --test diagnostic_signer_owned_probe
--test diagnostic_signer_owned_cli --test diagnostic_signer_gnu` with the same
package/locked/offline/nocapture flags. Check/lint use both affected packages,
`-p hydracache-long-run-supervisor-074 -p xtask --all-targets --all-features`;
strict Clippy adds `-- -D warnings`. No serial test flag is used.

The positive losses send SIGKILL only to the original creator; installed and
delivered first establish a matched SIGSTOP group-stop. Negative worker SIGKILL
is explicit cleanup after observing a survivor, never automatic-death proof.
Healthy paths finish ACK/close/zero exit through the original creator. Terminal,
reap/ECHILD and bounded EOF precede original root cleanup in all passing cases.

MSRV's extra-libm outcomes are attributed refusals, not runtime support. Its
Linux Clippy component is absent; MSRV Linux lint is unperformed. Windows MSRV
strict Clippy was executed and failed on seven existing uninlined_format_args
in hydracache-macros, also reproduced by linting that unchanged package alone.
Those three macro sources are pinned unchanged from baseline; no lint allowance
or unrelated macro edit was added. This remains a recorded gate gap, not a green
MSRV lint result. No product workload,
server/GitHub mutation, full release gate, signed attestation, qualification or
performance claim belongs to this packet. Trusted credentials/custody,
authenticated worker original-start, P2/P3 all-writer cessation and later cells
remain closed, as do pre-arm/running/thread-only and fresh creator-CLI loss.
The six cuts do not interrupt the creator between attempt-latch/send or
successful-send/recv-resume; these are the next separate local uncertainty
windows, not proved by the completed receive-entry and delivered controls.
