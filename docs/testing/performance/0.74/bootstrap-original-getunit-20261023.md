# Original owner attempt after the GetUnit correction

The separately authorized original-owner attempt passed the corrected non-loading
unit-absence preflight and created its own transient slice. It then refused during
live kernel revalidation at issue_progress_challenge after publishing Reserved,
before causal/consent frames,
durable Starting, held delivery or child readiness. The exact first exit is 101,
with Reservation(Input(Kernel(Invalid))) and first kernel site clock-order. It was
not retried, relabelled as readiness, or used for measurement/qualification.

## Candidate and execution boundary

The [one-attempt plan](bootstrap-original-getunit-plan-20261023.md) keeps every Rust
runtime and fixture byte, Cargo.lock and compatibility contract at 8fb0fb5a.
getunit_once.py requires a clean committed candidate, the previous retained
preflight refusal, unchanged qualification/previous archive, green source-stable
checks and exact fresh GNU build metadata and hashes. The existing shell only
adds one exact new output path; its enrollment, timeout and cleanup bodies remain
unchanged. The exclusive native-once directory is consumed and must not be reused.

The executed source was 541bdc792bb92b0b763ae50050ce0dc462ae50e6. GNU libtest
SHA-256 was ca5484aa951414b80aefd0148c7d8f5185a8fdf73bb197cf98a790de59b8cff1.
The temporary diagnostic bootstrap remained 1,680,168 bytes, SHA-256
4750a12568c4293ad8db6ce356abb272e36ea2eb81d6c1639c55f1a74947dfce. It was installed
as an approved fixture but never launched: the libtest and its admission/cleanup
helpers ran, not a bootstrap or worker. Test-only
bootstrap documents stayed under the same private protected trust root; no
production enrollment or key changed.

## First refusal and offline attribution

The saved kernel trace records these two TimePoint observations:

- BOOTTIME: 4761189865740 to 4762640585769 ns, advancing 1,450,720,029 ns.
- REALTIME: 1792752579332718998 to 1792752574760274264 ns, decreasing 4,572,444,734 ns.
- Same boot flag: 1. Original packet hold limit: 60,000,000,000 ns.

diagnostic_kernel_admission.rs ordered() at line 259 rejects a changed boot,
backward BOOTTIME or backward REALTIME. Here the saved REALTIME observations
violate that unchanged predicate; the positive BOOTTIME interval is below the
unchanged hold limit. This is not evidence of a preparation-budget overrun or a
reason to enlarge a window. The trace does not identify a particular clock
service, vDSO/CPU behavior, WSL mechanism or implementation defect as the cause.
No operator action changed clocks, masks or existing services.

The recorded kernel phase is unassigned; issue_progress_challenge is identified
by the failing fixture call site, not invented as a trace phase. The signed
Reserved publication and fence were reached. Causal observations and all
three age-site call counts remain zero. No prepared-delivery, manager-send or
readiness marker appeared. All later original live checks remain present in the
unchanged code but were not exercised by this refused attempt.

## Cleanup and preserved evidence

The fixture emitted one creation intent and confirmed the new transient slice.
Its Group and Scope cleanup both reported success. The shell retained test_rc=101
and cleanup_rc=0. The temporary UID/GID 986, supplementary GID 987, diagnostic
image/created parents, mailbox and recovery directory are absent. A separate
read-only GetUnit check found both exact fixture unit names not loaded. Only the
newly created marked slice needed retirement; no diagnostic service was started.

Original account entries were byte-compared after account removal, and account
tool backups restored to this attempt's baseline. Session pre/postflight bytes
match for boot, manager namespaces, supervisor state and existing clock masks.
The supervisor is absent in this isolated distro, not stopped or restarted.
The 120-second outer libtest bound and every original 300/60/10 window stayed
unchanged. Cleanup success does not substitute for original all-writer retirement.

The separate archive local-runs/bootstrap-original-getunit-20261023 contains 242
logical files in 228 SHA-256 objects: raw check/native logs, exact checked sources,
compiler metadata, build proof and the offline analysis method. Binary copies
and private account recovery contents are excluded. Its index SHA-256 is
9f2fd0f457ec6d48f4e87e7bfc2bd1262bbf191cf51b760fe744eb369394b9f6.
getunit_seal.py validates the first negative trace and exact objects without
contacting the host. Git index audit verifies byte preservation on add.

The prior evidence index remains
d3858133295f5d202be21c9dcce48d87051f5690d84a110ee131796f80059f82; the qualification
manifest remains 11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.
Neither earlier refusal is overwritten by this experiment.

## Local checks and remaining gate

Before dispatch, 16 Python controls, 22 source contracts, two focused GNU tests,
GNU check/strict clippy, a fresh selected bootstrap/libtest build, shell syntax
and formatting passed with stable source snapshots. Four later offline clock
classification controls also passed, for 20 Python controls in total. Test counts
are correctness checks, not throughput, latency or allocation measurements.
Final doc-check, registry validation (188 discovered gates, all registered),
formatting and the local nonpromotable performance contract also passed.

The non-loading absence correction is now exercised on the real manager. Healthy
original delivery, authenticated readiness, all-writer cessation, completion,
later cells and server admission remain unproven. The next useful step is
read-only attribution of the time-observation inconsistency, keeping this first
failure and all guards. Any later live candidate needs its own recorded boundary;
the present lane permits no repeat or origin refresh.

Code commits: 541bdc79 (one-shot runner and plan), ea464f50 (offline attribution
and evidence sealer), ef6ee1ae (exact negative evidence). Documentation is a
separate follow-up commit.
