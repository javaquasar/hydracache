# Owner retained installed entry materialization

Consume the same InstalledBootstrapUnit into a new private wrapper. It must not
accept caller entry bytes, paths, images, FD values or another owner. Retain the
original owner before the new resource in field order and explicitly refuse it
before new descriptors close, including construction failure and Drop.

Read HCBT0741 through the original retained EntryResource. Verify its retained
stamp, readonly/CLOEXEC/four seals/zero offset and exact prior census before and
after reading. Derive HCBI0741 from these bytes and the same installed census;
require its old 27-FD/15-role prefix to match the original entry census. Compute
the parent-local fingerprint over fixed-order roles/FDs and original-entry,
installed-image and four ancestry stamps with a separate domain label. No
numeric fingerprint is transferable authority or a child remapping table.

Use the existing sealed constructor to retain one readonly/CLOEXEC/four-sealed
1280-byte record with zero offset, no writable alias and no descriptor getter.
Extend parent census by one FD/role to 33/17, retaining the exact 32/16 prefix
and rejecting every old inode/FD alias. HCBI0741 continues to describe its input
32/16 inventory; do not make its fingerprint depend on the new record itself.
Repeat expected-byte derivation around resource inspection and compare every
retained observation. Missing retained resources refuse without recreation.

Reuse the unchanged complete validator: old installed full frame, new record
inspection, old installed full frame, new inspection, old closing installed
frame. This adds work, not a new window or fewer old checks. All original
300/60/10 origins, order and predicates remain exact. Constructor failure keeps
the refused owner, and a refused wrapper does no further inspection.

Capture missing-component baseline first. Add real local sealed-resource and
census checks for exact bytes, write/truncate refusal, offset/flag/metadata
drift, same-byte replacement, malformed old input, wrong prefix, aliases and
sticky refusal. Add full-frame owner fault models and a compile-only consuming
API control. Private fixtures are not a live original-owner proof. Preserve old
runtime bodies using only three exact additive module hooks and COMPAT views.
Check Windows contracts, GNU 1.94/MSRV 1.88 selected controls, all-target check,
strict all-feature lint, format, docs, registry and local nonpromotable contract.

No child mode, FD export/remap, transport/send, exec/start/release, credential or
namespace enrollment, writer cessation, durable completion or server admission
is implemented by this materialization. Do not install, change clocks/services,
start server/product workloads, rerun original live/measurement attempts or
alter the frozen qualification manifest. Child observation/admission through
the original manager remains next after this retained resource is verified.
