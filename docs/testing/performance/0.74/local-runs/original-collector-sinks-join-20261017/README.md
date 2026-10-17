# Original output join: attribution and candidate evidence

Completed narrow prerequisite under the [preregistered plan](../../diagnostic-output-join-plan-20261017.md).
Implementation is `6baf3ee157e5c880ba07be2b7ac0195b0371ed3d`; instrumentation and preregistration
are separate from this packet and documentation closeout.
Baseline `e06e2ceb83c3aeacbe5cd2aa6b541773e9e4c004` has no active sink constructor.
The previous sink failure packet is not overwritten or reclassified.

Only isolated nonroot synthetic correctness fixtures are authorized on the
dedicated host. Installation, supervisor changes, clocks, product workload and
qualification remain out of scope. This packet grants no production admission.

## Baseline attribution

Both normal GNU release lanes complete three publication chains and three
separate validation chains. Runtime code is unchanged from the baseline. The
test-only entry is retained byte-exact in `attribution-built-test.rs.txt`;
subsequent formatting of the repository test does not describe a different
runtime candidate. `baseline-194.log` and `baseline-188.log` are raw server logs.

| Observed region | Rust 1.94 range | Rust 1.88 range | IPC authentications per sample |
| --- | --- | --- | --- |
| Request through accepted framed consent | 5.289–5.311 s | 5.231–5.247 s | 640 |
| Starting, 38 complete rounds | 5.873–5.884 s | 5.788–5.808 s | 646 |
| Pipes, 8 complete rounds | 1.233–1.235 s | 1.218–1.224 s | 136 |
| One pipe validation, 2 complete rounds | 0.308–0.310 s | 0.304–0.306 s | 34 |

These are three descriptive observations per lane, not independent product
benchmark pairs or release speedup claims. Wall time includes fixture checks,
storage and scheduling. IPC signature duration is nested inside authentication
duration and covers only that instrumented crypto primitive. Starting's measured
signature duration is about 20.2–20.6 ms on 1.94 and 18.9–19.1 ms on 1.88, far
short of its total wall time; this does not measure every crypto primitive.

The first offered round spends 132.2–133.7 ms in ReservedLive on 1.94 and
130.0–131.6 ms on 1.88. StorageBefore/After are roughly 0.1 ms each; Progress,
Causal, Prepared and Consent are individually 3.4–6.5 ms. FinalClocks plus the
entire unhooked second round remain an explicitly combined residual. Do not
present that residual as FinalClocks alone or infer a syscall breakdown from it.
This supports coalescing redundant outer complete traversals while keeping the
ReservedLive leaf unchanged in the next candidate.

Server root is `/tmp/hc074-output-join-20261017.Spppak`, owned 1000:1000 mode 0700.
Boot, installed binary/receipt and supervisor PID 8839/start/NRestarts zero remain
exact at both lane pre/post guards. An initial directory-creation transport command
printed the valid root but exited 1 because PowerShell appended a standalone CR
line after a trailing newline. No test ran in that command; the existing root
then passed the saved LF guard. This transport failure is not a runtime refusal
or a discarded candidate measurement.

Baseline binaries: 1.94 `394824e37576711b8d9ca76c8ad143a662cfa95d3d46c2f101ef569e3c945e7c`;
1.88 `12b731c294a21582a924a3fed54715df6e24ced2d673c5bf9505ecf718357ad2`.
At that baseline checkpoint no candidate joined or crash result was claimed. Prior broad original-path
audit failures remain in the previous packet and require matched-baseline
attribution; narrow attribution success does not waive them.

## First full joined cohort remains red

The current-compiler sink family passes all 27 cases. The next family passes
15 Starting/pipe interruptions and all 25 original trust drifts, then fails its
intentional expiry oracle: after the unchanged ten-second wait the original
Progress path returns Kernel(Invalid), before the expected Causal(Time) branch.
`j1-join-fault-194.log` retains that complete failed observation and
`j1-matrix.patch` its source. Neither a complete joined pass, MSRV joined matrix
nor a SIGKILL/lost-ack pass follows from this cohort.

Source order suggests the original exclusive Heartbeat age guard, which runs
before the challenge check. A separately preregistered first-site/ordered-clock
diagnostic must establish that mechanism on a fresh chain before an exact
oracle correction; an unspecified Invalid is insufficient. The original failure
did not capture its first-site event, so the later diagnostic cannot supply it
retroactively. All runtime checks and deadlines remain unchanged.

Raw Git output also interleaved CRLF warnings inside source lines. The
`*-git-output.txt` files retain that transport output. The `.patch` capsules
remove only the exact warning records, rejoining split source lines; all 14/16
source blobs then reconstruct against their recorded Git index hashes, and the
16 active source blobs matched exactly at the initial matrix checkpoint.
The unfiltered staged whitespace check flags ten original space-only patch
context lines in those two raw transport files. They remain byte-exact evidence;
the check passes with only those two raw files excluded, not with rewritten logs.

The separately preregistered expiry diagnostic then passes with first site
`progress-age`, ordered original samples and age 11,742,320,973 ns against the
unchanged 10,000,000,000 ns bound. Its raw `j1-expiry-attribution-194.log` and
`expiry-attribution.patch` retain the independently started entry. Original
kernel expiry references pass on both compilers. This is a fresh-chain mechanism
proof, not an event recovered from the red cohort. The next oracle-v2 cohort
changes tests only: exact numeric/site expiry plus stronger Input(Invalid)
trust-drift checks that exclude kernel/time failures. Runtime source blobs stay
identical to the initial healthy constructor. At that checkpoint, no corrected
cohort pass was claimed before its complete logs existed.

The oracle-v2 full joined cohort now passes both compiler lanes: 27 sink cases
and 41 Starting/pipe/live/expiry cases per lane. The four `j1v2-*-fault-*.log`
files retain these complete independently started families. Each lane's exact
expiry event confirms progress-age with ordered samples and the unchanged
10,000,000,000 ns bound. All 25 trust substitutions additionally exclude
kernel/time refusals. That checkpoint closed the joined gate only; the full
process-loss runner was separately dispatched after both lanes passed.

## Complete process-loss result

The separately dispatched frozen oracle-v2 binaries complete all 25 effect cuts
and AfterReturn in three repetitions per compiler. Each lane passes 78 actual
SIGKILL, three uninterrupted controls and 162 fresh read-only inspections:
156 SIGKILL, six controls and 324 inspections in total. The complete raw logs are
`j1v2-crash-194.log` and `j1v2-crash-188.log`. Their correctness-test durations
are 748.71 and 754.74 seconds; these are not product performance observations.

The parent pins each exact unreaped child with a pidfd before waiting, requires
its bounded logical readiness record and Busy ownership, snapshots original
bytes/metadata, sends SIGKILL through that pidfd and requires signal 9. Two new
processes inspect the retained context without recreating an owner. The control
uses the same process and completes without kill. Each repeated case retains
its original seed `0x074051d1`; no replacement process continues old work.

Coverage includes absent/empty/prefix/complete-unsealed/sealed/fsynced Starting,
both partial pipe allocations, every directory/file creation and sync, and
loss of the runtime acknowledgement after a successful constructor return.
This is not external wire-ACK, storage-device failure, power-loss, reboot or
exported-writer cessation proof. No worker, pump or product process was started.

The independently repeated final `postflight.sh` verifies no exact original test
process remains, no production campaign/lifecycle context exists, supervisor PID
8839/start/NRestarts zero stay exact and installed binary/receipt hashes remain
unchanged. `postflight.log` retains the values. Nothing needed restoring because
the supervisor was never stopped. Qualification inputs and clocks are unchanged.

This closes only attributed constant-depth original output preparation and its
process-loss/lost-runtime-ack boundaries. Production trust enrollment,
authenticated executable/streams/process/cgroup original-start, root collector
writer/watchdog, all-writer cessation, completion/cells 2–4, signed installation
and integrated nonproduct rehearsal remain before product/server admission.

## Reproduction and source boundary

Preregistration is `a8f4d604d5a3e4548256b195a17b00022622b042`; the separate
expiry-site amendment is `cfa9eee3eb421ba4ffba0688cd870a75900f7060`.
Reconstruct the source snapshots against the former, not against a moving HEAD.
The initial healthy screen, first full matrix, separately named expiry diagnostic
and corrected oracle-v2 source are retained separately. `verify.ps1` reconstructs
all Git blobs, checks the active snapshot and refuses missing/duplicate process
observations or a false server-ready scope. It is a final-packet verifier, not
a way to promote an incomplete proof.

`unchanged-runtime.json` pins 188 existing Rust/configuration inputs against the
preregistered baseline. The only existing runtime-source exceptions are the two
exact additive module registrations in pipe and Starting storage, independently
restored by the static contract. Five J1 runtime blobs remain identical across
the healthy, failed-oracle and corrected-oracle cohorts. No original clock,
signature, policy, authority, controller, storage or lease leaf was rewritten.
Cargo inputs and qualification manifest are unchanged.

Build each frozen source in WSL Ubuntu with its explicit toolchain and separate
target directory:

```sh
CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-194-check \
  /home/artur/.cargo/bin/cargo +1.94.0 test --release \
  -p hydracache-long-run-supervisor-074 --lib --no-run --offline --locked
CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-188-check \
  /home/artur/.cargo/bin/cargo +1.88.0 test --release \
  -p hydracache-long-run-supervisor-074 --lib --no-run --offline --locked
```

The saved shell runners contain exact libtest names, single-flight order,
timeouts, umask and fixed read-only host guards. They are records of this
authorized temporary root, not generic production commands. Each stage creates
a new scratch directory and refuses an existing one. Do not rerun a saved
runner over an old attempt or use these synthetic fixtures as worker enrollment.
The crash runner additionally requires both complete oracle-v2 joined logs and
absence of the exact original test processes before dispatch.

Local affected-package checks cover Windows/Linux current/MSRV all-target builds
and strict Clippy, 68 integration/API cases per compiler, 64 selected Original
compile-fail docs per compiler, four sink and four pipe kernel tests per compiler
and two process-protocol tests per compiler. These tests do not count an ignored
SIGKILL gate as executed. Static contracts pass 188 cases; the registry discovers
181 cfg/env gates and all are registered. Workspace formatting passes in WSL;
native Windows whole-workspace cargo-fmt exceeds the platform command-line limit,
and that invocation is not reported as a formatting pass. The ship check remains
expected-red because candidate identity and release qualification are incomplete.
