# Bounded pulldown-cmark security continuation

The first Windows milestone on `7d0f28851` refuses four findings, including new
RUSTSEC-2026-0414 on pulldown-cmark 0.13.4. Official 0.13.5 metadata retains
Rust 1.71.1 and unchanged dependency/features; root Rust stays 1.88. Review all
source changes, including the additional list/UTF-8 fixes. Build.rs changes are
test-generator gated; do not infer runtime compatibility from semver alone.

Prepare disjoint standalone baseline/candidate fixtures from the current exact
root lock. Shared Rust/corpus bytes and compiler-specific target roots are fixed
before compilation. The only dependency delta may be pulldown-cmark 0.13.4 →
0.13.5 with its official checksum; no unrelated resolver refresh is accepted.
Bind actual Cargo test artifacts and list the complete three-case inventory.

The baseline regression intentionally remains red: count parser events without
collecting them, for 1/2/4/8/16/32/64 copies of `[[a|]] ` and both wiki-only and
actual prost-build options (`all` minus smart punctuation). The one-million-event
cap predeclared in the shared test bounds CPU/memory even on the broken parser;
it is not a product threshold. Fixed expected output is 4*n+1. Preserve the entire
first baseline output, including capped observations, without retry-to-pass.

The compatibility corpus contains every tracked proto's comment blocks plus a
fixed plain/list/code/link/table/Unicode set. Record exact events and HTML under
actual consumer options, with all cases/order/bytes equal across both versions.
No shortening on mismatch. Short malformed/Unicode controls must stay bounded.
Use GNU/MSRV compilation and strict fixture Clippy before root adoption; compare
actual affected consumer build outputs/tests separately as needed.

Retain original 1200-second local review/4-GiB/five-second cleanup contracts,
strict inventories, source/HEAD/lock/artifact pre/postflight and all first results.
No clocks, services, server, product workload, benchmark or publication is involved.
No native/RESP speed or retained-memory claim follows. Chitchat S remains declined.

The completed source review and matrix are recorded in
`docs/testing/performance/0.74/markdown-security-result-20261029.md`.
Run `analyze.py` only on saved results: it binds generated output to the two actual
Cargo build-script records and compares complete case catalogs by target identity.
Parallel Cargo artifact arrival order is retained separately and is not a semantic
ordering claim. The first inert list-order comparison refusal is kept. `test_review.py`
checks missing/duplicate records, changed output/registry sources and fixed argv.
The HTML corpus probe uses parser default features; actual root metadata has no
parser features. Actual consumer builds/tests cover that root configuration. SIMD
is off in both, and the changed production parsing path has no HTML/getopts cfg.
