# Historical blob decoder and call site audit

The extracted Rust decoder passes eight controls on both Rust 1.94 and MSRV 1.88,
with current-toolchain all-target check and strict Clippy green. It is a pinned
protocol component, not a complete native reader or an installed optimization.
The offline flag audit narrows the earlier batching hypothesis: only 45 of 12383
captured `show` invocations combine full IDs and explicit replacement disabling.
No existing loader is redirected and no integration entry is admitted.

## Rust component boundaries

Commit `0fdb897d7fa3565eed7b842ef94886c8f8f175a4` adds
`tools/historical-blob-batch-074/decoder.rs` and the ordinary
`crates/xtask/tests/historical_blob_batch_074.rs` target. The decoder consumes a
bounded stream against ordered independent reference pins: object ID, type,
length and SHA256 of body bytes, or an exact missing-object specification. It
requires canonical headers, complete bodies/newlines and complete consumption;
errors return no validated prefix. Duplicates remain ordered and borrowed byte
slices remain exact, without UTF8/newline transformation or shared state.

The Rust component deliberately does not claim Python-reader parity. It has no
process transport, fallback/error exchange, deadline owner or Git SHA1
recomputation. Existing `sha2` supplies byte checksums without a dependency
change; synthetic OIDs and caller-created pins establish no historical identity
or trusted source provenance. A pin derived from the observed batch body is not
an independent reference. No default producer, loader or authority conversion is
provided, and the earlier Python SHA1 prototype remains unchanged.

Eight controls cover all four object kinds, empty/binary/CRLF/large bytes,
missing records and duplicates; independently modified body/pin refusal;
every truncated prefix and extra suffix; wrong OID/type/count/canonical header;
reference and size/record/header budgets; and independent concurrent observations.
They are synthetic byte/protocol controls, not real-Git delivery or original
execution evidence.

## Exact command shapes

Commit `1aefe13a07d763e6d7392c393b6634ed30d2ad70` adds the offline eligibility
reader and its four controls. It verifies all 12425 archived Trace2 members,
including the separate control, against the old packet hashes; checks the exact
retained parser; and accounts for all 12424 target root sessions. It executes no
Git command and resolves no revision. The old index remains
`11e89b85754443b1c5ff818ebfa4e5fcd2e896d812a85065cac2ffe70389e085`.

| Captured `show` shape | Invocations | Integration meaning |
| --- | ---: | --- |
| Full ID, explicit `--no-replace-objects` | 45 | Matching flag/ID shape only; no admission |
| Full ID, default replacements | 1839 | Cannot silently add replacement disabling |
| Short ID, default replacements | 10499 | Cannot silently expand to a fixed full ID or alter replacements |

The counts partition the 12383 `show` invocations exactly. They do not measure
the time attributable to each group or prove that the forty-five-call group
cannot be expensive. They do show that broad coverage cannot be inferred from
the full-ID-only replacement-disabled prototype.

## Reviewed call sites

The six-entry manual ledger stores exact source hashes, anchors, line numbers,
excerpts and individual barriers in the retained derivation. It is not an
exhaustive static call graph, and Trace2 sessions are not assigned to individual
Rust functions without additional evidence.

| Source site | Remaining barrier |
| --- | --- |
| `authority-runtime-074/gate.rs::load_frozen` | Conditional cached reads; full-ID assertion also accepts uppercase; no preplanned batch |
| `census_provider_input.rs::frozen` | Two full constants and replacement disabling, but conditional view/acceptance and error schedule |
| `performance_contract_074_closing_validation.rs::git` | Short revision, default replacements and interleaved current-source comparisons |
| `performance_contract_074_collector_notices.rs` | Fixed four-source list but short revision and different flags |
| `performance_contract_074_boot_buffer.rs::old` | Full constant in a 353-source loop but default replacements |
| `performance_contract_074_fixed_grouped.rs::frozen` | Short revision, default replacements and conditional/mutation controls |

A matching command shape does not prove unchanged current-file observations,
conditional read count/order, root/environment, error behavior or independent pin
provenance. The forty-five-call shape is an upper-level screening category, not
forty-five approved substitutions. All six reviewed entries remain unadmitted.

## Gates and evidence

All ten local check phases exit zero with unchanged captured source and HEAD:

- Rust 1.94 targeted decoder tests: 8 passed;
- Rust 1.88 targeted decoder tests: the same 8 passed;
- Rust 1.94 `check -p xtask --all-targets --offline --locked`;
- Rust 1.94 `clippy -p xtask --all-targets --all-features --offline --locked -- -D warnings`;
- four Python eligibility controls, then the combined 24-control Python suite;
- the sole offline eligibility derivation, syntax, indentation and focused Rust formatting checks.

The [sealed packet](local-runs/historical-blob-rust-audit-20261024/index.json)
contains 57 logical members in 40 content-addressed objects. Index SHA256 is
`8ba258582c8be5aed238c835f5e1040f1a7a7d0de5743d45235b68bf82d191de`.
It retains exact sources, the six audited files, raw phase inputs/results/logs,
derivation and coordinators. Raw CRLF evidence is retained without normalization.

The next gate is a bounded native transport and independent real-Git/Python
differential proof, together with explicit treatment of default-replacement and
short-ID semantics. Neither the decoder's asserted pins nor matching argv shape
can waive those gates. Do not integrate the narrow prototype as the presumed
solution to the full-package timeout or claim a CPU/memory/throughput gain.

The old interrupted package remains incomplete; its unstarted targets are not
filled by this new target. No old guard, dependency, qualification digest,
ReservedLive observation or original 300/60/10-second window changes. Server,
services, clocks, original-owner attempts and product workloads are untouched;
there is no installation, new measurement, qualification or push action.
