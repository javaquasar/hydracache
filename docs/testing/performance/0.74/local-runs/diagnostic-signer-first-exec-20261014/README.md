# Owned fixture first exec evidence

Tested code is `6ea57052920f7db4528a6c4f76c31560a1d2ab2e`, baseline
`09c9d439934203e1b8e4383c1fc378f8df1f6f2c`, preregistration `790b750d`
and inherited-writer amendment `9e4e4f1c`. Exact hashes, observed/expected outer
exits and unchanged inputs are in [manifest.json](manifest.json). This is the
first local slice of the approved [bootstrap prototype](../../diagnostic-signer-bootstrap-decision.md),
not a signer/runtime custody or worker-start admission packet.

## Observed result and negative evidence

The original eight executable-observation cases pass before the missing owner
refuses compilation with E0433. The first nine-case development run has one
fixture preparation failure: opening a writable FD before restoring owner write
permission. The test preparation was corrected without weakening the FD/mode
predicate. A later nine-case check/lint pass is retained as development evidence.

Concurrent ten-case runs then fail at actual exec. Adding bounded errno exit
diagnostics identifies errno 26, ETXTBSY (`0x9a00` wait status). The deterministic
two-child inherited-writer regression fails before the fix. No retry, seed,
deadline or threshold adjustment. The final code closes all nonselected
descriptors before the initial stop; the regression and independent launches
then pass. These failures remain distinct from the final exact-source results.

Three final eleven-case Linux repeats pass. Each includes an exact first kernel
exec stop with empty output, actual same/different-inode second exec refusal,
327680 wait-status/event combinations, header/FD/metadata refusal, truncated
header kernel refusal, foreign inode, unexpected signal/overflow, CLOEXEC/drop
cleanup and deterministic writer closure. Seed `0x07403b25` adds 16 fresh
launches and four independent tracer threads per repeat. Signals/reaping target
only owned fixture children; no service, cgroup or product process is involved.

Linux regressions pass 18 bundle/executable, ten legacy IO and 12 producer cases,
plus current check/strict Clippy and MSRV 1.88. Windows portable signer tests,
current check/strict lint and MSRV pass; the launcher module is excluded there.
The normal Linux library build has no fixture symbols. Root contracts pass 137
cases with xtask check/strict lint. Format/docs, the unchanged 169-gate registry
and local non-promotable contract pass; require-ship remains expected red.
No full release verify, performance comparison or qualification is claimed.

## Offline packet verification and remaining boundary

Run `python verify_packet.py` and `python -m unittest test_verify_packet.py` in
this directory. They read local Git/files only, compare source to the retained
commits, check raw captures/qualification and strict false admission claims.
Mutation tests use copied packets only. Later source changes require checking
this packet at its retained source rather than rewriting old evidence.
Six copied-packet tests pass on Windows and Linux. Those suites check 29 build,
development and gate captures; retaining their two logs brings the final packet
to 31 captures, with eight tested-source and 16 unchanged-file bindings.

First exec is observed while the new image is stopped, but later exec is not
denied before the syscall. Seccomp, its exact installation proof, all-thread/ABI
coverage, dummy handoff, actual tracer loss and authenticated runtime are not
implemented by this slice. EXITKILL is requested, not yet fault-rehearsed. The
copied shell/system runtime is a mechanics fixture; its identifying header is
not an admitted signer ELF profile. Production issuer/pins/job/root selection,
receiver replay/fence, worker P2/P3, all-writer cessation and later cells remain
closed. Existing schemas/keys/dependencies/workflows and frozen numerical inputs
stay unchanged; no actual credential or external job is used.
