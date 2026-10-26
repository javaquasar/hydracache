# Two reader repairs and a separate Windows cleanup diagnostic

Baseline: `52abf7ab0efd35557eb3117a45c253e0ebe76469`. This packet retains
bounded correctness commands and one separately contracted synthetic control,
not a full package run, product benchmark or server admission.

The immutable index maps logical names to exact raw SHA256 objects. Index digest:
`d0ce29b1044d27e41c7342a20865034bf107fbfc2e094fbeb9123eaa9e995331`.
The offline verifier checks raw objects, source/HEAD stability, fixed
constructor edits, per-compiler cohort accounting, first refusals and held process
birth/terminal identities. It never executes retained Rust/Python/Git/Cargo.

```text
python docs/testing/performance/0.74/local-runs/history-status-cleanup-20261026/verify.py
python docs/testing/performance/0.74/local-runs/history-status-cleanup-20261026/test_verify.py -v
```

Final corrected graph: 82 cases per compiler (Rust 1.94 and MSRV 1.88), each in
one invocation, with zero failed/ignored/filtered cases. Seventy-eight are existing
cases and four are new construction/refusal controls. All twelve old cases in the
two changed consumers and their assertion tokens are preserved. All-target check,
strict all-target/all-feature Clippy, producer reproduction and scoped format are
separate checks, not additional test cases.

First outcomes stay separate: red 2/4 (including one new-oracle historical-revision
error), initial 26-case pass with warnings, format refusal, and two earlier
82-case compiler passes followed by duplicate-module Clippy refusal. The final
shared-module correction is not spliced into those earlier cohorts. Raw initial
test, initial candidate, pre-format and pre-Clippy bodies are source-receipt bound;
this packet does not claim a complete raw source tree for the first red command.

`cleanup/` is a different diagnostic. A single taskkill returns success and the
same held root/leaf objects are terminal, but the marker already exists. The
diagnostic starts its delay after enrollment, unlike the old fixture. There is
no timestamped marker-write/termination cut here: do not infer post-termination
writing, the original failure's cause, an old-test pass or arbitrary all-writer
retirement. The original watchdog, timeout and assertions remain unchanged.

The original v5 full timeout/three failures and missing coverage stay in the
[previous packet](../package-closure-v5-20261026/README.md). Old effect pins are
not refreshed. Changed-source full execution needs a new exact source/effect
review; no such full execution occurs in this packet.
