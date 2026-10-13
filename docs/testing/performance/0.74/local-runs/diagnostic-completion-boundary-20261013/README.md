# Local process exit and completion boundary evidence

The original process exiting is necessary but insufficient for authenticated
cell completion. This packet tests strict retained pidfd readiness and an actual
descendant stream write after leader SIGKILL. No completion, execution, cleanup,
server or qualification authority is created.

## Sources and reproduction

Baseline: `34b3cb37a2774347cf3865af031ad3bd0e2d04c3`.
Preregistration: `d2f3b04120f433f0cf287ae979a35ff27424ff5e`.
Implementation: `fb6afed217ed6193f2898001bbd58da80ecdfc83`.
The manifest pins the implementation, four changed source/registry files,
unchanged original authority/lifecycle code and eight frozen numerical inputs.

Run `C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-completion-boundary-20261013/verify_packet.py`
for read-only verification. The manual Linux exact test is
`cargo +1.94.0 test -p hydracache-long-run-supervisor-074 --locked --lib
diagnostic_process::terminal::tests::terminal_process_owned_leader_exit_does_not_stop_descendant_writer
-- --ignored --exact --nocapture --test-threads=1`.
It requires nonroot Linux and python3, with a preregistered 180-second gate and
30-second bounded fixture readiness. Do not relink libtest during child proofs.

## Observations and rejected hypothesis

ProcessRead.observe_terminal uses only the original retained zero-flag pidfd.
Procfs origin and boot bracket one nonblocking poll; no-event results require
the old original live-generation check. Only exact IN, HUP or their union with
one ready descriptor return terminal data. Errors/unknown bits refuse. Terminal
generation is cached pin data, not authenticated final group/cgroup membership.
No replacement PID is opened, no exit code inferred and no state written.

Three final-source manual repetitions each kill only an owned Python leader
through its retained pidfd, verify signal-nine death and terminal readiness,
then observe the still-live descendant writing to stdout. The descendant's
kernel PPid is verified under both original generation checks before it enters
the RAII signal set. The control pipe remains owned independently of leader
wait. Explicit EOF makes the descendant terminal; fallback cleanup signals
only retained owned pidfds. This proves fixture exit, not global cgroup cleanup.

The claim that leader exit establishes writer cessation is rejected. The
descendant counterexample cannot be promoted to a completed cell by a supplied
boolean, persisted state, signature or empty point-in-time tree snapshot.

## Baseline and development negatives

The fifteen existing process-reader tests pass before changes. The missing
boundary red compile fails with E0405/E0425/E0433. The first fast implementation
passes four tests but fails two that assumed immediate HUP after owner wait.
WSL kernel 6.6.87.2 retains readable exit. Tests now verify actual owner wait
separately from strict kernel readiness; the three accepted terminal masks and
all error refusals stay unchanged. This failure is retained, not relabeled green.

The kernel-parent safety review adds a separate foreign-generation refusal.
Pre-review captures remain separate from the final-source seven-fast-test and
manual repetitions. Synthetic HUP/mask tests are not represented as observed
kernel hangups. Compile-fail examples prohibit forging a view or calling
completion/resource-release operations through it.

## Gates and unfinished execution dependency

Three final fast repetitions pass seven tests each; each exhausts 65536 masks.
The existing Starting protocol separately passes nine SIGKILL cases, one control
and twenty fresh inspections. Scoped process/credential/namespace regression
passes 103 with the manual proof skipped; five diagnostic integration targets
pass 65. Windows passes 204; reviewed root contracts 166; Linux doctests 55.
Current/MSRV checks, warnings-denied Clippy/Rustdoc, scoped format, documentation,
book and artifact checks are captured. No full release/merge/tag gate is claimed.

Existing live-reader implementation remains exact after removing its new module
declaration/exports. No persistent filename/schema or write boundary changes.
Authenticated original start, validated successful output, every-writer
revocation and original empty cgroup still have no enrolled producer. P1
completion/cells two through four depend on the approved P2/P3 work before any
new completion publication and SIGKILL/lost-acknowledgement matrix can execute.

Qualification dry-run and require-ship remain expected exit-one refusals. The
requested qualification artifact is absent, frozen digests remain exact, and
no product measurement or server operation occurs.

The first packet verifier rejects an incorrect expected integration total of
52; the five retained summaries total 65. Its exact source and failed capture
are retained. The corrected count is an evidence correction, not a product
test failure or a changed threshold.
