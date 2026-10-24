# Ordinary regression repair: owned oracles, frozen inputs and retained refusals

The changed GNU/MSRV ordinary library gates remain red. The focused fixture,
descriptor and bootstrap repairs pass, as do all 321 old source-contract tests,
current/mutation controls, all-target check and strict all-feature Clippy.
This closes specific test defects, not original delivery, retirement, a stable
clock environment or release admission.

The [990-member packet](local-runs/ordinary-repair-20261024/index.json) has 904
content-addressed objects, 16,072,219 object bytes and index SHA256
`5d32f67e9b2f36028750ed4fb67b581678dbba50ed19ad72f5ea0cab20ce1bf3`.
It retains the first compile/lint/source refusals, rejected fixture candidate,
focused results, both complete ordinary results, source/image hashes, native
strict checks and the final cohort's complete input-byte snapshot. It does not
contain ELF images. Older uncommitted coordinator versions are not all retained
as raw bytes; their source-map hashes and available before-change captures are
kept without claiming complete raw-coordinator provenance for those lanes.

## 1. Isolate an oracle's ownership, not its assertion

Two existing descriptor tests assumed no unrelated thread/fork in their process.
The first complete cohort retained an immediate-EOF refusal and a global FD-count
refusal. Independent owned controls showed that a CLOEXEC writer can remain in
another fork before exec, and that thread-self procfs FD enumeration still sees
other threads' open descriptors. These controls establish possible interference,
not the identity of a historical unobserved holder.

`e1f4be9c` adds a test-only exact-case subprocess scope. It runs each audited old
body once in the same test image, requires exactly one matching successful case,
keeps its leader unreaped during owned process-group inspection, and bounds
retirement. Timeout, nonzero/empty selection or a surviving descendant is failure;
cleanup never converts it to success. Six helper controls pass per compiler.
Both old FD cases pass with unchanged assertions and four parallel parent threads.
This scope is not a production cgroup/fence or a proof of cessation by all writers.

The first combined compile/test command expired while compiling and executed no
tests. Later lanes separate bounded build from execution of its exact copied and
hashed image. First MSRV explicit-write lint refusals remain; direct stdout
write_all preserves the uncaptured child-barrier bytes. No broad lint allowance,
ignore attribute, full-suite serialization or retry loop was added.

## 2. A valid signature must not override an independent root pin

Six old material tests supplied current Cargo.lock bytes against a historical
receipt. The initial correction hypothesis was wrong: re-signing the current
digest still correctly refuses at the unchanged independent ROOT_LOCK_SHA256.
Both v6 compilers retain seven Invalid failures. The candidate was withdrawn.
The preceding v5 control also retains a wrong-argument compile error; GNU's
diagnostic renderer panicked while reporting it. Neither compiler ran tests there.

`bcbf94d8` instead supplies the exact 175300-byte historical root lock already in
the protected bundle and independently identical to `62114be0:Cargo.lock`, hash
`be46eaf8e97e507bda5e3f2b671d0d622e6a639ce1d4613769a4693723d62b19`.
The original receipt field, current workspace lock and production verifier/pin
do not change. Seven cases pass on GNU/MSRV. Negative controls distinguish wrong
supplied bytes/Contents with sticky material refusal from a valid synthetic
signature over a foreign digest/Invalid at the independent pin.

This corrects the earlier plan's field-rebinding hypothesis; it does not rewrite
the old receipt or erase the rejected result. Selecting a final clean production
build and its reviewed input pins remains a separate release/trust decision.

## 3. Keep intentional busy-executable refusal

`f5e59e3f` extends the same checked scope to fifteen exec-sensitive bootstrap
cases in three existing files. The two ancillary cases remain unchanged, so the
complete target still selects all seventeen with four parent threads. Both
compilers pass 17/17. The intentional open-writer ETXTBSY case keeps its writer
and expected shell refusal; it is not quietly made executable.

Whole predecessor bodies remain exact after the closed additive source delta.
The first broad reader adaptation was rejected: applying historical transforms
to unrelated raw readers broke seven contracts. The final adapter routes only
the three changed bootstrap files, and its independent mutation guard preserves
all old assertions. Source ordering and unused-import Clippy refusals are kept.
The final 22 bootstrap preservation tests plus three owned-scope controls pass.
All 321 old performance source tests pass without ignored/filtered cases.

## 4. Complete ordinary cohorts are still first negative results

Both v9 builds are pinned before separate execution, with all features, four
threads, no selectors and no request for ignored/manual tests. Their source maps
are unchanged. They overlap locally; this is not a paired timing experiment.

| Toolchain | Selected | Passed | Failed | Ignored | Filtered |
| --- | ---: | ---: | ---: | ---: | ---: |
| GNU Rust 1.94 | 1623 | 1474 | 8 | 141 | 0 |
| MSRV Rust 1.88 | 1623 | 1473 | 9 | 141 | 0 |

The old six material and two isolated FD failures are absent from both final
failure lists, and the added material control passes. Other old failures can
disappear while new names fail, so comparing totals does not diagnose them.
Each lane records eight clock-order events: saved REALTIME goes backwards while
BOOTTIME increases and the boot ID remains equal. The original predicate refuses
as designed. Parallel event/panic adjacency cannot assign each event to a case
or establish the cause of every earlier Kernel/Time refusal.

The MSRV result also retains another unisolated transport EOF case reading -1
instead of zero. Its separately preregistered
[scope follow-up](transport-eof-scope-plan-20261024.md) audits all callers of the
same oracle. Do not repeat the complete kernel/time cohort to obtain a green
interval. The offline saved-log reviewer passes five synthetic controls; the
build/image runner passes six, without live processes or measurement.

## 5. Separate product correctness from performance

The first ordinary embedded Windows cohort passes 686 tests over 102 targets,
with 25 ignored and zero filtered. Its raw command/output/result are retained
here. The earlier separate RESP/client-surface cohort passed 251 over 23 targets,
also with 25 ignored. Product sources/Cargo.lock are unchanged by these repairs.
No product goodput, latency, allocation/RSS or native floor is measured here.

Current/MSRV all-target checks and strict all-feature Clippy pass for the
supervisor. Xtask check/strict Clippy and package formatting pass. Earlier source
cohorts with 320/1, 20/2, 15/7 and reader-order failures remain negative evidence;
the final source result does not relabel them.

Original windows, fresh observation count/order, qualification manifest,
production trust and old 0.73 artifacts remain unchanged. No original-owner
manual retry, service action, new server attempt, installation, product load or
expensive qualification belongs to these repair lanes. The
[closure audit](release-closure-audit-20261024.md) lists the real next gates.
