# Installed bootstrap entry assertions

The next local prerequisite is a separate canonical assertion envelope for the
installed executable origin and the unchanged sealed bootstrap reference. It
does not authenticate a child or enable an inherited FD map. Its input remains
untrusted even when canonical and internally consistent.

Implement HCBI0741 as exactly 1280 bytes with little-endian integers. Embed the
entire canonical 1024-byte HCBT0741 at offset 16 without changing its fields,
origins or bounds. Keep the installed device/inode/size at 1040, content SHA-256
at 1064, fixed pathname SHA-256 at 1096, and asserted parent inventory fingerprint
at 1128. Parent counts at 1160 and 1164 must be 32 physical descriptors and 16
semantic roles; they are not child FD slots. Offset 1168 contains SHA-256 of the
embedded entry. Header padding 12..16 and trailing padding 1200..1280 are zero.

The installed identity must have nonzero components and a different
device/inode pair from every old asserted regular-image identity. Its size and
content hash must equal the old sealed bootstrap reference. The pathname digest
is fixed to /opt/hydracache-perf/bin/diagnostic-held-bootstrap-074, not caller
configuration. These checks establish assertion consistency, not filesystem
identity, root ownership, signed build verification or original-owner custody.
The inventory fingerprint is opaque untrusted assertion data, not an authority.

Use a separate opt-in codec module and a separate COMPAT entry. Old readers and
the new reader reject each other's magic/length; no migration, fallback or old
mode reinterpretation is permitted. Preserve all old runtime bodies except the
single module declaration. Do not yet materialize the record, extract owner/FDs,
add a child mode, remap roles, send, exec, start or release anything.

First capture the missing-component refusal. Test round trips, every byte's
canonical representation, length/version/padding/count/digest corruption,
content/size disagreement, image aliases, old embedded shape/deadline refusals,
and forged but internally consistent assertions. Test exact additive source and
COMPAT views and the unchanged qualification digest. Run Windows and local GNU
1.94/MSRV 1.88 unit/check/strict-lint controls, scoped formatting, docs, registry
and the local nonpromotable performance contract. Keep all first results.

This step adds no fresh live checks, timing windows or observations. After this
schema is checked, bind and retain it under the same consumed installed owner,
then implement a separately versioned child inventory/current-executable reader.
Actual sandbox enrollment through the original manager, authenticated delivery,
all-writer cessation and SIGKILL/lost-ACK integration remain closed. No clocks,
services, installation, product loads, measurements or server runs are in scope.
