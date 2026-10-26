# Windows CI watchdog first results

Predecessor: `e2b96d64fd85419b5a2be837b9b7336f51b39ed5`. Index SHA256:
`455bbaebec68d634de633811fa7a94135366d7e2271408f9721f3f17b665d36c`.
Logical names address exact raw SHA256 objects; this is offline
correctness evidence, not product qualification or a new full package result.

The [late-witness follow-up](../ci-watchdog-cleanup-budget-20261026/README.md)
supersedes the main helper's source with two additional cleanup deadline checks.
This initial packet and its seventeen-case stage remain immutable; current final
coverage is nineteen unit cases plus a six-case native cohort in that follow-up.

```text
python docs/testing/performance/0.74/local-runs/ci-watchdog-owned-cleanup-20261026/verify.py
python docs/testing/performance/0.74/local-runs/ci-watchdog-owned-cleanup-20261026/test_verify.py -v
```

The verifier executes no retained code. It checks fixed runtime bytes, source/HEAD
stability, original old-case verdicts, separately recorded native cohorts, original
held process births/terminal states, post-terminal file sizes and exact error-50
injection. Seven offline integrity controls cover changed index/runtime/old verdict,
PID birth, file growth and error injection.

The first seven-test deterministic run on the old helper has nine failure entries
(five case failures plus four nonfinite-argument subtests). Its wrapper did not
inventory CI scripts; the separately copied before-source is not represented as
a pre-command hash receipt. Candidate commands use the extended script inventory.
The first seven-case candidate and first twelve-case old Rust pass are separate
from final seventeen-unit and twelve-case-per-compiler coverage.

`native/` retains the first five-case cohort and exact harness source. `streams/`
is a separate one-case cwd/argv/stdin/stdout/stderr check. The first attribute-update
control proves refusal without child creation, not specifically error 50: setup
could overwrite LastError. `unsupported/` contains the strengthened one-case
injection at the actual failing call, with exact `ERROR_NOT_SUPPORTED` assertion.
`posix/` contains four old CLI/import controls, not POSIX early-root-exit proof.
Source snapshots keep these distinctions; passes are not spliced into a full run.

Runtime/main module hashes stay identical across the final old-case compiler
checks and native controls. Qualification manifest and old Rust cases are unchanged.
Old v5 package evidence/pins stay fixed; no server, service, clock, dependency,
product workload or qualification action is performed. See the
[analysis and remaining scope](../../ci-watchdog-owned-cleanup-result-20261026.md).
