# D-Bus source backport: actual Rust 1.88 boundary

This continues the approved security migration after the preview candidate was
declined. It prepares local zbus 6 source; **no HydraCache dependency or caller is
changed**. No server, bus/manager connection, services, clocks, installation or
product workload is used. All live predicates/order/frequency and original
300/60/10, package 780, workspace 4800 and local 1200/4-GiB/5-second contracts
remain unchanged. No performance hypothesis or qualification is started.

## What is actually fixed

Fresh primary sparse metadata and checksum-reviewed archives confirm zbus and
zbus_macros 6.1.0 and zbus_utils 6.0.0 declare Rust 1.89; zruntime 0.2.0 declares
1.88. The three standalone crates named by
[0370](https://rustsec.org/advisories/RUSTSEC-2026-0370.html),
[0371](https://rustsec.org/advisories/RUSTSEC-2026-0371.html) and
[0372](https://rustsec.org/advisories/RUSTSEC-2026-0372.html) are unmaintained and
have no patched release. They are already merged in zbus 6. No exception or
renaming of those old packages is used to mask the findings.

| Frozen input | Change / first outcome |
| --- | --- |
| U | Published source/declarations, explicit tokio+proxy, defaults off. Actual Cargo/Rust 1.88 refuses zbus/macros/utils and newly resolved UUID 1.29.0. |
| M | Only the three floor declarations become 1.88. Actual compiler still refuses UUID. Editing declarations is insufficient. |
| P | Only isolated UUID lock identity becomes the product's existing 1.23.4. Compiler reaches source and refuses two E0716 errors in MatchRule Display. |
| A | Original upstream source/declarations with P's compatible graph and the added match-rule regression. Five tests pass on 1.94 before Q is prepared. |
| Q | Pass format_args directly into both formatting helper calls, preserving the rest of P. Actual 1.88 build, five tests, check and strict Clippy pass. |
| F/F2 | Inert Rustfmt output decoding refuses before source freeze: first child/parent headers, then final argument commas. Both first refusals and the single projection remain retained. |
| F3 | Corrected exact section/known-call projection, formatting only beyond Q. Build, five tests, check and strict Clippy pass on both 1.88 and 1.94. Scoped dependency policy passes. |

Rust 1.88 cannot retain the temporary backing `let key = format_args!(...)`
until the next statement. Passing the arguments directly extends their use to
the containing call without allocating a String. The two sites format indexed
argument and argument-path keys. The common regression requires exactly
`arg0='it'\''s',arg1='a,b',arg2path='/org/test'` and an equal parsed rule. It passes
on the unmodified 1.94 baseline and the actual 1.88 source candidate. This is a
concrete source backport, not --ignore-rust-version or a declaration-only claim.

## Scope of the green controls

The five-case probe covers derive/wire roundtrip, invalid name/path refusal,
method/reply serial, sender and destination, wrong body signature, upstream
synchronous runtime timeout at the unchanged 250-ms method duration, nested
runtime entry refusal, and both MatchRule key branches/quoting. A nested-entry
panic is deliberately caught and asserted; it is not an unexplained test failure.
No real manager credentials, original FD handoff or service is contacted.

A/1.94, Q/1.88 and final F3/1.88+1.94 each pass five cases: 20 selected native
executions, with ten on the final source. Actual Cargo compiler-artifact paths,
source and executable SHA are bound before full nonempty five-case listings;
counts, ignored/filtered totals and native exits must agree. Compiler/source
target paths are disjoint. Full fixture/root/source/HEAD and image postflights
pass, and every owned Job retires cleanly. Three inert decoder controls and
formatting of owned Rust overlays and final patched source pass.

The isolated final graph has 52 registry nodes, 30 identities different from the
product graph. Every cache archive matches its locked checksum; archive member
bytes equal the observed checkout, and declared minima are at most 1.88. This is
a **postflight** identity inventory, not a continuous per-lookup seal or a complete
source/effect approval. Build-script inputs are inventoried; most non-Windows
target nodes are not compiled by this probe. No unsupported all-features/default
runtime or GNU claim follows from the selected tokio+proxy build.

The unchanged root deny policy passes advisories/bans/licenses/sources on this
standalone graph. Its existing unused ignore warnings remain in stderr; no ignore
is added. Advisory DB 99670cfc0bc574433034b75876a4b791ccaa2efb is observed clean in
postflight only, not asserted sealed across the command. Product policy and the
complete package/workspace command are not rerun or relabeled green.

## Caller migration boundaries remain explicit

The existing direct-use audit has 18 supervisor source files using zbus, two
blocking users and eleven old wire-alias users. This count is not a closed module
or AST proof. A real caller migration must also handle these observed changes:

- Blocking Connection/Proxy APIs disappear; root manager calls still use them.
- Message setters record validation errors for build instead of returning each
  setter error. Preserve the existing refusal/effect ordering when adapting.
- Header Flags becomes a bitflags struct with uppercase constants; old enum
  casts must not be mechanically retained or unknown-bit checks weakened.
- Deprecated zvariant aliases must be migrated under strict warnings, retaining
  owned/borrowed FD, value and body signature semantics.
- Old Tokio block_on uses new_multi_thread; zbus 6 with only tokio uses
  new_current_thread, unless tokio-multithread is explicitly selected. Connections
  and tasks need a runtime owner that preserves progress between observations.
  A per-call block_on substitution is not an accepted compatibility design.

These are retained source observations, not a demonstrated delivery/fault failure.
The probe checks neither autonomous background progress nor all-writer retirement.
No root manager, validation frame, deadline or cleanup authority has changed.

## Retained source and next gate

The [portable fragments](../../../../tools/dbus-msrv-backport-074/patches/receipt.json)
contain three normalized manifest floor changes and one Rust member change with
the two formatting calls. Inert hunk replay reconstructs all 263 original members
of the three archives; four members differ. Receipt SHA-256:
`59a5cd3f030648218b36f0f4c2f066d1d04c1c5320d059ffef5435df5ad8d2d2`.
The fixture lock is labeled as tested, not shipped. Authoring workspace manifests,
version/ownership, distribution and registry consumers still need packaging work.

The [raw evidence packet](local-runs/dbus-msrv-backport-20261028/index.json)
contains 2,754 logical references and 532 distinct raw objects. Index SHA-256:
`ab5c211321f2557437f0c93c0bf2e99b82f8dd417a68e95f84590fb8a45c109e`.
It retains primary metadata/archives, every frozen source input, original first
native and inert refusals, exact compiler/test/Job receipts and source patches.
Native images/target trees are excluded; their path, size and SHA receipts remain.

Next retain and test the runtime ownership/caller migration separately, including
background progress, original sender/receiver and reply state, Unix FD ownership,
old dispatch compatibility, malformed/partial I/O and cleanup. Then GNU/MSRV,
complete source/effect and distribution reviews must precede root adoption and a
fresh complete package/workspace gate. Chitchat's costly preview remains declined;
security closure and original execution/recovery remain open. This result grants
neither release nor server admission and makes no numerical performance claim.

The later [source-package consumer gate](dbus-source-package-result-20261028.md)
now materializes standalone authoring manifests and binds the three local source
archives, including the separately tested Linux CLOEXEC/CTRUNC fixes. Every
compiler Rust body and manifest equals the 67-case predecessor; the reconstructed
consumer passes all 67 on GNU Rust 1.94/1.88 with check/strict Clippy. The original
MSRV/formatting refusals remain. Registry distribution/actual shipping consumers,
complete source/effect/caller migration and final product admission stay open.
