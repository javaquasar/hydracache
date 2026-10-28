# Isolated Swagger/ZIP security migration

This continues the [eleven-advisory review](security-review-20261028.md), whose
sys-only successor left ten findings. It changes the sandbox documentation UI,
not RESP/native execution, production enrollment or original-start. Rust stays
1.88; deny.toml, qualification manifest, all existing live checks and original
300/60/10, 780/4800-second and capacity/retirement contracts are unchanged.
There is no new performance hypothesis, server operation, service/clock change,
product workload, merge, tag or release admission.

## Candidate and compatibility boundary

The root dependency moves utoipa-swagger-ui 9.0.2 to 10.0.2, with default
features disabled and only axum/vendored selected. The lock selects zip 9.0.0,
the minimum patched version for
[0367](https://rustsec.org/advisories/RUSTSEC-2026-0367.html),
[0368](https://rustsec.org/advisories/RUSTSEC-2026-0368.html) and
[0369](https://rustsec.org/advisories/RUSTSEC-2026-0369.html).
The exact registry-vendored UI is 5.32.6, replacing the old 5.17.14 UI. This is
an intentional static documentation asset update, not a cache protocol change.

Swagger 10 also depends on utoipa 6. The application deliberately keeps its
existing utoipa 5.5.0 generator and macros. An explicitly cached typed document
now serves the same public `/swagger-ui/openapi.json`; Swagger's
configuration points to that path instead of taking a differently versioned
OpenApi Rust object. No unchecked JSON bridge, schema regeneration, new
deserialization or fallback document is used. `/openapi.json` and sandbox data
retain their existing token guard; Swagger HTML/assets/document remain public
as before. Existing paths, JSON type, headers, method and traversal refusals
are covered by before/after HTTP tests.

Final review also restores Swagger 9's document ownership: one generated
utoipa 5 document is retained in an Arc and serialized through a typed Json
response. The first migration reused the ordinary `openapi()` handler, which
would regenerate the document per request. That earlier source's complete
milestone remains separate; the Arc-preserving successor has its own four HTTP
cases, check/Clippy and Rust 1.88 all-target checks. All pass with stable source
frames. No measured allocation or latency improvement is claimed. The ordinary
protected `/openapi.json` handler is unchanged.

The complete OpenAPI response is byte-identical before/after: 117,641 bytes,
SHA-256 `989c92b97e9c4475792864fd073664a6ac81cf2f246f38492412636fa2768414`,
still schema 3.1.0. No wire/durable artifact version or COMPAT entry changes.
This is a compatibility observation, not a performance result or a browser UI
functional certification.

## Primary source and effect review

New locked packages are base64 0.23.1, typed-path 0.12.4, utoipa 6.0.0,
utoipa-gen 6.0.1, utoipa-swagger-ui 10.0.2, utoipa-swagger-ui-vendored 0.2.0
and zip 9.0.0. Every archive checksum matches primary sparse metadata and the
actual Cargo cache; every declared minimum is at most Rust 1.88. The old Swagger,
ZIP, rust-embed family and derive_arbitrary disappear. Existing effective
dependency identities are preserved except the declared sandbox UI replacement
and removal of arbitrary's unused derive edge formerly supplied by ZIP. Fuzz
sources do not use that derive. Normal base64 0.22.1 consumers and the existing
application utoipa/derive pair remain on their original versions.

The new utoipa-gen uses syn 3 already present in the graph; it does not regenerate
the application's existing schema. typed-path is a lexical path library with no
build script. The added libraries declare no build scripts; Swagger's reviewed
build.rs remains the asset processor. Licenses are MIT/Apache-compatible with
the unchanged policy. Normal registry fetches occur explicitly before build;
the UI build uses the registry-checked vendored crate instead of the upstream
HTTP/curl downloader. The selected features do not enable gzip, minified, cache,
url or reqwest. Inherited SWAGGER_UI_* build overrides refuse before the fixed
Cargo dispatch, preserving the reviewed asset path.

The exact vendored crate checksum is
`552a64d6a0d251ae28bc021838800943cce60c9ec2e60ff331f54e79f696268f`;
its original ZIP checksum is
`b3c07e091559b59a833f66547eb1fc18f2896f96e3f1f953e2f1efb328aa3394`.
The independent Python reviewer reads all 711 entries/CRCs (6,284,256 expanded
bytes) without extracting anything. No symlink, special file, privileged mode,
duplicate/case alias, traversal or file/parent conflict is found; required dist
assets exist. Nine mutation controls check the reviewer's refusal paths. Its
32 MiB archive/64 MiB expanded/16 MiB file/10,000-entry review bounds are declared
before candidate review and are not product workload or release thresholds.

The upstream custom extractor still writes dist files and forwards Unix modes.
This review accepts one authenticated, fixed archive with safe entries; it does
not certify arbitrary ZIP input or a hostile preexisting OUT_DIR. No archive is
extracted by our reviewer, and no host installation path is touched. A different
vendored archive requires new source/input review, not an automatic pin refresh.

## Baseline, first refusals and corrected graph

Four HTTP integration tests are introduced before the product change. They pass
on the sys-only worktree at `66f05b21f3ee58e0eb414ae8df5d5910278bba68` plus the
new test file; that test-only source is committed as
`eaebf65b9bb6a7250bc35fcd1e361a33f9848073`. Baseline all-target check and
all-feature/all-target Clippy pass. No baseline dependency update occurs.

The first metadata probe retained four successful primary responses and then
failed because its ZIP URL incorrectly used the four-character path rule for a
three-character name. Successful bodies are reused as data; the already saved
primary ZIP catalog supplies the correct entry. No compiler or workload retry
is used to disguise that preparation failure.

Initial resolution selected ZIP 9.1.3. Selecting the reviewed minimum 9.0.0 removed
nix and selected typed-path 0.12.4, but also changed seven unrelated parents'
Windows dependencies and tempfile's getrandom edge. The effective-graph audit
refused this. Its HTTP/check commands had passed, but that graph is **not
accepted**. Its not-yet-started Clippy/MSRV commands refuse before dispatch.
The original results remain intact.

Only the unrelated edges are restored to their baseline identities. The corrected
graph passes `cargo metadata --locked --offline` without modification and the
independent full graph/primary checksum review. It then passes a distinct four-case
HTTP command, all-target check, strict Clippy and full sandbox all-target check
with Rust 1.88. First outcomes are not overwritten or joined into acceptance.
Two offline inverse-rule generation assertions comparing CRLF raw hashes with LF
hashes are separately retained as explicitly transcribed utility failures. Both
manifest hashes now use the inherited review's unchanged LF normalization; no
live command is replayed for those corrections.

## Exact Windows source successor

Normalized lock SHA:
`00efd4a2cafeb5e82d757cc4855e440c627211c1544c22dcbf8c875f85337cd8`.
Normalized manifest SHA:
`fd57accedc5029f0024c45e72f3c3ef66501d670eaa8776cf293553ff8d5a1ad`.
The manifest's raw Windows CRLF hash is separately
`a494481b85fa84fd07e19a447c6d5fe7a49aa12557e0c0befc86fdea390d3bf2`.

SHA-pinned rules (`868507587efb67519060d05854dcc44dd2de4395e4d9d478f40cfc442580c517`)
contain 37 exact lock inverse edits and one manifest edit. They verify complete
current inputs, ordered edits and complete predecessors. The preceding sys
inverse proof still runs on the reconstructed lock. Only those two auxiliary
entries change in a copy of the inherited catalog; all 394 Rust bodies,
58 auxiliary entries, module edges and 156 xtask targets retain the original
review. There are no extra Git reads in the new guard. Historical sys tests use
their immutable exact packet fixture; all old refusal assertions remain, with
separate tests for current inputs. All 58 runner controls and the source/environment
review pass. Old execution receipts are not reinterpreted as a package pass for
this new dependency graph.

## Milestone and remaining work

The first original workspace command on clean source
`836800c836580a26f7fa0beb4eaeab41ff764d58` completes eleven stages and then
refuses seven remaining advisories. All three ZIP findings disappear; the nine
following stages remain unstarted. Its frame postflight and owned retirement
pass. This is the pre-Arc-preservation source, not an interchangeable suffix for
the successor.

The final original command runs once on the distinct committed Arc successor
`11c17e7cfc68594e380d87028d43d1a7942a7dcb`. It independently completes all
eleven preceding stages, then exits naturally with dependency policy code 1:
the same **seven** IDs remain (three vulnerabilities and four unmaintained
findings). ZIP 0367/0368/0369 are absent. Licenses, sources and bans pass.
Nine subsequent stages stay unstarted, including workspace/xtask tests, rustdoc,
performance contract and management console. No previous suffix or test count
fills those stages. The original Job retires, no cleanup error or timeout occurs,
and complete source/ref/configuration/toolchain/environment frames match.

The original 4800-second envelope, 16 GiB initial/4 GiB running floors,
five-second capacity cadence/retirement and fixed unfiltered command remain.
Postflight observes clean advisory DB
`99670cfc0bc574433034b75876a4b791ccaa2efb`, the same revision as the preceding
sys report; this is an observation, not a retroactive preflight or immutable
per-lookup pin. Compiler/version/binary probes independently confirm installed
Rust/Cargo 1.88.0 and 1.94.0 without auto-install or build replay.

The [retained packet](local-runs/swagger-security-20261028/index.json) has 396
named references/199 objects, including both complete workspace streams/ledgers,
all baseline/refusal/corrected/cached commands, four identical full HTTP documents,
new crate archives and primary metadata, asset inventory, inverse rules and source.
Index SHA-256:
`d645f31d20591bbd57a4e31744f77270165a0da9765658d240bd5f11f51855a0`.
Independent verification checks every object, Git helper/product provenance,
requests, limits, ledgers, seven actual IDs, all new archive checksums and the
embedded ZIP/CRCs without importing archived project code or repeating Cargo.

Focused Windows/MSRV compatibility does not establish a complete package,
Linux/browser gate, server rehearsal or production admission. Existing Rust
historical-input adapters still need exact successors for the final dependency
graph before their old package contracts can be reused; no such old package
receipt is claimed here. D-Bus/MSRV, YAML and chitchat/Zstd security blockers
remain separate. No other dependency migration or performance candidate is mixed
into this step. Release and server admission remain **false**.
