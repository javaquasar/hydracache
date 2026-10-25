# Fixed launch preflight and observation coordinator

A pinned local launch receipt and the caller-owned coordinator now pass their
first preflight and synthetic end-to-end controls through the unchanged
780-second wrapper. Neither exposes a real-target dispatcher. This closes a
diagnostic plumbing step, not actual Cargo topology, performance attribution or
production execution admission.

## Fixed inputs before discovery

[The local receipt](contract-observer-launch-receipt-20261025.json) fixes the
original four-thread/unfiltered target command, CWD, wrapper bytes, Python,
Cargo shim, toolchain Cargo and built test executable paths/SHA256 values. Its
raw SHA256 is
`0f364991c37248f7f7d73d271932b3aaa77250d308df4a6314b0eb09905d65b5`.
The JSON has an explicit LF checkout rule; a different receipt is not adopted
from discovered processes or supplied as a new trusted CLI argument.

One new `cargo +1.94.0 test --offline --locked -p xtask --test
performance_contract_074 --no-run --message-format=json` command completes
without executing tests. Its original source/HEAD receipts, raw Cargo output
and build status are retained in the
[no-run build packet](local-runs/launch-build-20261025/index.json), SHA256
`572c66863cbc5702ef24ec24c039b96e27c2d15a1ec40b26381f8813c4ec08d9`.
The selected test artifact is fresh; the separate xtask binary is compiled.
The selected executable is 12468224 bytes, SHA256
`1f92f6532690febd531cb1f5377ddf4c7a5af2fec9409ee6a46a8f261dadb95b`.
This does not retroactively attest the binary from the earlier 321-case pass.

`launch_preflight.py preflight NEW_OUTPUT_DIR` checks the literal receipt pin,
Python/PATH resolution, wrapper/image bytes, exact build command/profile/features,
one unambiguous compiler artifact and the completed no-run status. It joins the
2028 declared crate/config/Rust-tool inputs to their original build fingerprints
and current bytes, then reuses the unchanged v3 source/effect and environment
gates. Duplicate JSON keys, path escapes, drift and ambiguous/incomplete build
responses refuse. Intent precedes preflight; first refusal is retained and output
directories cannot be reused.

The first local preflight passes with 343 reviewed Rust sources. These are
declared local build inputs and disk-file hashes, not an independently
reproduced toolchain, mapped-image attestation or production enrollment. The
real test executable is not copied into Git; deleting it requires a new reviewed
build/receipt, not an automatic pin refresh. `dispatch_authorized` remains false.

## Coordinator behavior

`observation_coordinator.py` accepts an already-owned root and the
[held-lineage collector](held-lineage-result-20261025.md); it never spawns,
kills, terminates or refreshes that owner. Elapsed time comes from the caller's
pre-launch origin. Discovery stops at 30 seconds or 300 snapshots, with up to
two additional snapshots for the single held attachment. Absence is WAITING;
ambiguity, API failure or failed attachment is a latched first refusal.

After refusal there is no further census, reacquisition or counter sampling.
The caller continues polling the original owner so its existing deadline remains
in control. After attachment, one-second observation slots sample only the held
leaf; missed slots are not fabricated or replayed in a burst. Terminal leaf
sampling occurs once. Observation completion and the owner's return code remain
separate; even complete telemetry never sets `selected_target_pass`.

At the 1200-second outer diagnostic bound, an owner still alive remains explicit
live custody. Closing is forbidden until its exit is observed. This is a
caller-owned library state, not a durable process-independent custody mechanism,
writer fence or production recovery guarantee. The real-target coordinator CLI
and its failure/custody handoff are not opened by these controls.

## First synthetic session

`coordinator_control.py` has a finite three-case schedule. Each case launches the
byte-exact original wrapper with only the previously checked local Python node
fixture, not Cargo or a product binary. The outer harness keeps the original CWD
and supplies no `env`, `stdin`, creation flags or priority overrides. The original
wrapper retains its own two existing Cargo environment settings and its original
child stream handling, five-second wait loop and 780-second origin.

The healthy case attaches the wrapper-to-leaf chain and observes leaf termination
and owner exit. The wrong-full-path case retains `held lineage image/PID mismatch`.
The injected census-fault case retains `injected census fault` after exactly one
discovery snapshot. Neither negative case reacquires a chain, claims complete
telemetry or stops the owner early. In all three cases the original wrapper exits
zero, all three fixture nodes print their completion lines, source/HEAD stay
unchanged and no cleanup/kill log is produced. First results are preserved in
[the coordinator packet](local-runs/launch-coordinator-20261025/index.json).

Eleven focused coordinator controls pass first. The first combined suite passes
131 cases; five additional harness cleanup/schedule controls bring the final
suite to **136 passing cases**, including **25 additions**. Unit countermodels
cover terminal-on-attach, owner-before-leaf exit, nonzero owner status with complete
telemetry, clock regressions/nonfinite inputs, discovery exhaustion, failed
attachment without retry, missed slots, terminal read failure and live custody
at outer expiry. The native synthetic session is executed once, not retried to
select a passing outcome. No overhead or product speedup is measured.

Code commit: `4937183cacc3bedb3ba477589ddc2b15bd59e814`.
The primary evidence index SHA256 is
`b29405c011bf7e05f0b13d0695ffacb4b19d4d2a6337ec3036568473720f6838`
(77 members, 54 objects). Its
[supplemental gates](local-runs/launch-coordinator-gates-20261025/index.json)
have SHA256
`5adf30a4ab86e3d768c39e6a7267a9c5af82ffb0da519bdbc5c3e17c6ba72d19`
(30 members, 20 objects). A separate staged data-only reader verifies the pinned
receipt, wrapper bytes, native producer source joins, all three original wrapper
verdicts and retained first outcomes without executing a producer.

The 91 historical-tool cases also pass, making **227 Python cases** across both
final suites. Compileall, tabnanny, unchanged v3 source/environment preflight,
doc-check and the local non-promotable performance-contract check pass. Raw
commands retain source/HEAD-preservation receipts; the no-run build is not a test
execution or a new Rust lint result. No full package or measurement cohort is
repeated.

An [additional staged build/gates verification](local-runs/launch-final-verification-20261025/result.json)
also passes. Its preceding shell one-liner failed to parse because of PowerShell
quoting before reading any data. The observation of that tooling error is labeled
as such, not reconstructed raw stderr. The corrected reader source, raw output
and status are retained; no build, native session or target is repeated for it.

## Remaining admission boundary

The receipt declares two possible Cargo shapes, with and without a retained shim;
neither has been observed on the actual target. A shim that exits early, an
unexpected delegation shape or an ambiguous process must refuse, not silently
drop an edge or substitute a PID lookup. Current controls do not attest hostile
parent spoofing, argv, credentials, loaded bytes or all descendants' CPU.

Before another real target observation, freeze its finite diagnostic protocol,
review the exact final observer/build composition and implement the explicit
dispatch/result/custody handoff. Preserve the original command, source checks,
300/60/10 live windows, four threads, 780-second wrapper limit and every first
outcome. A real target verdict needs its own exact case/result join; the earlier
321-case semantic pass cannot supply these new observations or a full-package
pass. No dependency, Rust product/test, clock, service, server, installation or
qualification change occurs here.
