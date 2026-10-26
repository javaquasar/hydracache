# Linux CI descendant retirement: confirmed defect and bounded repair

This closes a CI-runner defect, not original worker release, hostile-writer
retirement or release admission. The default Windows Job implementation is
unchanged. Non-Linux POSIX hosts now refuse this Linux-specific path rather than
silently claiming containment without the required kernel interfaces.

## Baseline, review refusal and implementation

The first isolated dedicated-host baseline runs four controls. Two pass and two
fail: a successful leader returns success while its writer remains alive; a
failed leader retains exit 23 but also leaves its writer alive. The external
harness holds each original writer pidfd and stops that fixture on refusal.
These first failures are retained, without extending the two-second fixture
deadline or changing the existing five-second cleanup budget.

The first candidate passes four controls, but review rejects its reasoning:
`/proc/thread-self/children` is not an authoritative atomic empty-tree witness.
Its passing output is preserved as an unaccepted candidate, not substituted for
the baseline. The revised implementation uses a private process-local Linux
subreaper and requires the dedicated owner to be the sole child reaper, with
default SIGCHLD disposition and no preexisting children. It retains the original
Popen argument/stream/session behavior. Terminal original exit status is cached
when reaped; only kernel ECHILD after that reap establishes retirement.

The proc list supplies cleanup candidates only. Each candidate must still be an
unreaped child of this exclusive owner before opening its pidfd. This ownership
prevents PID recycling across that operation. Signals use the retained pidfd,
never an unprotected numeric PGID. Subsequent cleanup observations adopt/reap
new orphans; each reap observation is bounded to 32 events. The private subreaper
state is restored only after retirement. A cleanup refusal overrides success.
This is not cgroup fencing and does not survive SIGKILL of the watchdog itself.

## First changed controls

The revised four-case native cohort passes. A distinct extended cohort passes
five cases, including a writer that creates a new session. Success/failure codes,
missing-program classification, live-writer timeout and cessation of fixture
writes are asserted. Each remote cohort uses only its own temporary /tmp tree;
the exact supervisor PID 377415, NRestarts=0, active/running state and boot identity
are unchanged before/after, and all nine campaign/lifecycle markers are absent.
No release installation, clock/service change or product workload occurs.

Eight deterministic ownership tests pass, covering kernel ECHILD, lost ownership,
nonterminal status, bounded reaping, stale candidates, pidfd cleanup on failure,
live-custody refusal and one-time restoration. Their first Windows invocation
has two fixture errors because Windows lacks SIGKILL; adding that mocked constant
repairs only the portable model. All nineteen existing watchdog models pass.
The twelve existing Rust CI reliability tests pass unchanged, including descendant
cleanup and original status classification. All six existing Windows native
controls pass through their required CLI. A first mistaken unittest-discovery
invocation fails six setup cases before any fixture starts (`EVIDENCE` unset);
that invocation is retained, not reported as native behavior.

The old exact-source package/effect review does not cover the changed heartbeat
or this new Linux helper. A new review and full package coverage remain required;
the 780-second V6 result is not rerun or promoted by these focused controls.

Raw source snapshots, preflight, commands, first failures and results are retained
in [the closure packet](local-runs/closure-preflight-20261026/index.json).
Its 109-member index SHA256 is
`3fca8292fe4aa75a00bf327b264e75ca11293f595fade83d56f7418274c8da82`.
Independent offline integrity, doc-check and local performance-contract checks
pass. Xtask all-targets check and all-targets/all-features strict Clippy pass;
these are focused checks, not a complete milestone. Implementation commit:
`bf3f048bd`. Original frozen qualification digest is unchanged.
