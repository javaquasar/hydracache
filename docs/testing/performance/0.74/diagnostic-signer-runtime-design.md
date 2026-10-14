# Bounded original GNU runtime input inventory

This private Linux x86_64 libtest fixture inventories four original files. It
does not launch from them or certify a running signer's runtime. Its only reuse
of the joined fixture is the cached fixed GNU helper byte compiler; joined
launch, protocol, policy and helper source remain unchanged.

## Byte parsing and declared edges

The parser accepts only the preregistered ELF64 little-endian x86_64 profile.
It bounds file size, program headers, LOAD segments, DYNAMIC entries, strings
and names. Arithmetic and file intervals are checked before slicing; LOAD
virtual/file intervals must not overlap. Alignment, entry range, non-WX segments
and nonexecutable GNU_STACK are checked. One DYNAMIC and optional fixed
PT_INTERP must map to matching file-backed LOAD offsets. DT_STRTAB is a virtual
address, translated through one readable file-backed LOAD, not a file offset.
These field distinctions follow the [ELF manual](https://man7.org/linux/man-pages/man5/elf.5.html).

Names are bounded and NUL terminated. Duplicate singleton tags, singleton
segments and dependency names refuse. Unknown metadata, RPATH/RUNPATH, ORIGIN,
TEXTREL, audit/filter/auxiliary forms and unsafe dependency names refuse. Known
relocation, symbol, version and x86 PLT tags are only inventoried; their values
are not independently verified. Section tables are ignored. A synthetically
accepted metadata image is not necessarily executable or safe to load.

The flat inventory is exactly signer, ld-linux-x86-64.so.2, libc.so.6 and
libgcc_s.so.1. SONAMEs and declared NEEDED edges must resolve to selected names,
with all four reached, no self/foreign/unused object, and a dependency-free
loader. The signer identifies `/lib64/ld-linux-x86-64.so.2`. Copying local files
does not force the kernel or loader to use these copies. Environment, loader
cache/config, preload, hwcaps selection, later dlopen and other inputs remain
outside this proof. Their role in runtime selection is described in the
[dynamic-loader manual](https://man7.org/linux/man-pages/man8/ld.so.8.html).
Neither ldd nor an inspected image is executed by the inventory.

## Retained files, not custody

Fixture setup copies explicit local system paths and helper bytes into an owned
temporary directory, then freezes files and root to 0555. Local system sources,
ancestors and nonroot UID/GID are fixture assertions, not authenticated production
selection. Setup writes different inodes from joined launch images. No runtime
FD, path, metadata or acceptance bit is exported to a launch consumer.

The owner retains the original directory and four readonly descriptors. Fixed
children are opened relative to that directory with NOFOLLOW/CLOEXEC/NONBLOCK;
the last flag avoids a FIFO hang before regular-file validation. Regular files
must be single-link, bounded and 0555, with matching fixture UID/GID. Directory
entries must match exactly. Rechecks bracket original SHA-256/parse comparisons
with original and fresh named metadata and original/named root stamps. They
never refresh acceptance after substitution; the first failure latches refusal.
Healthy drop closes owned descriptors without touching another holder's FD.

This is point-in-time consistency, not an atomic immutable snapshot, revocation
of external writers, protected ancestry or independent runtime custody. No
ownership operation, namespace, mount, cgroup, service, real job, private key,
public API, CLI, schema or dependency is introduced.

## Verification and next boundary

Ten ordinary tests cover synthetic address translation, header/range/dynamic
mutations and explicit bounds, actual four-file inventory, mode/type/link/FIFO/
symlink/identity errors, equal-byte root/named replacement, hash-only content
drift, graph/extra/missing objects, CLOEXEC/positional reads/drop and seeded
no-panic mutations with independent owners. Tests directly reset synthetic
metadata only to isolate the inode/hash guards; the owner has no reset API.
The 128 mutations are no-panic probes, not proof every altered byte must refuse.

The [preregistration](diagnostic-signer-runtime-plan-20261015.md) and
[contract](diagnostic-signer-runtime-contract.toml) preserve closed admission.
Next specify and enforce selected runtime root and loader configuration, then
compose runtime retention with the owned launch without exporting generic
capabilities. Independent production custody still requires separate selection.
Dummy handoff, actual tracer-loss proof, authenticated original-start, P2/P3
all-writer cessation, completion/later cells and server admission remain open.
