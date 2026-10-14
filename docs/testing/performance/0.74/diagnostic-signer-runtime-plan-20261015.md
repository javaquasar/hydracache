# Bounded GNU runtime inventory plan

Baseline: `75c2d2d20db386b05190e67d0cee2b016ee106c9`. Continue the approved
local bootstrap with bounded ELF metadata inspection and original file retention.
This slice does not launch from a runtime root, attest loaded mappings, select
production custody or permit dummy/real handoff.

## Fixed files and scope

Add private `diagnostic_signer_runtime_fixture.rs`, its parser
`diagnostic_signer_runtime_elf.rs` and tests under Linux x86_64 library-test cfg.
Only the existing joined fixture's fixed compiler-byte helper gains pub(super);
its launch, policy, protocol and helper source remain unchanged. No public API,
CLI, dependency, wire/durable schema, credential, mount, account, cgroup, service,
GitHub job, product workload or qualification input changes.

The parser inspects bytes only and never uses ldd or executes an inspected ELF.
Support little-endian ELF64/x86_64 ET_EXEC/ET_DYN, SysV/Linux ABI version zero,
bounded ordinary program headers, nonoverlapping LOAD virtual/file intervals,
valid alignment/file-backed ranges, non-WX LOADs and nonexecutable GNU_STACK.
The executable entry must lie in executable LOAD memory. Require one DYNAMIC
segment inside its matching file-backed LOAD. PT_INTERP, when present, must be
exact `/lib64/ld-linux-x86-64.so.2`, terminated without embedded NUL.

Bounds: 8 MiB per file, at most 32 program headers, 512 dynamic entries,
64 KiB string table, eight NEEDED names and 128 bytes per dependency name.
Translate DT_STRTAB through exactly one file-backed PT_LOAD, never treat its
virtual address as an unchecked file offset. Reject overflow/truncation, duplicate
singleton tags/segments/names, ambiguous mapping, unsupported segment/dynamic
tags and malformed terminators. Preserve NEEDED order. Reject RPATH/RUNPATH,
ORIGIN flags, TEXTREL, audit/filter/auxiliary and names with separators, dollar,
colon, controls or traversal. The known dynamic-tag set includes the x86_64
PLT metadata observed on this host, not every future ELF extension. Ignore section
tables for metadata inspection; no full relocation/symbol/version/linker proof.

## Original input retention

Use a private four-file fixture: signer, ld-linux-x86-64.so.2, libc.so.6 and
libgcc_s.so.1. The signer is the existing fixed Rust 1.94/GNU helper. Setup copies
only explicit local system library paths and freezes temporary file/root modes;
those sources and fixture ownership are assertions, not production enrollment.
The flat fixture is inventory, not a root that the loader is forced to use.

Open the original root/children with NOFOLLOW/CLOEXEC and bounded descriptor-relative
fixed names. NONBLOCK prevents FIFO preparation from hanging. Require regular,
single-link readonly 0555 files, matching fixture UID/GID, exact root entry set
and original directory stamp. Keep original descriptors, hashes and parsed
metadata. Dependency edges must resolve only to the three selected objects with
matching SONAME, no foreign/unused object or self-dependency. Loader has no
interpreter or dependencies; other images may identify the fixed loader.
Rechecks compare original and fresh named metadata/hash/parse under root brackets;
failure is sticky, with no replacement, refresh or exported FD. Healthy drop
only closes retained inputs. Fixture cleanup touches only owned temporary dirs.

## Tests and evidence

Capture the nine-case joined baseline and read-only readelf metadata for the
helper lint artifact and three local system objects. The lint artifact is not
the tested helper identity. Add a missing-parser/owner red first, then positives
on synthetic ELF and the actual compiled helper/library copies; bounded header/
program/dynamic/string mutations; malformed pointer/terminator/name, preload/
search-path flags and unknown forms; readonly mode/type/link/FIFO/symlink errors;
equal-byte named/root replacement, in-place drift and sticky refusal; graph
substitution/extra/missing objects; CLOEXEC/drop ownership; seed 0x07403b28,
128 parser no-panic mutations and four independent retained fixtures.

Run three focused repeats, joined/standalone/bundle/producer regressions,
Windows portability, scoped format, current strict check/Clippy, MSRV 1.88,
normal-library symbol exclusion, root contracts/docs/registry and local
non-promotable checks. Require-ship remains expected red. Retain all development
failures and raw/source/frozen hashes, update the article, commit and push.

The result is original-file consistency and bounded declared dependency
inventory, not loaded runtime closure or independent root/issuer/job custody.
Loader configuration, preload/cache/hwcaps, later dlopen, TLS/config inputs,
enforced root mapping, runtime/launch composition and mapped-memory assurance
remain open. Dummy handoff and actual tracer loss follow only after the relevant
runtime prerequisites. Worker P2/P3, all-writer cessation, later cells and server
admission remain separate. No threshold or build-target change is permitted.
