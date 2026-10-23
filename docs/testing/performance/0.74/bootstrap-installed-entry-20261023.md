# Installed executable and sealed reference entry assertions

HCBI0741 keeps the installed executable origin distinct from the sealed bootstrap
reference inside a separate canonical assertion envelope. It contains the exact
old HCBT0741 bytes, the new installed identity and the asserted parent inventory
fingerprint. This codec does not yet materialize an owner-held record or define
a child transport, inherited FD layout or authentication result.

## Separate identities with unchanged original assertions

The [local plan](bootstrap-installed-entry-plan-20261023.md) follows the retained
[installed origin](bootstrap-installed-origin-20261023.md). The installed image
must assert nonzero device/inode/size, a distinct device/inode pair from each old
regular image, and the same bounded size/content hash as the sealed bootstrap
reference. A hash match cannot substitute for required inode separation. Inode
numbers on different devices are not aliases.

The 1280-byte little-endian format embeds all 1024 old bytes at 16..1040. Neither
the original owner, documents, guardians, streams, origins nor bounds are
translated. The fixed installed pathname digest, asserted parent counts 32/16,
embedded-entry digest and zero padding have one representation. The counts are
parent inventory dimensions, not child slots. The COMPAT entry fixes the layout
and rejects unknown versions, lengths, padding, counts and digest mismatches.
Old and new decoders reject one another's records without fallback.

## Structural consistency is not authentication

Seven codec controls cover exact offsets/round trips, all 10240 single-bit
mutations, truncation/extension/version/padding/count corruption, image aliases,
zero identity, content/size disagreement and the 128 MiB bound. They also check
that a recomputed outer digest cannot hide malformed embedded assertions or a
cross-image mismatch. Old embedded shape and deadline refusal behavior remains.

A forged but internally consistent record still parses. The codec deliberately
cannot establish live owner custody, real root ancestry, signed build/policy,
fresh credentials, namespaces, time or manager identity. Parent inventory digest
is opaque assertion data; SHA-256 here is a consistency checksum, not a signature.
The next owner factory must derive fields from its original retained resources,
not promote caller-supplied canonical assertions into authority.

Exact source contracts allow only one new codec module declaration and its
COMPAT block. The old parent/installed-source/entry/census and child modes remain
byte-identical under the additive view. No File/FD/owner extraction, executable
handoff, transport, clock or service operation is added by the codec.

## First outcomes and remaining join

The [first-result record](bootstrap-installed-entry-first-result-20261023.json)
keeps the initial absent-codec refusal and the development test compilation
errors separate from subsequent checks. Comparing digest slices required two
test-only reference fixes; production predicates were unchanged. First raw
snapshot logs also retain the Windows duplicate-module Clippy refusal.

The duplicate test module is now imported through the existing guard rather than
loaded twice. No lint is suppressed and no runtime predicate changes. All final
lanes completed on the same 115-file source snapshot:

| Scope | Passed checks |
| --- | --- |
| Windows | Seven new codec controls plus fifteen preceding controls; 57 source contracts; all-target check, strict all-feature Clippy and affected-package formatting. |
| GNU 1.94 | 80 selected controls including the seven codec controls and nine installed-origin controls; all-target check, strict all-feature Clippy and exact selected unsigned build. |
| GNU MSRV 1.88 | The same 80 selected controls, all-target check and strict all-feature Clippy. |
| Metadata | Four Python controls, doc-check, 188 discovered/registered gates, local nonpromotable performance contract and diff checks. |

The codec implementation is 7975386c; the checked source/tooling commit is
ce7931bc36c846c1097e669398b56920c7400fb6. The exclusive
[inventory](local-runs/bootstrap-installed-entry-20261023/inventory.json) has
SHA-256 87fae1b9676e6689d2aef1e8f295f99ca3d000d7a1da83f02682d2795a6b248c.
Its 1113 members plus inventory were audited against all 1114 exact staged Git
blobs. First baseline and lint failures remain separate from final passes.

The selected unsigned GNU image is 1603400 bytes, SHA-256
2b55fec61b8c1b2f9a37d0bdb47024552085bb3c48e38bf909a826b6c2424a65.
This dirty-worktree check build is not a clean signed build receipt, installed
origin enrollment or numerical size/performance improvement. Qualification
manifest bytes remain unchanged at SHA-256
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.

Before this record can be sent, a consuming factory must retain its sealed
materialization under the same installed owner, repeat the complete validation
frames and reject missing/replaced inputs without reconstruction. A separate
versioned child reader must then bind its actual executable and admitted sandbox
context to that origin. Authenticated delivery, writer cessation, durable
completion, later cells and full SIGKILL/lost-ACK integration remain closed.
No installation, clock/service change, server action, original live attempt or
performance measurement is part of these assertion controls.
