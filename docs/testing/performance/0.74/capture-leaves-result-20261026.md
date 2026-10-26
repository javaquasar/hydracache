# Held owner and decode leaf telemetry behind a separate gate

An explicit leaf mode now narrows coordinator owner checks and log decoding
without changing the original operational calls, predicate order or deadlines.
Eighteen new model tests and 56 existing regressions pass on source
`8a0a10cab2a87da519e882838b11645d702fb5e9`. The new mode has not run with
native processes or clocks; compatibility and overhead remain unqualified.

## Implementation and untouched boundaries

[`capture_leaves.py`](../../../../tools/xtask-package-attribution-074/capture_leaves.py)
adds two held-owner leaves and six decode leaves. The
[design and protocol](../../../../tools/xtask-package-attribution-074/capture-leaves-design.md)
fix their boundaries and budgets. Only exact
`--coordinator-frames --capture-leaves` selects this mode and its separate
exclusive output root. Intent precedes lazy import/schema refusal and all
original preflights. Default and outer-only mode keep their existing measure
functions and result shapes; no environment activation or fallback exists.

Owner identity/path validation and initial held-process read/validation run in
their original setter order. The child is retained before the first new clock
read. No probe is inserted into successful CreateProcessW-to-publication, its
last-error handling or cleanup. The CI Job companion and Rust capture helper
remain byte-identical. These owner leaves do not isolate native process-creation
cost or the rest of spawn preparation, publication and resume.

Decode separately brackets stat/bound, read, text decode, record parse, original
protocol checks and result join/residual checks. The original read/rows compound
expression remains in that order, with only the leaf-aware rows equivalent.
Read timing ends at that function's entry, so it includes a call boundary and
must not be called pure disk IO. The rows operations and predicates retain their
original AST. No key, workload, byte/status, source/image, terminal or deadline
check is removed, cached or reordered.

There are at most sixteen additional probes/48 recorder clock reads per worker,
or 84 recorder reads with the unchanged outer Tape. Original metric/deadline
clock observations remain additional and unchanged. Raw endpoints, domains and zeros
remain. CPU is coordinator process-wide, not worker/native Git phase CPU. Wall
bounds concern bracketed CPU-read instants; nested leaves cannot be added to
outer spans. Export and sample-file append are outside the original frame
metric but still consume the existing shared 120-second origin. Cleanup stays
five seconds; the 780-second package guard and 300/60/10 live origins are intact.

`Child::wait_with_output` remains a single opaque worker span. Separating its
wait and pipe collection would require changing the mechanism, not inserting a
clock at an exposed boundary. This stage deliberately makes no such replacement
or causal claim, and leaves that attribution gap open.

## Fault and differential checks

The first ordinary leaf fault freezes both leaf groups. Original owner
execution, wait and close continue; a refused diagnostic stops later workers
without selecting a subset or retrying. The independent outer Tape retains its
previous policy. Original operation verdict, cleanup errors and recorder errors
remain separate. Metadata precedence preserves an existing outer refusal when
leaf attachment runs; it is not a cross-recorder temporal ordering claim.

All 48 additional read positions have ordinary-fault and fatal-interrupt models.
They preserve original healthy draining or owned cleanup respectively. Direct
setter tests prove retention before a fatal first probe and prevent replacement.
Other controls cover invalid clocks/domains, cross-group wall/CPU regression,
incomplete/incorrect schedules, export copies, unchanged decode/protocol errors,
final late-deadline refusal, strict flag/schema/import gates and first-refusal
cohort stop. Model differential preserves all original fields/calls, argv and
four deadline reads. The final combined source-preserving run passes 74 cases.

The first twelve-case run has one fixture error: expected short temp pathname
was not canonical. Only the fixture's expected image becomes `resolve()`;
the original runtime path predicate stays exact. That first source and failure,
then passing fifteen-, seventeen- and eighteen-case sources/results, remain
retained. These are model tests, not SIGKILL, lost-ACK or native fault proofs.

## Evidence and remaining native gate

The [63-member packet](local-runs/capture-leaves-20261026/README.md), in 44
addressed objects, retains nine raw receipts, exact final/historical sources,
original bodies, prior native index and unchanged qualification manifest.
Index SHA256:
`d539ff2fa93959cf1fbb176e4994ebb5d7f83c5e141a0e1bae81f672f0cb2000`.
Independent replay verifies the operational AST after narrowly stripping leaf
marks and normalizing only the exact job/rows substitutions. The original
default frame, owner setter and rows bodies are checked separately, with full
byte equality for the untouched CI/outer/accounting dependencies.

Five final integrity cases pass. The independent verifier's two earlier errors
are retained: it initially compared the entire intentionally extended caller,
then selected the property's getter rather than setter. Corrected scope still
requires the original measure AST and exact CI dependencies. Neither error was
a changed runtime predicate. xtask all-target check, strict all-target/all-feature
Clippy and Python syntax/indentation pass.

The separate [final-checks ledger](local-runs/capture-leaves-20261026/final-checks/README.md)
retains both verifier failures and all reader revisions alongside the corrected
five-case pass, doc-check, verifier syntax and local non-promotable performance
contract. Index SHA256:
`64b1ada902211b722908a13f62a05cdaeb8e9d976aa0318670c05c705fddf8d1`.

Next is a separately recorded first bounded native leaf-mode control on an
exact source/build with all existing preflights and first-result retention.
Recorder overhead needs its own on/off design; no speedup, accepted product
candidate, full-package pass or production admission follows. No new measurement,
V6, server, service, dependency or clock change occurs. Qualification SHA256
remains `11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
