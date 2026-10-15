# Shared GNU signer declaration catalog

The normal library and old owned fixtures now share one bounded ELF parser and
four-role dependency-graph check. This removes the test-only parser boundary
before preparing actual runtime manifests. The
[preregistration](diagnostic-signer-gnu-catalog-plan-20261015.md) and
[contract](diagnostic-signer-gnu-catalog-contract.toml) preserve the old profile;
no relocation/symbol/version, loaded-object or custody validation is inferred.

## Library and fixture reuse

`diagnostic_signer_gnu::inspect_gnu_declarations` accepts exactly four borrowed
byte slices in signer/loader/libc/libgcc order. It checks every image's 64-byte
minimum and 8 MiB maximum before parsing or hashing. The existing strict ELF64
little-endian x86_64 parser is compiled once through this module, with only
crate-private visibility for its internal functions and projections. The old
runtime fixture imports that parser, graph and fixed names rather than keeping
another copy. Its owned filesystem/launch mechanics remain test-only.

The parser limits and declaration semantics are unchanged from the
[old runtime inventory](diagnostic-signer-runtime-design.md): fixed GNU
interpreter, bounded mapped string table, header/segment/dynamic/name budgets,
no unsafe/unknown search tags and an exact reachable four-role inventory with
matching SONAMEs. Sections and relocation/symbol/version contents are not
validated. A matching graph is not a safe-to-execute ELF proof.

The serialize-only `diagnostic-signer-gnu-declarations-074-v1` catalog contains
logical roles, SHA256/byte length, interpreter, NEEDED, SONAME and header shape.
Its complete-validation, closure, custody, original-start, signing and admission
fields are always false. It cannot deserialize into its private-field Rust type
or convert into a bootstrap guard. No trust, key, epoch or environment is selected
from these bytes, and no controller/supervisor consumer treats them as authority.

## Local file example

`inspect_signer_gnu_catalog` is a Linux example, not an installed supervisor mode.
It requires four explicit absolute paths; opens read-only/no-follow/nonblocking/
close-on-exec leaves; checks regular single-link files and the size budget; and
compares original metadata around bounded positional reads. Files remain open
only for their own read. The output binds the resulting byte snapshots, not a
continuous or atomic four-file lifetime. It does not check trusted ancestors,
revalidate current names after reading, retain mapped inputs or revoke writers.

The example prints a catalog only after all four images pass. It never runs ldd,
an inspected image, a loader or a signing operation. Ordinary example/compiler
processes use their host runtime, which is not attested by their own output.
Unsupported platforms and invalid inputs refuse without a partial catalog.

## Checks and enrollment boundary

Ordinary portable integration tests independently construct four synthetic ELF
images and verify role/digest fields and every closed authority field. They reject
wrong/missing/foreign/self roles, duplicate NEEDED names, interpreter/ABI changes,
scripts, short/over-budget images, unsafe flags, search paths, unknown tags and
overflow. Byte changes outside declarations still change the digest, demonstrating
why matching declarations alone cannot authenticate runtime bytes. Seed
`0x0740c076` covers 256 no-panic mutations; four concurrent readers agree.

The old parser and owned-file fixture corpus still exercises the shared code.
Linux example tests cover original reads and relative/directory/symlink/hardlink/
size refusal. Two compile-fail examples prohibit deserialization and bootstrap
conversion. No persistent publication is added, so no new SIGKILL write boundary
or power-loss proof belongs to this step.

The operator's instruction to proceed authorizes this local implementation under
the [reviewed model](diagnostic-enrollment-review-20261015.md). It does not fill
actual receiver namespace, workflow SHA, protected signing-job receipt, runtime
root/launcher or signed worker-body pins. Initial epoch 1 remains an explicit
new-authority intent, not an installed or observed current epoch. Preserve the
old inventory/proposal's historical unapproved flags. Protected configuration,
installation and execution remain separate admissions.
