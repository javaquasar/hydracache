# Next eight preparation packages: implementation evidence

Baseline: `98d7321b1cda568c386cbfdef1993c7e4dcebe38`. Plan:
[pre074-next-eight-implementation-plan.md](pre074-next-eight-implementation-plan.md).
Release 0.75 remains planned; W0 and production IMap remain blocked by 0.74.

| Package | State | Local evidence |
| --- | --- | --- |
| P1 managed builds | Implemented | 8 process-controller tests, 9 CI contract tests, 10 existing process tests; all-target check and strict Clippy; real Cargo/Maven preparation and package reproduction |
| P2 helper ports | Implemented | 7 helper variants in Rust/Java ports and external consumers; core/helper/async tests, all-target check, strict Clippy and unchanged Java coverage ratchet |
| P3 native observation | Implemented | Native Maven suite, strict independent Rust verifier, preparation v2 artifact/replay; actual managed packaged probe passed |
| P4 native histories | Implemented | Two real official clients; seeds 117/118 with actual overlap; Java and independent Rust DFS, strict raw/normalized binding, replay and bounded shrinking; preparation v3 artifact |
| P5 listener recovery | Implemented | Controlled writer gate and exact callback tuples; 29 native suite tests; actual packaged recovery probe independently checked, preparation v4 artifact/replay |
| P6 continuous soak | Implemented | Clean-source actual smoke: reference 19 rounds, native 5, daemon 11, all reaped/no live members; checkpoint replay creates a new segment; preparation v5 and local-archive workflow; no 6/24-hour execution claimed |
| P7 development SBOM | Implemented | Clean-source `61b08b5794e1617930d49d4707f8c02089a4905b`: 995 components; complete generation and independent reconstruction passed; 12 Rust tests plus actual five-stage graph replay and six Java tests; dependency policy Rust failed / Java unknown, unsigned |
| P8 scoped coverage | Validation in progress | LLVM/JaCoCo capture/reconstruction, strict source/tool/cfg/session binding, zero-missed changed-line gate and negative canaries implemented; focused Rust tests and strict Clippy passed; 39 native and 10 provenance Java tests with mandatory class gates passed; native source report has no missed executable lines; no Rust coverage percentage claimed before actual capture |

P1 uses a separate build execution policy. Public short proof workers retain the
60-second maximum. Deadline, output/event/stage exhaustion, cancellation and failed
cleanup stop further build stages. Failure reports distinguish complete logs from
captured prefixes. Workflow budgets begin before checkout and reserve cleanup time.

P2 uses the original helper async facades, service owners, retained ledger, credits
and leases. Read invocation identities do not create retained mutation entries.
Immutable development grants and bindings check local adapter consistency; they
are not production authentication. Prepared requests belong to their local port.
Java completion leaves a leased result usable until invocation.close, which also
closes an already delivered lease. Rust results retain their existing owned lease.

Local logs reside under ignored `target/imap-075/`; exact source-bound receipt and
remote matrix qualification are performed after the final implementation commits.
General repository CI failures inherited from 0.74 are reported separately.

The P8 capture at `5835aaeb519f3c0ff4f99fbb09e8ee5c1b6b9d1e` completed its
instrumented Rust workloads, actual preparation and strict preparation replay.
Its analysis correctly refused an incomplete live LLVM profile; it produced no
qualified coverage report. The subsequent fix defers only the measurement
process's own profile and requires it after physical exit. Seventeen coverage
tests and the explicit public-CLI refusal canary cover this boundary. Fresh
current-source capture and zero-missed-line qualification remain required.

All four prepared-artifact integration tests are registered in a closed 0.75
extension imported by common discovery validation, and wired to the bounded 0.75
qualification step. A scoped test checks their registration and actual workflow
references, with canaries for foreign metadata. The common registry still
reports eight unregistered Linux targets in the unchanged 0.74 supervisor test
sources; this is not a passing global-registry or general-CI claim.

The clean `95fed9bba` capture completed preparation/replay and native probe CLI
commands, then refused its all-cohort smoke because the caller supplied only
60 seconds of shared host budget. Cleanup was confirmed. A separate diagnostic
run with a 180-second shared budget completed reference/native/daemon smoke and
both checkpoint/report resumes. The actual five-stage Cargo replay and all eight
provenance mutation cases passed; the soak mutation fixture's distinct-producer
binding was diagnosed and corrected. These diagnostic profiles are retained;
they do not constitute a qualified coverage report. Fresh final capture remains
required after the fixes.

The clean `344ce949e` capture passed tooling/helper/consumer tests, preparation
and replay, public native probes, all three smoke cohorts, both resumes, and all
four actual prepared-artifact tests. Its default 40-minute outer budget expired
during coverage analysis; cleanup completed and no qualified report was issued.
An explicitly unqualified closed-profile diagnostic retains 229 profiles and 35
objects; the empty profile of the killed analyzer is recorded as excluded, so
this diagnostic cannot satisfy qualification. The Linux matrix independently
exposed an `ESRCH` procfs exit race and missing bootstrap binary IDs. Follow-up
code handles vanished procfs entries explicitly and uses v2 control/capture
records with immediate immutable archives of Cargo executables, host build
scripts and proc-macro libraries. Fresh qualification remains required.

Additional focused canaries cover capture publication/dispatch without recursive
Cargo, actual Windows junction refusal before creating soak controls, idempotent
atomic cancellation markers, and exact log-prefix hashes after interrupted or
partial writes. Finite raw worker faults are allowed to exit so their final LLVM
checkpoints can be observed; their malformed, leaked, replaced, partial or
wrong-profile executions remain ineligible for completed segments. These checks
do not replace final clean-source coverage qualification.

The `277aa0ec6` matrix completed without qualification. Both Rust 1.94 lanes
reached the final strict LLVM reconstruction and refused uncovered changed
executable lines. Both Rust 1.88 lanes refused the missing tracked external
consumer source component. The Windows `31ee66134` capture passed its workloads
but reached the same missing-consumer refusal during measurement; no qualified
report was produced. Its 150 raw files are retained in a ZIP with every entry
independently SHA-256 verified before removing the uncompressed copies.

The follow-up consumer manifest compiles the tracked absolute source directly
inside an isolated external workspace, avoiding copied relative source remapping.
New tests cover parser work bounds, deleted-file handling, malformed cfg
attributes, UNC/source aliases, runtime-matrix refusal, retained Java phase
bindings and real build-output redirects. Linux procfs parsing, bounded inventory
and inherited-session checks have injectable read inputs with tests against the
actual source module on Rust 1.88/1.94 in Ubuntu WSL. Additional prepared SBOM
mutations check target/compiler/stdlib/version/distribution substitution and
source changes. Fresh full capture, independent checking and the complete matrix
remain required before P8 can be marked qualified.

The clean `59dd581fe` retry completed the Windows workloads and final strict
reconstruction, which refused 197 uncovered added executable lines. All four
Linux Rust 1.88/1.94 × Java 17/21 lanes in run `41097005376` likewise reached
strict reconstruction and refused uncovered lines; HC/2 run `41097005395`
passed. These are diagnostic results, not qualified reports. The Windows raw
archive retains all 140 files with every entry SHA-256 verified. The actual
SBOM inventory contains 1001 components, remains unsigned, and reports failed
Rust policy and unknown Java policy. The short smoke and resumed segments
remain short runs; they make no six-hour or real-cluster qualification claim.

Follow-up profile collection includes the bootstrap role as well as all five
workload/analysis roles. Empty bootstrap data is permitted when no host program
runs, but every emitted bootstrap file must be validated and merged. Unknown
roles and redirected role directories refuse collection. Atomic profile staging
now lives outside `raw`, so temporary files cannot become unregistered inputs.
Tests cover empty and malformed bootstrap data, unknown roles, actual directory
redirects, source changes during runner preparation, oversized controls and
absolute output ownership. This correction preserves the v2 record shapes and
does not relax the zero-uncovered-lines requirement.

Private reconstruction tests exercise all eleven external-command boundaries,
stale/duplicate Java sessions, incomplete Java builds, changed raw inputs,
projection mismatches and duplicate source components. Their simulated counts
are zero, their capture is incomplete, and they never publish a coverage report.
Separate lifecycle tests distinguish a live controller, a reused PID, confirmed
absence and unreadable process identity. Pure metadata checks cover capture
aliases, workload producer/tool substitution, the six compiler streams, exact
object inventory and closed analysis profiles. The actual entry points retain
fixed tool selection and mandatory LLVM/JaCoCo reconstruction.

The follow-up Windows checks passed 141 library tests and eight native-history
tests in both default and instrumented builds; the coverage/CI targets passed
39 and 12 tests respectively. All-target checks, strict all-feature Clippy,
formatting and doc-check passed. The actual Linux process/build source modules
passed 19 tests and strict Clippy on each of Rust 1.88 and 1.94 in Ubuntu WSL.
Those tests include real finite owned shell/nested-controller executions as
well as modeled syscall, ownership, procfs and deferred-reaping failures. The
modeled trees suppress Drop, so unit failures cannot signal fixture PIDs. This
source harness is not full Linux package or P8 matrix qualification.

The retained Rust 1.94/Java 21 final stream for `633b469a9` has 176 uncovered
added executable lines, compared with 216 in that lane for `59dd581fe`. It
remains an unqualified diagnostic. Its bootstrap-inclusive capture reached
strict reconstruction rather than failing input collection.

The next CI/process block tests all 27 orchestration boundaries through private
fixed-function bindings. Modeled coverage is always unqualified and cannot
publish a CI receipt; the actual coverage checker also refuses an unqualified
relative report before preparation starts. Actual bounded adapters run one
finite child without recursive Cargo. Pipe-setup I/O refusal reaps the real
owned child before returning a containment failure; direct and nested worker
PID-publication failures exit normally enough to retain their LLVM counters.
The default/instrumented checks passed 148 library, 12 CI and 13 process tests;
the instrumented coverage target passed 39 tests. All-target checks and strict
all-feature Clippy passed. Linux source-module checks passed 22 tests and strict
Clippy on each exact Rust version, including the full procfs exit-race lookup.
Fresh complete source-bound coverage and matrix qualification are still required.

All four `633b469a9` lanes in run `41110867177` reached strict reconstruction
and refused uncovered lines; HC/2 run `41110867182` passed. The clean Windows
`22a3c9b10` capture completed every workload, actual prepared-artifact test and
reconstruction but refused 152 uncovered changed executable lines (197 in the
earlier Windows diagnostic). It retained six compiler streams, 107 objects,
three Java sessions and an unsigned 1001-component inventory with failed Rust
and unknown Java policy. Its 161 raw files were archived with per-entry and ZIP
SHA-256 verification; no qualified report was produced.

Follow-up guards read the source SHA both before and after expensive profile
and source-snapshot collection, and independently recheck all bound inputs at
worker/analysis completion. Tests refuse control aliases and substituted
producer hashes. CI publication now rejects unqualified metadata at the common
writer boundary, including direct callers. Linux uses the exact pre-exec
callback in parent-side tests with modeled function pointers; production keeps
fixed libc calls and performs no LLVM/file/allocator work after fork. Source
and configuration errors are tested before any child can be launched. Each
exact Linux toolchain passed 25 source-module tests and strict Clippy; this
does not replace full matrix or coverage admission.
This guard block also passed 152 library and 12 CI tests in both Windows build
modes, 38 default/39 instrumented coverage tests, all-target checks, strict
all-feature Clippy, formatting and doc-check.
