# Signed bootstrap provenance and consumed original owner

Baseline is dd820049. The protected bootstrap selector exists, but no bootstrap
receipt, sealed image join or original-owner wrapper exists. The user approved
these additions without clock/service changes, original execution or server runs.
The sealed-memfd bind refusals and all earlier first outcomes stay unchanged.

Add separate canonical, bounded signed policy and build receipt schemas. Their
Ed25519 domains differ from each other and all previous roles. The policy key
and complete policy digest come from the protected selector, not incoming claims.
The policy binds repository, builder/controller keys, authority epoch, bootstrap
image pin, source commit/tree/lock, original held policy/receipt and host/launcher
image digests. The receipt derives fixed GNU 1.94, target/profile/no-feature build
observations and content/log digests; signatures are build assertions, not proof
that a clean isolated compiler actually ran. No key defaults, generation or signer
CLI are introduced. No bootstrap executable or installed configuration is created.

Consume MaterializedHeldUnit into a nonextractable wrapper retaining a readonly,
CLOEXEC, exactly four-sealed bootstrap image. Fresh unchanged full owner/material
frames bracket new source, signature, original-context and image checks. Retain
the protected file identity as well as bytes across rounds; same-byte replacement
must refuse. Match policy to the actual consumed context's held policy/receipt,
host and launcher digests, never caller-supplied expected values. Closing full
frames keep all original observations and absolute 300/60/10 origins and limits.
On any error retire the same original owner before dropping added resources;
failure also retains that owner. No owner/FD/start/send/release/unlock/continuation
is exported. This is not live manager, child credentials or execveat admission.

Test first with current-source integration guards missing implementation. Then
cover canonical schemas, signatures/domains/key/pin/epoch/source mutations,
signed false build assertions, bounded ELF/log/lock content, image metadata/seals,
protected-file replacement and sticky refusal. Deterministic owner models check
every fault cut, context drift, complete frame order and shared retirement. Linux
compilation checks the concrete consumed-owner wiring; no original live fixture
or measurement cohort is selected. Do not describe models as original live proof.

Run focused portable contracts and Linux units on GNU 1.94/MSRV 1.88, package
check/strict Clippy/format, doc/registry/local performance contracts and diff checks.
Retain exact source/commands/results separately from old negative packets, commit
in logical pieces, and leave actual bootstrap execution and all-writer tests open.
