# Remaining D-Bus/Zstd security boundaries

This follows the [YAML migration](yaml-security-result-20261028.md). Root
manifests, lock, deny policy and qualification identity are unchanged. Rust stays
1.88; all live checks and original 300/60/10, package/workspace, local review,
capacity and retirement limits are preserved. There are no server, service,
clock, installation, product workload or new performance experiments.

The last complete product workspace command on `c9eec2b3b` refused six findings.
That graph is unchanged here; dependency policy is not rerun or relabeled green.
The present work is isolated dependency-migration feasibility, not integration.

## Fresh primary sources and concrete candidate

Fresh sparse metadata still shows chitchat 0.11.1/0.11.2/0.12/0.13 depending on
Zstd ^0.13. The maintained
[upstream manifest](https://github.com/quickwit-oss/chitchat/blob/main/chitchat/Cargo.toml)
also retains it. Patched Zstd 0.14.3 requires zstd-safe 8.2.0; both declare Rust
1.64. This cannot be solved by changing the supervisor's direct dependency alone.

Checksum-reviewed registry chitchat 0.11.1 is copied into standalone ignored
fixture projects. Both receive identical private test overlays and serde_json;
only the candidate's requirement changes from Zstd 0.13 to 0.14.3. Every upstream
Rust body is retained. The exact manifest delta is stored as
`capture/preparation/candidate.diff` in the packet below. No path patch,
replacement package, registry publication or root dependency change is applied.

Both resolved fixture graphs have 92 packages. Only Zstd 0.13.3/zstd-safe 7.2.4
move to 0.14.3/8.2.0; chitchat is the only existing changed edge owner. The
zstd-sys 2.1.2+zstd.1.5.7 identity remains unchanged. New archives match primary
metadata and the Cargo cache; complete wrapper source diffs are retained.
zstd-safe's build script only selects std for wasm32/hermit, as before. Optional
new CMake/allocator features are not deliberately enabled by this candidate.
The selected published package still needs a complete shipping effect review.

## First results, without retry-to-pass

The first metadata preflight refuses before compilation: the test overlay
duplicated package/workspace resolver declarations. Its input, streams and
refusal remain immutable. Separate v2 fixtures remove only the shared duplicate
package resolver and retain workspace resolver 2. The pair still differs only
in its Zstd requirement.

Both fixture libraries compile. Their original pure unit groups pass separately:
10 serialize, nine delta, four message and two digest cases, 25 per fixture.
The existing property test uses the recorded PROPTEST_RNG_SEED 7606312. Existing
rand-based tests are unit controls, not identical generated benchmark workloads.
No gossip/socket test group is executed. Both all-target fixture checks pass.

The additional deterministic wide snapshot fails its original
`encoded.len() <= upperbound` assertion on both baseline and candidate. It emits
no complete comparison snapshot. The planned 37-stream/seven-malformed matrix
therefore **has not passed or completed**. There is no byte-compatibility or
complete malformed-input claim. The first failed test, its source and unchanged
bounds remain retained; a smaller corpus does not replace it.

Offline source inspection explains a possible mechanism: upperbound_after
reserves metadata for at most two new blocks, while append may split a large
serialized item across more blocks. Extra per-block metadata can exceed that
estimate when compression falls back to uncompressed blocks. This is a
source-derived attribution, not a separately executed minimal counterexample.
DeltaSerializer uses a threshold min(16,384, mtu), so the small-threshold probe
is not by itself proof of a HydraCache production exploit. Actual supported-MTU/
operation reachability still needs a focused regression and cannot be dismissed.
No MTU/backpressure threshold or rejection rule is changed here.

Strict library Clippy also refuses on both inputs, with the same two existing
mismatched-lifetime-syntaxes errors: KeyChangeEvent and DeltaOpRef omit explicit
`'_` lifetimes. No lint allowance is added. These are additional source
maintenance work for a fork, not evidence against the two-node dependency fix.

## Independent context and artifact controls

The known
[borrowed-context defect](https://rustsec.org/advisories/RUSTSEC-2026-0391.html)
is examined separately using upstream's synthetic first/second-job examples.
The baseline context test refuses during the second decoder read with
Unknown frame descriptor before emitting its complete snapshot. Thus no paired
complete plaintext-leak observation is claimed from that command.

The first 1.94 candidate context command exits zero but selects zero tests.
Its printed executable path matches the earlier fixture's path, despite the new
source overlay. That is an unaccepted artifact/test-selection result, not a
green context test; the precise cache cause is not proven here. Future fixtures
must use source/compiler-specific target roots and bind actual artifact/test
inventories. Pure accounting guards now reject zero, missing, duplicate, wrong
name or wrong total/filtered test summaries. The original command is not retried.

A separate Rust 1.88 context cohort compiles successfully. Before its first test
execution an exact native listing confirms one selected context test. The actual
one-test command passes, with 101 other tests filtered: encoder and decoder each
produce exactly `second job`. This confirms that candidate's synthetic
constructor behavior on MSRV. It does not complete the missing paired snapshot,
wide wire gate, strict Clippy, GNU, legacy/seekable exploit coverage or a whole
dependency/production acceptance. The original failures remain unchanged.

Eight inert comparator/accounting controls and Rust formatting pass. Every owned
fixture command retains stable root source/HEAD and frozen fixture inputs after
resolution, with successful Job retirement and no cleanup errors under the
original 1200-second/four-GiB/five-second local review contract.

## D-Bus boundary

Both exact zbus 6.1.0 and zbus_macros 6.1.0 archives declare Rust 1.89. The
[maintained workspace](https://github.com/z-galaxy/zbus/blob/main/Cargo.toml)
does too. The old blocking-api feature and blocking module are absent; wire and
names move into zbus, and its zvariant compatibility alias is deprecated.

The captured direct-use audit finds 18 top-level supervisor source files using
zbus, including two blocking users and eleven old wire-alias users. This is a
boundary inventory, not a complete migration/module-closure proof. It shows why
changing the version or suppressing rust-version would be insufficient. No
manager channel, FD exchange, policy/enrollment, runtime or live frame is changed.

## Backport-package preparation approved; adoption remains gated

The compatible upstream-only route is exhausted for this graph under Rust 1.88.
The following work introduces maintained dependency ownership and therefore
has now received explicit user approval for preparation, retaining Rust 1.88 and
all gates. Publication, installation and server runs are excluded. The following
steps remain prerequisites before product integration:

1. Prepare a separate, reproducible Chitchat security backport from the exact
   reviewed 0.11.1 archive. Keep the Zstd requirement change isolated. Preserve
   the two Clippy failures and fix their annotations in a separate source commit.
   Attribute the wide MTU failure with an actual supported-operation regression;
   fix only a confirmed defect, with original limits and old/new interoperability.
   The unmodified wide gate remains red until its cause is resolved explicitly.
2. Establish a distributable source identity: archive/base checksum, minimal
   patches, receipt, maintenance owner and registry consumer verification, using
   the repository's existing reviewed-fork discipline. A private path override
   alone cannot qualify published HydraCache consumers. No publication or new
   remote repository is authorized by this feasibility report.
3. Independently assess a real Rust 1.88 zbus 6 backport, including all new
   transitive minima, without --ignore-rust-version or merely editing the
   declared floor. Migrate blocking manager/proxy/runtime ownership and deprecated
   wire aliases, retaining exact sender/receiver, credentials, FD, reply and
   legacy dispatch checks. Reject the candidate if those invariants cannot hold.
4. Before adopting either package, run exact API/wire/archive, malformed/partial
   I/O, concurrency, GNU/MSRV and regression gates, plus fresh source/effect and
   historical-input reviews. Only the accepted final graph gets a new complete
   package/workspace milestone; no earlier suffix or receipt substitutes for it.

Waiting for compatible upstream releases is the alternative. Increasing Rust's
minimum, adding deny exceptions, hiding Linux targets or renaming unsupported
crates to mask findings are outside the approved contract. No release/server
admission or performance claim follows from the current partial feasibility.

## Evidence

The [packet](local-runs/remaining-security-20261028/index.json) contains 381
logical members and 183 distinct raw objects: primary metadata/archives, exact
fixture patches/locks, source diffs, every first refusal and command, unit and
MSRV controls, inert analysis and D-Bus boundary review. Index SHA-256:
`06c216b4efc54dcf6cab2cad8f2d2d342d20652d0bd14086b1f2ea8d5fabf541`.
Native binaries/target trees are excluded; the packet does not invent historical
per-launch image attestations. Raw streams and the zero-test refusal are kept.

The [pinned original ReplyState gate](dbus-reply-state-result-20261028.md) is now
complete within the standalone zbus candidate: 25 original cases plus 42 prior
cases, check and strict Clippy on GNU Rust 1.88/1.94. The first import refusal
remains. This advances item 3 without closing distribution/source/effect, complete
caller migration or root package/workspace adoption. It grants no execution or
server authority.

The [reconstructed zbus source packages](dbus-source-package-result-20261028.md)
advance item 2 with whole base/final inventories, portable replacements, stable
local source identities and standalone authoring manifests. The packaged-source
GNU consumer passes 67/67, check and strict Clippy on Rust 1.94/1.88; thirteen
inert archive/export controls pass. This is local, unsigned source preparation,
not registry qualification, product adoption or a new dependency-policy verdict.
Chitchat S remains declined after its separate allocation attribution; its earlier
green functional tests must not be used to reverse that decision silently.

The [whole legacy systemd helper gate](dbus-legacy-calls-result-20261028.md) now
passes 78 cases plus check/strict Clippy on GNU Rust 1.88/1.94, preserving eight
original helper bodies and all 67 earlier identities. Invalid fixture reply-sender
construction, its diagnostic and partial-repair failures remain. This advances
caller compatibility without root integration or a full module/source/effect proof.
