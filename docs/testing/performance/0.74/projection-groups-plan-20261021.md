# Exact allocation for the bounded fresh Groups collection

Test one leaf hypothesis after `07cfb39e`: eliminate temporary group-vector
growth and excess capacity during each fresh status projection, without changing parsing,
field order, fresh reads, predicates, refusal sites, live call counts or deadlines.
The existing parser reads at most 33 integers and rejects more than 32. A private
scratch array can represent the same intermediate values, including the 33rd
value needed to preserve the budget refusal and earlier numeric errors.

Retain a pure allocator baseline before implementation. Cover lengths 0, 1, 4,
5, 16, 32 and 33, 256 calls each, with the same integer parser and Result
collection. Candidate acceptance requires identical values and first errors,
seeded differential and malformed-document tests, exact whole-source reversal,
and one exact-size allocation for each nonempty collection, with no reallocation.
The final Projection still owns the original Vec type; no caller or retained
identity representation changes. DiagnosticPeer conversion is untouched.
The temporary bounded scratch is not a fresh observation cache.

Run only ordinary unit tests, pure allocator controls, source contracts,
formatting, affected all-target checks and strict all-feature clippy on GNU
1.94/MSRV 1.88 and Windows where applicable. No ignored/manual tests, original
owner/live repetitions, clock/service changes, server, supervisor, install,
qualification or product work. Keep the original first Causal(Time) refusals
and all prior evidence sealed. Allocation results cannot establish latency,
ten-second headroom, full material readiness or a server launch permission.
