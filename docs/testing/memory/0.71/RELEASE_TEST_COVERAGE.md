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
| W12 | `memory_campaign_admission_071` | Only an identity-sealed exact-candidate M3/M8/M9/M10 chain can satisfy ship admission |
| W13 | `release_governance_071`, `test_memory_release_claims_071.py` | Mandatory foundation, evidenced deferrals, no-win ship, exact D4 claim identity and safety-defect rejection |

The million-operation W3 plateau test is marked ignored in ordinary pull-request CI because it is a
scheduled stress test. Its shorter deterministic equivalent is mandatory on every change. W5,
W7-W11 tests deliberately verify their deferral boundaries rather than pretending that an
unqualified optimization was implemented.

Run the release-focused suite with the commands in the plan's **Mandatory test targets** table.
Before shipping, also run the exact-candidate campaign admission with `--require-ship`; ordinary
pull-request CI checks the contract without claiming that a local checkout is the final candidate.
Before M10 begins, its protected workflow runs `scripts/perf/memory_compat_071.sh` against the real
`v0.70.0` and exact-candidate binaries. The resulting sealed `compatibility-receipt.json` is part of
the M10 artifact and mandatory for final campaign admission; a compiling but skipped process test
does not count as S8 ship evidence.

After the accepted M3/M8/M9/M10 artifacts have been extracted, generate the no-win claim set
with `scripts/perf/memory_release_claims_071.py` using the verified evidence-branch checkout
and the extracted campaign directory. The generator checks the reviewed
[`d4-acceptance.json`](d4-acceptance.json) against the branch manifest and all four complete
receipts. `release-evidence --release 0.71 --require-ship` then consumes the generated
`target/memory-evidence/0.71/release-claims.json`, rejects numerical claims, checks the exact
campaign chain again, and requires the measured SHA to be an ancestor of the review commit.
The release tag itself must still point to the measured SHA, not to later documentation/tooling
commits.
