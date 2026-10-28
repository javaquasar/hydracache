# Native D-Bus source revision 2: reconstructed consumer gate

All three source archives reconstruct deterministically from retained upstream
bytes and whole-preimage replacements. The archive-only consumer passes 65/65
on GNU Rust 1.94 and MSRV 1.88, including ten private-bus fault scenarios, plus
strict test-cfg/all-feature Clippy. Generated test formatting now passes. Root
dependencies, original live predicates/windows and release admission are unchanged.

This follows the [native transport result](dbus-native-transport-result-20261028.md)
and its [reconstruction plan](dbus-native-transport-plan-20261028.md).
The [held packet](local-runs/dbus-native-source-2-20261028/index.json) has 165
logical members / 110 distinct objects; SHA-256:
`2babe9897ec6bd30ecfcb264af372de14129bde23baab8b5b324cec0fd1cf952`.
Executable bytes are excluded; actual complete-image receipts contain source,
path, byte count and hash. Source archives reconstruct from repository inputs.

## Source and distribution identities

The [recipe](../../../../tools/dbus-native-transport-074/patches/recipe.json),
SHA-256 `c3fe73bd7c470ca3f524c7dc3b84746ceb051fd43a8e5d64737879b66d648a5f`,
specifies all 266 final members, 12 upstream changes and exact replacement objects.
The existing pinned inert codec performs two in-memory reconstructions: no source
archive code is imported/executed or extracted to arbitrary paths. Order,
UID/GID/mtime/mode and owner metadata are canonical. Links, aliases, duplicate
names, stale preimages and changed objects/inventories/archive identities refuse.

| Archive | SHA-256 |
| --- | --- |
| zbus 6.1.0 local native source 2 | `f7b72098a9c8ae25801f71c0f4f3f9f0b3a6c4a826c031a3177ef424c4924c7f` |
| zbus_macros 6.1.0 local source | `0485f3fa2b311beb0ba6e2ce5080c7ab9dfee515a70ebd151fb897fe61cdecfe` |
| zbus_utils 6.0.0 local source | `be51fb124bbc2da01473d4d56da44911ac3df321042867cfd5e869ee087c312c` |

Compared with the previous reconstructed source, only four zbus members change:
the two matching manifests, explicit modified-backport provenance and the socket
module's nondefault native test factory. The other two crates remain exact.
The production feature selection is unchanged; the separate private consumer
test manifest additionally selects hc074-unix-fixture. These are modified local
sources with visible upstream names/licenses, not pristine registry artifacts.

`python -B tools/dbus-native-transport-074/source_package.py rebuild-only`
needs retained repository bytes only, with no prior build/target/native source
dependency. Its output is exclusive, unsigned, unpublished and not registry-
qualified. The raw readable review diffs preserve their blank context lines;
local data attributes prevent Git treating those diff bodies as source whitespace.
No source/diff bytes or original negative outcome are normalized.

## Actual consumer and first results

Preparation excludes the predecessor's loose fd/source tree, then materializes
only members read from the actual exported canonical archives. Every resulting
source member matches the already-qualified f2 inventory. Two cfg-test files
receive explicit formatting changes, and two vendor paths relocate. Production
code, manifests, lock, checks and observed deadlines remain byte-for-byte equal.
The isolated resolved graph remains the reviewed 135 identities, with no zbus 5.

The first inert consumer control uses a string-literal regex that mishandles
Rust's escaped newline in the XML configuration. Its original test/driver sources,
frozen preparation and explicitly labeled observed failure summary are retained;
there was no Cargo/test execution there. The corrected multiline-aware control
uses a distinct consumer2/native-a7 context. It preserves the exact literal
sequence, guards, stale-preimage refusal and original 65-case coordinator. It
does not change a Rust runtime assertion or use retry-to-pass.

| Gate | GNU 1.94 | MSRV 1.88 |
| --- | --- | --- |
| Actual full default library test image | pass | pass |
| Complete actual catalog | 1,593 identities | same 1,593 identities |
| Exact selected identities | 65 passed, 0 failed/ignored | 65 passed, 0 failed/ignored |
| Explicitly filtered | 1,528 | 1,528 |
| Native private-bus scenarios | all 10 | all 10 |
| Strict library/test-cfg/all-feature Clippy | pass | pass |
| Source/vendor/boot and owned postflight | pass | pass |

Rustfmt passes both changed files, with no output/error. Four archive/export
controls and three consumer/source/driver controls pass before execution. All
27 existing inert/retention controls pass, followed by three additional held-
packet checks: every byte, archive reconstruction and exact paired accounting.
No filtered/ignored case is counted as executed. The unchanged production source
inherits the previous r normal-library check; fresh consumer image builds are
recorded as builds, not fresh normal-library checks.

The original 60-second private matrix, three-second infrastructure/two-second
method limits, 1200/1100 owned origins, 4 GiB floors, five-second poll/retirement
and 300/60/10 live contract remain unchanged. No original owner or system bus is
opened. No existing service, clock, server, product workload or qualification run
is changed or started.

## Release boundary

This closes the source-reconstruction/formatted-consumer subgate for this native
backport. It does not adopt the root dependency or approve registry distribution,
complete source/effect/policy integration, Windows/package/workspace milestones,
production trust/enrollment or a C74 freeze. Chitchat preview stays declined.
No numerical product improvement follows.

Original delivery, autonomous cessation of every writer before fence release,
durable completion/cells 2–4, SIGKILL/lost-ACK and same-attempt reconnect remain
open. The one approved dedicated-host preparation attempt already failed at the
unchanged 10-second enrollment boundary, before original send; it is retained in
the [server profile result](server-preparation-profile-result-20261028.md).
Server systemd 255 still lacks the selected delivery prerequisite; the chosen
local systemd 257 completion/transfer route remains separate. No second server
attempt, release install, supervisor stop or product load is authorized by this
local package gate.
