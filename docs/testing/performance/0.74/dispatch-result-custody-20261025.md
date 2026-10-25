# Dispatch result and observation custody handoff

One-shot dispatch bookkeeping, a separate target-result reader and held-process
custody handoff now pass local controls. A first synthetic session confirms that
losing the spawn acknowledgement neither repeats the spawn nor resumes profiling.
The fixed real-target receipt still forbids dispatch. This step does not authorize
production completion, writer-fence release or a new package/qualification claim.

## One-shot dispatch and retained uncertainty

`DispatchOnce` writes an exclusive `dispatch-consumed.json` before calling its
single spawn callback. It retains the returned owner object before attachment
or ACK publication. An exclusive `owner-attached.json` records that owner's PID
and creation identity before ACK. The files are flushed/fsynced, left in place
on uncertainty and never deleted to make a retry pass. Both a second call and a
new dispatch object using the same reservation refuse before another spawn.

Lost ACK, attach failure and post-spawn receipt failure retain the original owner
and first dispatch refusal. Even a pre-spawn failure consumes that object; a
partially written marker blocks a new object using the same directory. These are
local filesystem replay guards, not a power-loss/directory-fsync proof, global
single-flight fence or process-independent recovery protocol. An operator must
not use a new directory to bypass an uncertain attempt.

`dispatch_request.py prepare|dispatch NEW_LABEL` saves intent before the fixed
preflight and refuses existing output directories. `prepare` succeeds only on a
successful explicitly non-dispatching response. The actual `dispatch` request
has no process-launch API and preserves the expected
`fixed launch receipt forbids real-target dispatch` refusal with zero target
spawns. A preflight result cannot provide its own execution authority.

## Custody after the observer stops

`ObservationCustody` freezes the original coordinator snapshot on its first
refusal, thrown observer error or terminal outcome. Draining then polls only the
same original owner; no new census, profiling read, attachment or PID reopening
is permitted. In particular, an owner that exits after the coordinator's outer
bound can receive a separate terminal proof without reopening the expired
observation window or clearing its first refusal.

`ExitApi` checks signaled state, PID and creation/exit time through each already
held handle. The original root and every admitted held descendant must be
terminal before query handles are released. A live held descendant retains
custody. Identity drift, status-query failure or cleanup error retains its first
custody refusal and cannot be retried into a pass. The frozen telemetry snapshot
is never rewritten into successful observation.

The exit identity call uses `GetProcessTimes`; its required CPU output arguments
are discarded. The native counters named `post_freeze_counter_reads` count only
the collector's profiling `read()` calls, not all Win32 calls. Zero there is not
a claim of zero terminal identity syscalls. Exit witnesses do not collect I/O or
memory samples. This is observation-query custody only: unobserved aliases and
writers remain outside its proof, and both `all_writers_proven_stopped` and
`execution_completion_authorized` remain false.

## Independent target verdict

`target_result_join.py` requires the original command, four threads, 780-second
bound, no retry, the caller-reserved HEAD/source digest and unchanged before/after
source. A separately pinned AST reconstructs all 321 qualified case names. The
reader rejects unknown, duplicated, missing or ignored cases, filtered scope and
contradictory summaries. A failed or partial target retains its own failed/partial
verdict rather than inheriting a successful observation.

`historical_target_replay.py` validates the already sealed first 321-case target
at source `050d0b3fe2592fcce414c18b3aa21cd91a64e5a5`, using fixed packet and source
pins. It passes as an **archived result only**, with `new_target_executed=false`,
`live_attempt_proven=false` and `current_release_gate_pass=false`. A future live
caller still must bind its own original output files and attempt identity;
the pure reader intentionally cannot attest that binding from log text alone.

## First native controls and fixed protocol

The finite `handoff_control.py` session has two cases, healthy and lost ACK.
Both use the unchanged 780-second wrapper and the previously checked local
Python node fixture, never Cargo. Each case spawns its wrapper exactly once,
observes the same original PID/creation identity through handoff and proves all
four held processes terminal before releasing query handles. All nodes complete
and each wrapper exits zero without source/HEAD drift or early cleanup.

In the lost-ACK case, a second dispatch object is refused while the first owner
is still alive. The first dispatch refusal remains, the original owner object
is retained, and both profiling-read and census counters stay unchanged after
freezing. The frozen observation stays unsuccessful despite successful terminal
query cleanup. These are actual local process/API controls with a simulated ACK
publication failure, not a real network-loss or SIGKILL/power-loss experiment.

The new [finite diagnostic protocol](contract-observation-protocol-20261025.toml)
fixes one future target, its command/cases/threads, discovery/sample limits and
original windows. It remains `prepared-not-authorized` with dispatch disabled.
The missing live output-file/attempt binding and final observer composition are
explicit, not inferred from the prior historical pass. Actual Cargo shape is
still unobserved; unexpected delegation must refuse without a weaker fallback.

The first 156-case combined Python suite passes; later refusal/fsync/harness
controls and protocol guards bring the final suite to 164 cases. All first native,
prepare, expected-refusal and offline-reader results are retained in
[the evidence packet](local-runs/dispatch-result-custody-20261025/index.json).
No product timing/overhead claim, Rust/dependency mutation, clock/service change,
server activity, installation or qualification run occurs.

## Checks and immutable references

Implementation commit: `2877bc134485bef491f131745fcad2a7e4bf627d`.
The final attribution suite passes 164 cases and the separate historical-tool
suite passes 91 cases: 255 Python cases across the two final suites, not a sum
of repeated runs. Compileall, tabnanny, the unchanged v3 source/effect review,
documentation checks and the local non-promotable performance contract pass.
No fresh full Rust package, product benchmark or qualification pass is claimed.

The primary packet has 100 members in 76 content-addressed objects; its index
SHA-256 is `3b42f4e9373ef4f99b03935b3af9a1acbc1439c246a71b43e86982bf2e7cf771`.
An independent reader verifies the staged bytes, member lengths/hashes, retained
owner identities, terminal witnesses and the historical-only result boundary.
The [supplemental gate packet](local-runs/dispatch-result-custody-gates-20261025/index.json)
retains that reader's first result and the six saved structural gates: 30 members,
20 objects, index SHA-256
`28f10c192918fdcd02bd163df033578917b7e54caadc0c4e69a4dd8bbb493b2a`.
The fixed protocol SHA-256 is
`6e515a055cd168d69a658f399d0aff78b0e448ecd619dadfe770f8c42a60b40e`.

Next is live attempt/output-file binding and final observer composition under
the fixed protocol, before any separately authorized real-target observation.
Durable production custody, all-writer cessation, the complete package gate and
server admission remain outside this step and unresolved.
