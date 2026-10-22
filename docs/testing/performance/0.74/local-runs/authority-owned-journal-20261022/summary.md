# First owned storage journal model observations

Both MSVC lanes agree: five cohorts, sixteen rounds, 6752 Leaf calls per arm.
Reference makes 6752 allocations/7133488 requested bytes; owned storage makes
422/445843, with separate eviction of 422 allocations/445843 bytes. At 128
generations successful retained cache capacity is 270464 bytes. Synthetic owned
Leaf layout grows 152 to 176 bytes; this is not actual Linux/RSS/copy measurement.

1152 leaf masks, 32 modeled journal gates, 582 metadata/IO/content cases, logical
append/reopen/operator-state and drop/thread-affinity controls pass. Full target
traces and terminal healthy traces match. First 1.94 reference clippy refusal
remains red; current equivalent parity lint repair passes, no measured retry.
See development-note.md for all source/measurement distinctions and limitations.

Twenty previous seals/1111 members retain original windows/refusals and gates.
Keep this model unpromoted; audit the exact actual-engine patch and historical
source-guard boundary before runtime mutation. No WSL, clocks/services, install,
supervisor, server, product workload, real fault/reconnect or admission result.

Source review leaves an operator-helper error-priority gap: logical transition
guards can precede failed-owner refusal, unlike the actual engine's change path.
Covered state examples do not prove exact operator control-flow equivalence.
