# Current bootstrap executable consistency before authentication

Baseline c3dd8469 checks fifteen inherited roles but not the current executable.
A forged sealed image can satisfy FD13 assertions while another inode runs.
Add a separate opt-in --systemd-bootstrap-image-v1 refusal-only mode, leaving
entry-only and census-only modes byte-identical after exact additive hooks.

The new mode repeats the existing complete census and entry checks. Between
those rounds, inspect the fixed procfs thread-self exe magic link and require
the current executable to be the same sealed regular inode as the asserted
bootstrap role. Open only a fixed procfs directory and exe child, verify procfs
and directory metadata, and close every owned inspection descriptor within the
leaf. Compare a second fresh observation after the unchanged census revalidation.
Never repair roles, hash a replacement into authority, send, start or exec.

This proves sequential same-image consistency only. Procfs observations within
an unenrolled mount/user namespace and an untrusted entry do not authenticate
policy, original manager/root/guardians, credentials or signed provenance.
Even a complete forged set executed from its matching sealed image must exit 2
with authentication closed. The original 300/60/10 deadlines, parent frames,
frozen qualification bytes and first live failures remain unchanged.

First capture a missing-component source-contract failure. New synthetic child
controls execute only the local diagnostic bootstrap from a private sealed copy,
never launcher/product/original owner transport. Compare matching executable
inode, same bytes at a different inode, changed assertion and extra FD. Keep old
controls unchanged. Verify Windows source/ingress, GNU and MSRV kernel/process
controls, affected-package check/strict Clippy, format/docs/registry and local
nonpromotable performance contracts. Retain exact source and initial failures.
No measurements, clocks/services changes, privileged work or server runs.

After this leaf, independently authenticate credentials/namespaces, protected
build/policy and original manager/root/guardian provenance before one-shot
launcher delivery or any all-writer/lifecycle completion claim.
