# Repaired package has no observed failure but still exceeds its bound

The first complete-command V6 attempt remains red at the original 780-second
guard. It completes 72 of 145 targets with 448 passed and zero failed cases.
The three targets containing V5's explicit failures now pass completely in this
same run. Missing coverage remains a release blocker; successful repairs do not
convert a censored package into a pass.

## Exact source and reviewed effects

Syntax-input commit `b108b3f9e` retains 368 Rust sources and 25,868 syntax sites,
with a closed contextual module graph. Review/dispatcher source is
`fa9e708868b22999d60c9ccd97b694662910a095`. Relative to V5, three Rust rows change,
three are added and 362 remain identical. Its separately fixed review pins all
45 auxiliary inputs and the complete 145-target catalog. V5 still refuses the
new graph. The [precommitted contract](package-closure-v6-contract-20261026.md)
preserves four case threads, the four-GiB disk floor and compilation inside
the original 780 seconds.

Manual review covers the exact history/status reader successors, their four-case
source-control target and the heartbeat's imported Windows Job companion.
Current-source observations, conditional read order and all old assertions remain.
The new companion contains cooperative CI children, not production Linux writers.
The historical package outer wrapper and its cleanup are unchanged. This is a
default-Windows effect review, not compiler expansion or GNU/all-feature admission.

The actual preflight and postflight source/effect/environment reviews agree.
The tree is clean at dispatch, source/HEAD remain unchanged throughout, and the
command has no filter, ignored opt-in, observer, retry or deadline override:

```text
cargo +1.94.0 test --offline --locked --no-fail-fast -p xtask --lib --tests -- --test-threads=4
```

## First complete command outcome

| Boundary | Retained result |
| --- | --- |
| Expected targets | 145 |
| Scheduled targets | 73 |
| Completed targets | 72 |
| Cases in completed targets | 448 passed, zero failed/ignored/filtered |
| Incomplete target | `held_complete_role_census_074`, four declared cases |
| Incomplete target observations | Three passed, one without a verdict |
| Unstarted targets | 72, individually enumerated in accounting |
| Cargo reported build span | 24.04 seconds, inside the unchanged guard |
| Outer result | Exit 1, `outer-timeout`, unchanged source and HEAD |

Within this actual prefix, `authority_rust_history_074` passes all ten cases,
`ci_reliability_071` all twelve and `fresh_status_reader_074` both cases. Their
V5 failures remain historical first results. The new source-control target and
the rest of the suffix cannot receive a verdict from older focused cohorts.
The incomplete case is not labeled unstarted or failed.

The unchanged outer cleanup records three successful termination messages and
one unsupported-termination error. A subsequent read-only query finds none of
the four named PID numbers; it terminates nothing. That observation has no held
birth identities and is not original-process or all-writer retirement proof.
The CI helper's Job repair does not repair or qualify this separate outer wrapper.

## Existing cost attribution and its limits

Completed-target printed spans total 709.35 seconds. The largest are artifact
fixture 261.17, custody preparation 74.52, document scratch 73.24, authority
history 73.05 and current-tree audit 56.43 seconds. These are rounded target
wall spans with concurrent cases, not CPU partitions or paired comparisons.
The difference from V5 is not a regression or improvement estimate. Subtracting
them and the build from 780 would not attribute the remaining time: the incomplete
target, wrapper boundaries and unreported intervals are also present.

A separate offline join verifies that V5's artifact helper bytes equal the
original helper used by the [retained complete helper profile](preservation-helper-spans-result-20261025.md).
The measured helper erases to that original body; the current helper also remains
byte-identical. The old profile locates most cumulative helper wall inside
`Command::output()`. This identifies an existing process/output investigation,
not another product hypothesis. It does not transfer old timings to V5/V6,
attest the current transitive input-reader graph or isolate startup from child
execution and collection.

The [existing reuse audit](preservation-reuse-audit-result-20261025.md) still
refuses naive caching/batching: fresh refs/config, ABA, short-ID resolution and
conditional observations remain semantics. No cache, persistent child, skipped
read, reduced observation frequency, parser replacement or runtime optimization
is installed on these observations. New measurement is unnecessary to repeat
that rejected inference.

## Controls and first refusals

Fourteen V6 review/dispatch controls pass together on the final source. They
cover exact Rust/auxiliary/target identities, old V5 refusal, host/include/module
and drift refusals, original command/wrappers, duplicate slots and zero dispatch
on dirty or failed preflight. The dirty scope additionally includes `scripts/ci`
and `.gitattributes`. All-target check, strict all-target/all-feature Clippy,
scoped formatting, Python compilation/indentation, final-source doc-check and
the local non-promotable performance-contract check pass.

The first eight-case review controls have one failed oracle: the test changed
the trusted canonical AST pin itself while expecting that same pin to reject
a changed known row. The corrected test keeps the real pin for that assertion
and separately checks extra formerly unchanged rows even under a mocked pin.
The admission code/pin is unchanged. The first six dispatch controls retain a
test-only undefined-variable error. Their exact initial sources are inverse-edit
reconstructions verified against original pre-command hashes, explicitly not
raw source captures. Both first streams remain untouched.

The offline join's first attempt incorrectly indexed a duration on the incomplete
target; the corrected reader includes only completed spans and never fills in
the missing duration. Its first raw failure remains; its initial ignored script
was not captured. One doc-check prints OK but its wrapper refuses a HEAD change
during receipt finalization; a separate final-source check passes without that
change. The first contract command uses an unsupported flag and refuses; its
corrected `--release 0.74` command does not change policy or implementation.

The [syntax packet](local-runs/package-review-v6-20261026/README.md) passes its
offline replay and four integrity controls. The
[result packet](local-runs/package-closure-v6-20261026/README.md) retains all first
outcomes, final controls, exact implementation, raw package streams, auxiliary
bodies, pre/post reviews and accounting; its replay and five integrity controls
pass. Result index SHA256:
`d194f84a6258a060156271d6704af3f38f3f5d7590e0f9f8ad672b531abacbd7`.

## Next closure boundary

Do not dispatch unchanged V6 again, increase its bound, filter the remaining
targets or combine historical suffixes into a green package. The next technical
step is complete-owner process/output attribution with the same fresh call graph,
followed only by a justified isolated validation-cost repair and its own exact
review. The saved output intervals do not yet select a safe implementation.

Security remains under the operator's audit-only policy. C74 composition is
unresolved, GET-owner remains terminally rejected, and production original
delivery, writer retirement, lifecycle, reconnect and trust/enrollment still
require their own gates. No dependency, service, clock, server, installed artifact,
product workload or expensive qualification changes. Frozen manifest SHA256 stays
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
