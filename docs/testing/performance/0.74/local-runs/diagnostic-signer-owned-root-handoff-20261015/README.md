# Ordinary selected self one shot dummy transfer evidence

Source `c30c0bc4cc417e44a38d7482dafaf651f3df3302`, tree
`cfd4c3ca0fd95c4ea65e0139b5854bdcdcbfee88`, follows baseline
`ae1c3b7c3582fd8facbacb09dbc95654122ca378`, preregistration
`20ccd4b1f9c7fb89646683997376d5bed4a66e7e` and feasibility amendment
`a8849dfdab4a10dc14dae57cbe693e5d86e2bece` before the next candidate.
This is local public-dummy lifecycle evidence, not a credential or product run.

## Fixed behavior and results

Baseline passes 52 tests. New-mode red fails 0/1 with the old binary's exit 2.
Final current Rust 1.94 groups pass 61/61 three times with default parallelism,
two after source commit: 41 root units, five ordinary child-entry, seven policy,
one root CLI and seven process tests. Every group actually exercises all 25
normal handoff modes beside the previous root/policy/stopped-loss controls.
The ordinary per-process five-second guard is unchanged, not serialized or widened.

One normal success requires exact original READY/read/dummy/ACK/close/zero-exit,
reap/ECHILD and EOF before final root cleanup. The public dummy is sent once,
only after the original recv entry and full context/maps check. Missing/wrong/
duplicate ACK after delivery has one successful send but is a refusal, never a
retry. Send failure is attempted with zero successful sends. Withhold/replay
send nothing. Original child SIGKILL and Drop controls cover installed, ready
and delivered stops. These controls do not establish creator loss at those cuts.
Markers explicitly distinguish one successful handshake and 24 refusal/cleanup
controls; no generic eventually-dead observation is promoted to handoff success.

Eight units cover closed selectors, state/event/send budgets, original endpoint
inode/access/NONBLOCK/CLOEXEC, actual record lengths, duplicates, queue/EOF and
48-byte failure drain, finite startup allowlist and 320 full-width argument
mutations. The normal raw fork child and policy bytes are unchanged. Parent
installation/stack/event helpers are reused, not replaced by readiness assertions.
Full context at installed, exact recv entry and post-close brackets a trace with
no FD/root/namespace/process mutation except final close. It is not a general
cached-context API or independent runtime attestation.

MSRV Rust 1.88 passes 61 tests while refusing all 25 handoff modes for extra libm
under the unchanged four-role graph. It is not accepted runtime support.
Windows current/MSRV portable groups pass eight each. Windows/Linux current/MSRV
checks and current strict Clippy pass. MSRV Clippy is not executed; the retained
installed-component inventory has no cargo-clippy. No toolchain installation.
Static contracts pass 163 and the gated registry remains 175. Formatting, doc-check
and local non-promotable performance-contract pass. Require-ship's wrapper exits 0
only after verifying the expected child exit 1. No full release verify is claimed.

## Retained development failures

The first refusal is followed by an instrumented trace identifying a rodata
loaded-marker pointer that failed the stack-only reader. Only the new socket
marker moved to stack. The next development attempts hit the existing five-second
outer guard; bounded stage/timing diagnostics locate repeated full context scans
before completion. Those timing values are development observations, not an
allocation/CPU/throughput benchmark or admitted optimization estimate.

The amendment records full-context boundaries plus the intervening exact trace
before the next candidate. That version reaches zero exit and EOF but correctly
fails the test because temporary diagnostic stderr remains. Instrumentation is
absent from final source. The temporary timeout helper that retained stderr is
also absent: the verifier compares the complete final outer-guard body to baseline.
An early 25-mode run overlaps briefly with a development 61-test group; final
repetitions are separate invocations, not duplicate concurrent matrices.

Linux lint then rejects items after an inline test module. Tests move into a
private trailing module/file; strict lint passes without an allowance. First-mode,
matrix and earlier 61-test successes remain development captures rather than
being counted as final-source repetitions.

## Reproduction and consistency

With the installed Linux x86_64 Rust 1.94 toolchain, strip/readelf, nonroot account
and the existing namespace/ptrace/pidfd kernel facilities, run from the repository:

```text
cargo +1.94.0 test -p hydracache-long-run-supervisor-074 --bin diagnostic_signer_root_probe --test diagnostic_signer_root_probe --test diagnostic_signer_root_cli --test diagnostic_signer_child_entry --test diagnostic_signer_isolation --locked --offline -- --nocapture
```

Old regression commands are the previous owned-root-loss packet's four separate
rooted/loss, first-exec, policy-parser and old-owned/GNU/CLI groups. The broad rooted
group passes 83; the remaining groups pass 11, 3 and 24, for 121 regressions.
It starts only after all final ordinary root process groups finish; no claim
that isolated success fixes the historical overlapping-broad-load timeouts.
Actual WSL target paths and compiler profiles remain visible in captures. Only
private binary copies are stripped; no graph, image/proc bound or policy exception.

From this packet directory run `python -B verify_packet.py` and
`python -B -m unittest test_verify_packet -v`. Manifest and log bytes, exact result
counts and all 25 classified markers are pinned. Historical Git source/tree,
preregistration/amendment order, 11 changed artifacts, 19 unchanged sources, frozen
inputs and the earlier load negative are checked. Original raw-child/wait/root
transition functions, parent installer helpers, recomposed child installer and
the complete outer guard are mechanically compared with baseline.

Logs normalize to LF and one final newline; empty format output is one newline.
The unsigned verifier establishes historical consistency only, not execution or
kernel attestation, and does not require a future checkout to equal this source.
Ten mutation tests reject promotion, hidden failed-send attempts, MSRV refusal
relabelling, changed bytes/counts/markers, incomplete capture and source drift.

No trusted credentials, issuer/job/runtime custody, authenticated worker start,
P2/P3 all-writer cessation, completion/later cells, host/GitHub operation,
qualification, ship or performance claim is enabled. Transfer-boundary creator
loss remains the next isolated experiment; earlier stopped-loss results cannot
be substituted for it. Frozen qualification inputs and old packets stay unchanged.
