# Fresh status and PID path preservation readers

Two retained source-preservation failures have targeted repairs. Both readers
now validate the exact current admission source and pinned status parser helper
before exposing its known predecessor to the original normalization chain.
No runtime implementation, dependency, live observation or deadline changes.

## Cause and permitted delta

The original wider xtask invocation retains sixteen failed case lines and an
outer timeout in the [package frontier](package-frontier-result-20261024.md).
Fresh-status and PID-path comparisons read the current
`diagnostic_kernel_admission.rs` directly, although their historical whole-body
assertions precede the accepted status-field parser. The existing strict runtime
source gateway already describes that exact parser delta and pins its helper.

Only that pathname selects the gateway. All other paths still select the standard
filesystem reader. Both original document-scratch normalizers run afterwards;
their downstream custody/PID/fresh-status normalizers, old Git revisions and
every original assertion remain unchanged. Unknown current source refuses
before any predecessor is returned. This is test input compatibility, not a
runtime trust waiver or a new product candidate.

Fresh reader commit `54401528c6f71f4c844b47ef1e4a3ff7bd5b57b1` changes
`crates/xtask/tests/fresh_kernel_cost_074.rs` and adds the independent
`fresh_status_reader_074.rs` canary. PID reader commit
`a6310c7a2ee48dbd40f2134edba7b67f00edb39d` similarly changes
`pid_path_cost_074.rs` and adds `pid_path_reader_074.rs`.

## First outcomes and final focused gate

The fresh canary's first baseline has one pass and one expected failure: the
gateway proof passes, but the old reader lacks the exact permitted delta. The
unchanged canary then passes with implementation; ten cases across five selected
targets, all-targets check and strict all-targets/all-features Clippy pass.

The PID canary's first baseline has one expected failure. The first candidate
passes fourteen selected cases and all-targets check, but strict Clippy refuses
unused imported gateway items in the new canary. That intermediate local commit
was made before the lint failure was handled and is not an acceptable final
gate. It was not pushed. Follow-up
`de3e9b029239a801ebb2967908863694adeb36ae` adds a substantive canary that exercises
exact admission, predecessor reconstruction and rejection of a removed live
predicate. No lint suppression is added; the original lint refusal is retained.

The final changed source passes fifteen cases across seven targets, with no
ignored or filtered cases: fresh-status 2, fresh reader 2, PID-path 3, PID reader
2, status-field scan 2, authority-hex 2 and authority-hex reader 2. All-targets
check and all-targets/all-features Clippy with `-D warnings` pass. Direct rustfmt
checks cover the four changed Rust files. This is not a green full-package gate.

Canaries reconstruct each entire permitted reader from its pinned predecessor.
They reject broad selection, pathname substitution, bypassed normalizers,
changed old assertions and unrelated functions. Source controls also reject
unknown admission bytes, removed live checks, changed hold duration and changed
parser helper. Original missing/duplicate-token and live-drift controls pass.

The [retained packet](local-runs/fresh-pid-readers-20261024/index.json) contains
raw commands, source hashes, stdout/stderr, first refusals, both runner sources
and exact old/current reader source. Each bounded local invocation reports
unchanged source and HEAD over its own execution. These are correctness and
structural results; no performance measurements or gain claims are added.

The sealed packet has seventy members and fifty-four content-addressed objects;
its index SHA256 is
`fff2645db67e36a22d83894419a4aa8e1aa2271bf1a162727b27bb77314fa446`.
Doc-check, local non-promotable performance-contract 0.74 and registration of all
188 discovered gated tests also pass. These structural gates do not resolve
the full-package or security-policy refusals.

## Unchanged release boundaries

Cargo inputs retain their exact working SHA256 values:
`18a8a27cf140a1d6eb31c8316ee39bb7d5832866a990346eee16cd0c30de344b`
for Cargo.toml and
`d2c7736ebaa8381cece9162bbbf384db60c46111051e4e78e51d4bae12feaf5c`
for Cargo.lock. Qualification manifest SHA256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
Dependency work remains audit-only; the fifteen-advisory refusal is unresolved.
ReservedLive retains its original observation count, order and deadlines.

No server action, service/clock change, original-owner rerun, performance rerun,
qualification or additional build deletion occurs. The previous wider-package
failure and first runtime refusals remain intact. Next audit custody/document
scratch preservation and exact source inventories separately, then return to the
still-open preparation/delivery/writer-retirement lifecycle gates in the
[closure audit](release-closure-audit-20261024.md). Targeted reader repair alone
does not open server admission.
