# Local context document stack scratch candidate

Test one storage-only hypothesis against baseline `71638d9d` and its sealed
packet: replace the temporary zeroed Vec in `Document::read_with` with private
owned stack storage for document lengths below 38. Preserve the original zeroed
Vec construction for larger lengths, including its original arithmetic. Storage
is constructed only after the first metadata check, at the old allocation site.
Every read receives exactly length+1 bytes, not the whole short array.

Keep all metadata, read, count/newline/content predicates and error propagation
in their original order; no observation caching, new origin, changed deadline,
retry or admission predicate. Exact whole-file reversal must restore the old
worker context and allocation-test file. Historical canaries must reconstruct
newly changed files completely rather than exempt their names. Test-only model
operands may replace only the two metadata observations, not any other check.

Run the nine baseline sizes and the same 256 constructions per size with the
existing allocator counter. Expected hypothesis: no Rust requests for lengths
through 37, and unchanged requests for 38 and 63. Also retain the unchanged
baseline controls, seeded full-body model differential, first-error/read-slice
schedule, local ownership, large-shape fallback and early-refusal controls.
If the allocation hypothesis fails, keep the result and revert only this leaf.

Only ordinary pure tests, source canaries, affected check/lint, formatting and
local structural gates are allowed. No ignored/manual test, original-owner/live
profile, clock/service change, server, installation, supervisor or product
workload. Verify all six prior raw seals and first refusals without modifying
their frozen reviewers. A pure allocation result cannot establish timing,
native throughput, successful material/cleanup, fault or server readiness.
