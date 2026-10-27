# Review of the eleven Windows workspace advisories

This is a security dependency review, not release admission or a performance
experiment. The first workspace refusal at `2fe613bf866f31c1800addb10790d350656b1790`
and its eleven findings remain in the
[Windows result](windows-fixed-cargo-result-20261027.md). Review starts from
`685e2c0b5b3e5808aeabe2a6712657384c42f492`. No new hypothesis, server operation,
service/clock change or product workload is introduced.

## Scope and policy

All eleven findings were checked against the RustSec advisory bodies, the actual
locked dependency paths, primary crates.io release metadata and relevant source.
The clean database observed before this review is
`99670cfc0bc574433034b75876a4b791ccaa2efb`. This does not retroactively pin the
earlier workspace lookup. Reachability observations below are source/feature
reviews, not exclusions: all-target dependency policy remains mandatory.

Rust remains 1.88; deny.toml, its existing exceptions and target coverage remain
byte-exact. The source candidate contains one semver-compatible lock update.
Major dependency migrations that also change parser, D-Bus or web-asset behavior
are separately identified; their availability is not a compatibility result.

## Findings and decisions

| Advisory | Actual path / exposure reviewed | Remediation and decision |
| --- | --- | --- |
| [2024-0449](https://rustsec.org/advisories/RUSTSEC-2024-0449.html) | xtask → serde_yaml 0.9.34 → unsafe-libyaml 0.2.11. Used to parse CI topology and release-governance YAML; malformed or differently interpreted workflow input matters to policy. | Unmaintained, no patched release. serde_yaml_ng 0.10 still uses unsafe-libyaml. Current serde-saphyr 1.2/1.3/2 requires Rust 1.89. Older alternatives require an explicit parser-semantic migration, including mappings, tags, aliases, duplicate/error and serialization behavior. Remains blocked. |
| [2026-0370](https://rustsec.org/advisories/RUSTSEC-2026-0370.html) | Linux supervisor → zbus 5.19 → zvariant 5.15. Sensitive original manager/FD/owner serialization path. | Unmaintained standalone crate, no patched version; moved into zbus 6 wire. zbus 6.0/6.1 requires Rust 1.89. No MSRV override, target removal or transport replacement applied. |
| [2026-0371](https://rustsec.org/advisories/RUSTSEC-2026-0371.html) | Same Linux transport → zbus_names 4.3.4. Manager/object addressing must remain exact. | Moved into zbus 6 names; same Rust 1.89 conflict. A newer 4.x is not a fix for an all-version unmaintained advisory. Remains blocked. |
| [2026-0372](https://rustsec.org/advisories/RUSTSEC-2026-0372.html) | Same transport → zvariant_derive 5.15. Macro-generated wire behavior is part of the affected graph even on a Windows policy run. | Derives moved into zbus 6; same MSRV conflict. No disabling Linux compilation to hide this finding. Remains blocked. |
| [2026-0367](https://rustsec.org/advisories/RUSTSEC-2026-0367.html) | sandbox → utoipa-swagger-ui 9.0.2 build → zip 3.0.0. Extraction permissions can preserve privileged bits. Build-time archive input, not a runtime cache command. | Patched zip ≥9. Swagger 9 requires zip ^3; the published route is Swagger 10.0.2, Rust 1.88. It also changes asset version 5.17.14→5.32.6 and removes rust-embed in favor of a new embedder. This is an available migration, not a compatible lock update; remains open. |
| [2026-0368](https://rustsec.org/advisories/RUSTSEC-2026-0368.html) | Same ZIP branch; library extraction may create a symlink outside the destination. The reviewed Swagger build script instead iterates entries, calls enclosed_name and writes dist files itself. | No blanket claim of safety from that partial call-site observation. Patched zip ≥9, same Swagger migration and hostile-archive/asset/HTTP checks required. Remains open. |
| [2026-0369](https://rustsec.org/advisories/RUSTSEC-2026-0369.html) | Same ZIP branch; chmod may follow a symlink. Swagger's own Unix extraction also passes entry unix_mode to set_permissions after writing. | Merely obtaining a green library version would not prove the custom extraction path safe. Review its modes, preexisting output/symlink custody, download source and clean OUT_DIR semantics separately during migration. Remains open. |
| [2026-0391](https://rustsec.org/advisories/RUSTSEC-2026-0391.html) | zstd 0.13.3 through both supervisor and production chitchat 0.11.1. Vulnerability involves reused borrowed streaming contexts. Supervisor uses fresh Encoder::new; chitchat serialization uses bulk compress/decompress. No matching with_context use found in reviewed paths. | Patched zstd ≥0.14.3. Latest reviewed chitchat 0.11.2/0.12/0.13 still requires ^0.13; a direct supervisor bump would leave the vulnerable version in the graph. No unsupported downgrade or substitute policy exclusion. Requires upstream/backport migration; remains open. |
| [2026-0398](https://rustsec.org/advisories/RUSTSEC-2026-0398.html) | zstd-safe 7.2.4; unsafe seekable offset/Vec length handling. Actual feature tree does not enable seekable. | Patched ≥8.2.0. A 7.3 update is not in the advisory's patched range. Safe 8 belongs to the zstd 0.14 graph, with the same chitchat constraint. Remains open; absence of the feature is not a deny exception. |
| [2026-0403](https://github.com/RustSec/advisory-db/blob/main/crates/zstd-safe/RUSTSEC-2026-0403.md) | zstd-safe 7.2.4 legacy-stream error can expose uninitialized output through write::Decoder. legacy is enabled. Reviewed supervisor archive tests use read::Decoder and chitchat uses bulk; the advisory distinguishes these APIs from the affected writer. | Patched ≥8.2.0. Disabling legacy would change supported data rather than repair this dependency frontier. No such change applied. Same upstream/backport issue remains. |
| [2026-0410](https://rustsec.org/advisories/RUSTSEC-2026-0410.html) | zstd-sys 2.0.16 wasm32 allocator shim mishandles failed/overflowed allocation and free(NULL). Native Windows runs do not exercise wasm32, but the locked all-target graph is affected. | **Applied** minimum patched 2.1.2+zstd.1.5.7, Rust 1.64 upstream minimum. Version/checksum are the only Cargo.lock edits. This fixes this dependency finding, not the other zstd-family findings. |

## Applied candidate and source/effect review

Old normalized lock SHA-256:
`e385c45d47050299da7e79a5133d5ec71ca3ea77f6bafbc36744968dff2d28b5`.
New normalized lock:
`50ec8e7e20a8caebeefdbdde7bbafb36b96257daf1204dcd6dfa8f6c5b7c925a`.
The new crate archive checksum is
`481ada9c543b2323cbb7bbdbb5249177fbf0188ed959ed2c15e4b22421c84ed3`,
verified against the primary sparse index before Cargo built it. Manifests,
qualification identity and deny.toml are unchanged. No fork or patch override
is introduced, and no unrelated resolver update occurred.

The reviewed source subset contains nine changed text members; the full archive
inventory separately records license/manifest and newly packaged CMake inputs.
All 90 bundled Zstd 1.5.7 C/header/assembly members are unchanged. Rust changes include checked allocation size
and Layout, allocation-null return and null-free handling; pregenerated bindings
are regenerated. Build changes add optional CMake/vendored support and symbol
visibility configuration. Bindgen leaves upstream defaults and changes optional
version; neither bindgen nor CMake is activated in our actual graph. The build
continues compiling the bundled C library through cc with the existing feature
set. Upstream SPDX changes to BSD-3-Clause, already permitted by the unchanged
license policy. No new tool, directory/service authority or network action is
added to HydraCache runtime. Cargo fetch and advisory-update effects belong only
to the explicitly recorded local build/policy steps.

The Windows source reviewer has a distinct exact lock successor. It verifies
the complete new digest, one exact version/checksum stanza and the complete
reconstructed predecessor digest, then changes only the independently reviewed
Cargo.lock entry in a copy of the inherited source catalog. All 394 Rust bodies,
58 auxiliary members, module edges, 156-target census and environment checks
still pass through the original reviewer. Unrelated lock edits cannot receive a
blanket pin refresh. Historical Rust lock adapters and old receipts are not
rewritten to pretend an old package result covers this candidate.

## Validation ledger

The baseline's four archive tests passed before any dependency change. On the
updated graph, four archive tests also pass, but their first broad source frame
records edits to prepare.py/dispatch.py made during that run. This first result
is retained as **source-drift**, not a clean command acceptance. The affected
package check and all-target/all-feature Clippy completed with stable source
frames. A Rust 1.88 check of zstd-sys passes. After the helper changes were
complete, a distinct stable-source archive command passed all four cases; it
does not overwrite the first drift result. The complete helper cohort passes
49 cases, including seven exact-lock mutation controls. Five additional native
semantic controls compile with Rust 1.88 and pass on the checksum-reviewed
upstream shim: allocator refusal, addition/Layout overflow, calloc multiplication
overflow, free(NULL), and successful aligned/zeroed allocation/free. These are
Windows/native results, not a wasm32 end-to-end exploit test or a complete Linux
lifecycle gate. The generated harness and its exact source are retained as data.

The fixed workspace entry ran **once** on
`58a994a4c79fd5e8dd2fa622bcd57bbf640ff2df`, preserving the 4800-second
envelope, original stage ordering, 16 GiB initial/4 GiB running floors and
five-second retirement. Eleven stages completed: doc-check, management/performance
contracts, release feature leak, format, the five preceding Clippy configurations
and dependency bans. Dependency policy then failed with the ten remaining IDs
listed above: **six vulnerability and four unmaintained findings**. Licenses,
bans and sources report success. Nine subsequent stages, including workspace
tests, rustdoc and performance/console gates, remain unstarted. Zero reported
test failures here does not mean the workspace tests passed.

Execution exits naturally with code 1, not a timeout or parser failure. The
original Job retires with no cleanup errors. Source/refs/configuration/toolchain/
environment frames match exactly. Lowest sampled physical free space is
75,616,149,504 bytes; a sample is not a capacity reservation. The clean advisory
DB has the same revision before review and after the command; no immutable
per-lookup database pin is claimed. The old green package receipt does not cover
this new lock. Release and server admission remain **false**.

The [retained packet](local-runs/security-review-20261028/index.json) contains
245 named references to 154 content-addressed objects: all eleven advisory
bodies, primary sparse metadata, reviewed upstream manifests/build/allocator
source, complete graph/feature comparison, baseline and first-drift records,
focused commands, controls, and the actual full workspace streams and ledger.
Index SHA-256:
`ec37fd3b0087032b44a417360dc7531e6251ee2e36e61a868283aaca9a76f1be`.
Independent inert verification checks every object, source/helper provenance,
the exact lock delta, request/receipt identity and eleven-complete/one-failed/
nine-unstarted accounting. It neither imports archived code nor repeats Cargo.

## Remaining decisions

Closing every finding under the unchanged MSRV requires more than compatible
published version updates: decide a maintained Rust-1.88-compatible D-Bus path
or audited backport strategy, migrate the YAML parser with policy-semantic
controls, migrate Swagger/ZIP with extraction and HTTP/asset checks, and obtain
a corrected chitchat compression branch. None is accepted here merely because
its replacement compiles. After those separate gates, rerun the complete package
and workspace milestones and Linux lifecycle/recovery gates before host admission.
