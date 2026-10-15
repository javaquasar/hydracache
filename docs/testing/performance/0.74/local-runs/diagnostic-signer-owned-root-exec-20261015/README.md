# Ordinary selected root execution evidence

Source `5ca2cf2ca28f6f39a010a205e9ab649e91ebe977`, tree
`9f56af0eab06092c8d84976fe1af286c8e2a8f58`, implements a fixed normal-build
root-exec observation, not signer or server admission. Baseline is
`73b5ccd515d8f47d0295a0f6b87628fed937c0e2`; preregistration is
`caeba7d839bd80dd039a4fb5476f8523c4d8ee00`.

## Positive local scope

Preparation baseline passes 11 unit, one portable CLI and three process tests.
The newly added root-exec test fails 0/1 against the old binary with exit2.
The final source passes 18 unit, one CLI and six process tests in each of three
targeted current-toolchain runs, using default test parallelism. One run follows
the source commit; two precede it with the same final source files. Each process
group includes one real root-exec and four further independent sequential owners.
Each real launch validates initial two and loaded four selected image identities,
then original SIGKILL/reap/ECHILD and private pipe EOF before exact root cleanup.
These are not throughput samples or concurrent-owner performance results.

The units cover maps/status refusals, 4096 seeded map mutations, phase replay,
initial/rooted/exec cleanup and an extra writer that prevents EOF after child reap.
The last control explicitly preserves the root rather than treating child death
as all-writer cessation. Synthetic ELF images can reach the kernel exec-stop but
cannot produce the real shim marker; loaded-stage acceptance refuses them.

MSRV Rust1.88 passes the same 25 tests. Its real binary adds libm: normal root
preparation/root-exec deliberately refuse the unchanged four-role GNU graph.
That is an attributed profile refusal, not successful MSRV runtime launch support.
Only private current-toolchain test copies are stripped; Cargo artifacts, runtime
graph, dependency lock and qualification inputs are unchanged.

Old rooted/loss regressions pass 83/83 with their own default parallelism. Ordinary
launcher/GNU/isolation process regressions pass 36/36, first-exec 11/11 and private
isolation-entry parsers 3/3. Static release contracts pass 160; gated-test registry
retains 175 rows. Formatting, Windows/Linux package checks and strict Clippy pass;
MSRV checks and portable Windows CLI tests pass. Local performance-contract passes,
while require-ship refuses with expected exit1. The full release gate is not run.

## Negative results retained

`overlapping-broad-timeouts.log` is a real final-source failure: while the separate
83-test group was running, the process group passed four and failed two on its
unchanged five-second outer guard. Units18 and CLI1 passed. The outer guard kills
the normal owner on expiry; that is not a successful root-exec result or proof of
root crash cleanup. The 83-test group itself passed. The overlap is observed;
scheduler/CPU attribution has not been traced and is only a plausible explanation.
No deadline, profile, terminal predicate or test parallelism was changed to obtain
the separate targeted successes. Load robustness remains open, and such owner
SIGKILL may retain a temporary root. No unknown retained directories are deleted.

Development failures also remain: a 17-byte initial marker declaration against a
16-byte type, a preflight inventory counting its own extra directory FD, the
incorrect assumption that synthetic ELF cannot reach exec-stop, two strict-lint
issues, and import formatting. The marker and inventory logic were corrected;
the synthetic oracle now requires real shim execution separately. A shared test
image module is imported once, without changing its bytes or old fixture behavior.

## Integrity and interpretation

The manifest pins 12 implementation files, 13 unchanged old sources, both frozen
inputs and 24 tool captures. Captures are normalized to LF and one trailing newline.
They are unsigned development observations, not a clean-checkout execution receipt.
The verifier checks exact summaries, negative outcomes, nonauthorizing markers,
source Git objects, tree, bounds and frozen inputs. It does not require a future
checkout to remain on this historical source and cannot establish execution origin.

Run `python -B verify_packet.py` and `python -B -m unittest discover -s . -p
test_verify_packet.py` from this directory. Eleven verifier tests reject tampering,
missing captures, changed claims/bounds and relabelled failures. WSL worktree paths
are bridged explicitly by the verifier, without changing Git state.

## Next boundary

Compose the actual restriction installer with the selected proc-less root without
weakening policy or granting host procfs to the child. Then test full ordinary-entry
creator loss and protected handoff. Runtime custody, real credentials, authenticated
original-start, P2/P3 writer revocation, completion/cells2–4, server rehearsal,
qualification and ship admission remain closed. No remote operation, signing or
cache/product workload was performed for this packet.
