# Expected journal name ownership: paired local control

This packet tests one change in private Directory::inventory: identical name
formatting into a 23-byte stack Cursor instead of temporary owned Strings.
The observed BTreeSet, live scan, owned opening/publication names, guard order,
metadata/read checks and original 300/60/10 windows remain unchanged.

## Sources and method

The hypothesis is [preregistered](../../authority-expected-name-plan-20261021.md)
in `c5a427f313ba6ae853192b462cff771c6208766f`. Recipes are frozen in `4c4e5165`.
Controls/Before source is `fe387e897e5ab56397187088d8fdcdc7396c4842`; Before
evidence is retained in `3d763f8fa028aeb303c6d297d3dbdd681ea5d8fe` before runtime
mutation. The initial authoring refusal, original check recipe/log/receipt,
explicit historical-template repair and formatted proposal canary are retained.
No old assertion or measured invocation is rewritten.

Runtime/After source is `be357d1073a6dc0cfccfc50437dc253311e1d847`: only the
inventory comparator and local candidate flag change in that commit.

The manual pure comparator has empty, healthy 1/7/128-generation and invalid
first-name depth-7 cases, prebuilt observed sets, 32 warmups, three alternating
1024-operation samples and two frozen arms. It measures successful Rust
allocation/reallocation requests and requested bytes, not timing. There is no
synthetic actual arm: the unchanged filesystem error-class suite and full
eight-case fixture inspect control exercise real runtime behavior. Complete
inspection must conserve twice the pure comparator reduction at depths 1/7.
Separate non-allocator I/O/tracked-kernel observations must stay equal.

GNU 1.94 and MSRV 1.88 each use exactly one Before and one After invocation.
`check.ps1` records non-overwrite focused tests, all-target checks, strict
all-feature lint, historical assertions and repository checks. The distinctly
named `check-repair.ps1` retains the original premeasurement refusal.
`profile.ps1` binds source, six input blobs/normalized digests and one test
image hash before/after each invocation; it refuses dirty relevant source,
ambiguous images, failed checks and overwriting observations. Recipes operate
from this repository and are evidence, not a command to overwrite this packet.

`review.ps1 -Mode WriteSummary` reconstructs facts from native logs;
`final-check.ps1` checks frozen workload/reference/qualification inputs and
repository gates. Review compares JSON structurally, rejecting eight value,
type, field and array mutations plus ten summary scope mutations. Seal is
write-once and rehashes thirteen preceding packets and their complete member
inventories without changing them. `review.ps1 -Mode Review` is read-only.

## Local outcome

All three samples on both compilers agree (per operation):

| Operation | Allocations Before to After | Requested bytes Before to After |
| --- | ---: | ---: |
| Pure healthy comparator, 1 generation | 2 to 0 | 44 to 0 |
| Pure healthy comparator, 7 generations | 14 to 0 | 308 to 0 |
| Pure healthy comparator, 128 generations | 256 to 0 | 5632 to 0 |
| Complete fixture inspect, 1 generation | 29 to 25 | 1806 to 1718 |
| Complete fixture inspect, 7 generations | 129 to 101 | 9116 to 8500 |

Reallocations stay zero. Empty comparison stays zero; the invalid first-name
control changes 2 allocations/44 requested bytes to zero in the pure comparator
only. Complete inspection saves exactly twice its healthy isolated comparator
cost: 4 allocations/88 bytes at depth one, 28/616 at seven. Read requests stay
4/28 per complete inspect; all six non-authority cases and tracked kernel counts
stay equal. The reference, proposal, workload and qualification input blobs stay
identical across phases. No measured invocation is retried or overwritten.

Before and After lanes each pass 196 authority tests, ancillary controls,
all-target checks and strict all-feature clippy. All 375 historical/current
assertions and repository format/docs/registry/local performance gates pass.
The candidate is accepted locally for Rust request reduction only.

Before boot is `157c7ccf-ae63-49f2-a26e-049ba9f36556`; After boot is
`2445eee3-28d9-4c4a-8d3b-39a19eb01467`. This is not process continuity, a
clock-order repair or a new admission receipt. Original live refusals remain.

## Changed files

- Runtime: `tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs`.
- Instrumentation: allocator hook plus `tests/support/authority_expected_name_controls.rs`
  and `authority_expected_name_alloc.rs` in that tool package.
- Contracts: `crates/xtask/tests/authority_expected_name_074.rs`, its exact guard
  and old/new/gate templates, and the chained name-capacity guard.
- Registry/control: `docs/testing/gated-test-registry.toml` and
  `docs/testing/performance/0.74/authority-expected-name-contract.toml`.
- Evidence/documentation: this packet, its scoped `.gitattributes`, preregistered
  plan, performance README, readiness intermediate plan and optimization article.

## Scope

This is local nonroot fixture/request attribution, not full ActiveAuthority,
fixed production ancestry, cache API, CPU, latency, retained memory, copied
bytes, all syscalls, stable clocks, original delivery or a server admission.
Original live-clock refusals and independent execution/lifecycle/trust gates
remain. No new server/live matrix, installation, supervisor, clock/service,
product, qualification or push operation is authorized by this result.
