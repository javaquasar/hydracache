# Bounded original runtime input local evidence

Tested source: `acffe2c544d2f501993b90d8ff9cc6ae1dafad71`.
Baseline: `75c2d2d20db386b05190e67d0cee2b016ee106c9`.
Preregistration: `47ff9e3fd152cf1230ed20777e6fb3477a73ee68`.
[Manifest](manifest.json) binds exact source, unchanged inputs and raw logs.
This proves test-fixture input consistency, not loaded runtime or server admission.

## Final results

Three ten-case Linux repeats pass. Four retained readonly original files have
matching metadata, hashes, bounded ELF declarations and exact graph membership.
Synthetic tests distinguish virtual DT_STRTAB from its file offset and exercise
overflow, truncation, unknown/duplicate metadata, unsafe names and loader-search
forms. Explicit bounds include oversized streams, dynamic entry count, eight
versus nine dependencies and 128 versus 129 byte names. Type/mode/link/FIFO/
symlink/UID/GID, named/root replacement, hash-only drift, missing/extra/foreign
objects, sticky refusal, CLOEXEC, positional reads and independent drop checks
pass. Seed 0x07403b28 covers 128 no-panic byte mutations and four independent
owners per repeat, not arbitrary-ELF acceptance or multithreaded signer support.

Each repeat observes the same helper, loader, libc and libgcc SHA-256 recorded
in the manifest. No numeric performance result is measured. The helper remains
the fixed Rust1.94/GNU O3/panic-abort image; parent-crate MSRV1.88 checks do not
change that builder profile. Actual system libraries are local fixture sources.

Joined nine, first-exec eleven, seccomp eight, bundle/executable eighteen and
producer twelve regressions pass. Windows portable signer tests pass nine;
Linux MSRV passes the new ten tests. Current Windows/Linux check/strict Clippy,
Windows MSRV, scoped format, 140 root contracts and xtask check/strict Clippy
pass. Normal-library symbols exclude runtime/joined/first-exec/seccomp fixtures.
Docs, unchanged 169-gate registry and local non-promotable performance contract
pass. Require-ship remains expected exit1. No full release gate, server job,
product pilot, performance workload or expensive qualification is claimed.

## Retained development failures and evidence checks

The baseline records joined nine-case success and readelf metadata for local
system sources and a separate helper lint artifact. That lint artifact is not
the tested signer identity. Missing parser/owner compilation refuses with E0433.
The first parser/owner development compile triggers a Rust1.94 ICE while rendering
a dead-code diagnostic (`slice index starts at 5 but ends at 4`,
`check_mod_deathness`). Completing the ten tests exercises previously unused
fixture paths and the same compiler then passes; no lint is suppressed, build
profile changed or compiler bug claimed fixed. Expanded bounds tests also pass.

The first regression command incorrectly selects an integration target instead
of a libtest filter and stops after passing runtime/joined/standalone tests.
Its failed raw capture remains separate; corrected final commands pass all
regressions. Raw exit values describe each captured command/group, not admission.

Read-only `python verify_packet.py` checks LF source hashes against the exact
commits, frozen raw qualification digest, raw capture hashes/exits and closed
claims. `python -m unittest test_verify_packet.py` mutates owned temporary packet
copies. It covers unknown/bool schema, wrong source/hash, empty bindings, dropped
frozen/positive captures, raw-log/exit substitution, fixture-input hash and false
admission. Nine copied-packet cases pass on Windows and Linux. The initial Windows
report was directed into the packet after the original inventory was frozen;
it is retained separately. Final tests direct reports to scratch first, then
copy/bind them, leaving the packet unchanged while its tests execute.

Final inventory: 24 captures, 11 tested-source and 25 unchanged-file bindings.
Later source changes require checking at this source rather than rewriting this
packet. Shared joined source is independently compared to baseline after undoing
only `pub(super)` on `compiled_helper`; its behavior/helper/tests stay unchanged.

## Still closed

The accepted four-file graph does not enforce a loader root, eliminate cache/
preload/hwcaps/dlopen inputs, validate all relocations or attest mapped memory.
Readonly descriptors and 0555 fixture modes are not protection from every
same-UID writer or independent ancestry/issuer/job custody. The module executes
no inspected ELF, exports no capability and has no normal-library consumer.

Next specify/enforce selected loader/root/configuration and join retention to
the original launcher. Dummy handoff and actual tracer-loss proof follow the
relevant runtime prerequisites. Production custody, authenticated worker start,
P2/P3 writer cessation, completion/later cells and server admission remain open.
Reviewed pins, dependencies, CLI/server code and frozen qualification are unchanged.
