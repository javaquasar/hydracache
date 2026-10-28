# Isolated YAML parser security migration

This continues the [security review](security-review-20261028.md) and
[Swagger successor](swagger-security-result-20261028.md). Production cache,
transport, original-start, enrollment and live predicates are unchanged. Rust
stays 1.88; deny.toml, qualification manifest and original 300/60/10,
780/4800-second, capacity and retirement contracts stay exact. No server,
existing service, clock, product workload or performance hypothesis is involved.

## Candidate and source effects

Only crates/xtask/Cargo.toml changes its serde_yaml dependency to the package
alias `yaml_serde` 0.10.7. Imports, caller bodies and policy predicates are
unchanged. Cargo.lock removes serde_yaml 0.9.34+deprecated and unsafe-libyaml
0.2.11 and adds yaml_serde 0.10.7 and libyaml-rs 0.3.0. Every other package
identity/checksum and effective dependency edge is preserved; xtask is the only
existing edge owner changed. The root manifest is unchanged.

The maintained successor is published by the YAML Organization; see its
[library documentation](https://docs.rs/yaml_serde/0.10.7/yaml_serde/) and
[low-level implementation](https://docs.rs/libyaml-rs/0.3.0/libyaml_rs/).
Both exact crate archives match primary registry checksums and the Cargo cache.
Declared minima are 1.82 and 1.60, respectively. Neither declares a build script.
Selected parser features are exactly default/std: standard I/O and randomized
mapping hashing remain enabled; new no_std I/O/FNV implementations are not
selected. No workflow run/uses field is executed by the review.

Full source diffs and member hashes are retained. Low-level parser, scanner,
emitter and other core source bodies are identical; documentation/lints and
dependency-only test binaries change. The Serde layer adds core/alloc/no_std
support, lifetime/style cleanups and equivalent ownership/pointer construction.
Its Error::source now exposes the immediate Io/FromUtf8 error rather than that
error's source. Actual ci_topology/release_governance callers use from_str, do not
construct these variants, and retain their original error handling. This is a
reviewed compatibility boundary, not an assertion that every API is identical.

Unsafe C2Rust code still exists. The finding being removed is the
[unmaintained-library advisory](https://rustsec.org/advisories/RUSTSEC-2024-0449.html),
not a demonstrated memory-safety defect repaired by rewriting the parser. No
blanket safety or performance claim follows from the maintained fork's name.

## Baseline, fixed corpus and first recorder failure

Before changing the root graph, all 12 CI reliability and 23 release-governance
tests pass. One identical fixture-only Rust probe is built in two separate
ignored target projects, never added to the shipping workspace or deny policy.
Each observes Value kinds, numeric accessors/bits, tags, mapping order, exact
serialization of every subtree and reparsing of the complete serialization.
Failures retain exact messages and line/column/index. Upstream source and fixture
locks are reviewed before either build.

The frozen corpus contains all 25 current workflow files, 24 fixed valid/invalid
fixtures, five depth boundaries and 48 seeded cases (`0x0741028`). Its SHA-256 is
`6e0ca0eb52841f74600a386923898f3da07b770ce7699ca853ed3618a324c78c`.
Every case has its own native process with the predeclared five-second bound;
the recorder has the existing 1200-second owned local review envelope. These
are inert semantic controls, not a release workload or benchmark.

Both first recorder invocations fail while saving case 54, seeded/0: the Windows
default cp1252 writer cannot encode an emoji. Child stdout/stderr and the
recorded exit=0/timeout=false prefix survive; partial JSON, traceback and
failed outer receipts stay unchanged. This is a recording failure, never renamed
as a parser pass or silently retried.

An independently tested inert recovery verifies original producer SHA, corpus,
binary, consecutive ordinal coverage, exact native status prefix and every raw
file hash. It recovers the 55 already executed cases per parser into a separate
UTF-8 packet. A distinct corrected continuation executes only ordinals 55–101:
47 previously unstarted cases per parser. It cannot accept a free start ordinal.
Comparison proves complete ordered coverage and no overlap. There are explicitly
two recorder invocations per parser, no successful continuous recorder invocation
and no executed-case retry. No performance lifetime or qualification suffix is
joined. Missing/unknown native status would have refused recovery.

## Results and exact source gates

Across 102 cases per parser, 89 parse and 13 refuse identically. There are zero
parse-decision, typed-value/order/tag, serialization, roundtrip or diagnostic
differences. Depths 32/64/127/128 parse; 129 refuses with the same diagnostic.
This finite corpus is not a proof for arbitrary YAML or all upstream APIs.

The real updated xtask passes the same 35 consumer cases, all-target check,
all-target/all-feature Clippy with warnings denied, and Rust 1.88 all-target
check. Eleven comparison/recording controls pass. No shipping Rust source or
default xtask target is added. Baseline/recovery tools are committed as
`2a357d6bd`; the dependency candidate is `5629014a9`.

The separately committed Windows source gate `c9eec2b3b871ad7b7178cee66b844a696b7fef69`
uses SHA-pinned inverse edits for both complete current lock and xtask manifest.
It reconstructs their complete preceding bodies, then preserves the original
Swagger and zstd-sys predecessor proofs. The root manifest's existing Swagger
proof remains required. Only these three named auxiliary entries change; the
inherited 394-source/58-auxiliary/156-target review remains complete. All old
Swagger assertions are retained against its immutable original packet, with
eight additional current-YAML controls: 66 runner cases pass. There are no
extra Git reads in these adapters and no cached live admission.

## Full milestone and remaining work

One actual fixed `cargo xtask verify` on `c9eec2b3b871ad7b7178cee66b844a696b7fef69`
completes eleven stages: document, management/performance contract, feature leak,
format, full Clippy/allocator matrix and dependency bans. Dependency policy
then refuses six findings; nine following stages remain unstarted. Licenses,
sources and bans pass. The owned Job retires without cleanup errors and the
complete source/ref/configuration/environment frames match. The original
4800-second bound, 16 GiB start/4 GiB running floor and five-second capacity/
retirement rules hold. This is a naturally completed failed gate, not a timeout.

Remaining IDs are RUSTSEC-2026-0370/0371/0372 (D-Bus crates moved into zbus 6,
whose published minimum is Rust 1.89) and RUSTSEC-2026-0391/0398/0403 (Zstd
wrappers retained by chitchat's ^0.13 requirement). These require separate
MSRV-preserving migration/backport decisions. A supervisor-only Zstd update,
hidden target exclusion or deny exception does not close them. None is applied.
The clean advisory DB head in the index is only a postflight observation, not a
claimed immutable per-lookup database pin.

Complete package/historical Rust input adapters, remaining workspace stages,
GNU semantic validation and original execution/retirement/recovery remain open.
The old 1,351-case package success does not qualify this dependency graph. There
is no merge, tag, release or server admission, and no throughput/allocation claim.

## Retained evidence

The [primary packet](local-runs/yaml-security-20261028/index.json) contains 928
logical references to 382 distinct byte objects: primary metadata/archives,
full source diffs, exact before/after inputs, all raw native cases and first
recording failures, recovery provenance, controls, focused commands, full
milestone streams/accounting/claim and current reviewed code. Index SHA-256:
`1e12021d3dcb7632e95f7cc10a17244178f3a1905af17ce8740eda1267bffcb0`.
Objects retain raw bytes across checkout. An independent inert audit verifies
all member sizes/hashes, raw native records, once-only coverage and semantic
comparison without executing archive code or replaying any native case/gate.
