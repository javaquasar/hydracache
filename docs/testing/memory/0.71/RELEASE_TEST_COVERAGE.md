# Release 0.71 memory test coverage

This document is the implementation index for the release plan's W0-W13 test targets. The
machine-readable authority is `docs/testing/release-evidence/0.71.toml`; CI executes every target
listed there in the `memory-contracts-071` job. The shared dynamic canary tests the registry and
receipt path, while the work-item targets below test the actual behavior or deferral boundary.

| Scope | Target | Release boundary proved |
| --- | --- | --- |
| W0 | `memory_baseline_071` | Baseline cohorts, corrected TTL checkpoints and immutable identity |
| W1 | `memory_footprint_071` | Coherent bounded ownership counters and complete release lifecycle |
| W2 | `memory_accounting_071`, `memory_admission_071` | Estimator correctness and atomic count/byte quota accounting |
| W3 | `retention_bounds_071` | Fixed-keyspace plateau, bounded idempotency and fail-closed audit pressure |
| W4 | `reclamation_071`, `memory_admission_071` | Repeated exact-zero reclamation, stale-load fencing and active expiry |
| W5 | `representation_071` | Public trait/estimator invariants; representation optimization remains deferred |
| W6 | `tag_index_model_071` | Tag membership agrees with a deterministic model and reclaims high fanout |
| W7 | `allocation_copy_071` | Allocation receipt schema/redaction; copy optimization remains deferred |
| W8 | `allocator_matrix_071` | Complete allocator capabilities; allocator switch remains deferred |
| W9 | `memory_profiles_071` | Disabled services own no dispatch surface; profile optimization remains deferred |
| W10 | `hc2_memory_071` | Idle HC/2 ownership and independent transport/decoded limit contracts |
| W11 | `persistence_memory_071` | Pre-allocation rejection and bounded labels; persistence optimization remains deferred |
| W12 | `memory_campaign_admission_071` | Only complete exact-candidate M10 evidence can satisfy ship admission |
| W13 | `release_governance_071` | Mandatory foundation, evidenced deferrals, no-win ship and safety-defect rejection |

The million-operation W3 plateau test is marked ignored in ordinary pull-request CI because it is a
scheduled stress test. Its shorter deterministic equivalent is mandatory on every change. W5,
W7-W11 tests deliberately verify their deferral boundaries rather than pretending that an
unqualified optimization was implemented.

Run the release-focused suite with the commands in the plan's **Mandatory test targets** table.
Before shipping, also run the exact-candidate campaign admission with `--require-ship`; ordinary
pull-request CI checks the contract without claiming that a local checkout is the final candidate.
