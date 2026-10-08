# Local diagnostic artifact verification

This slice starts at clean `6cf3d974` and does not change production routes,
fixed P0 workloads/floors, qualification inputs, old preparation seals or packets.
There is no product build, execution, install, SSH or live unit. The previous
metadata-only Python inspector remains unchanged: an unsigned hash-valid seal
is not compilation provenance.

A separate strict schema-1 build statement signs exact observer source/tree,
clean-before/after assertions, Rust/Cargo/target/profile, empty root features,
System/no counting allocator, fixed build command, binary/config/lock/log hashes
and sizes under `hydracache-diagnostic-build-074-v1`. The verifier needs a
separately supplied trusted builder public key and repository identity; a
request/controller signature is not build authority. The raw receipt digest
and binary digest must match the diagnostic lease. This is verification of an
attestation, not independent proof that an unreviewed signer compiled anything.
No real trusted builder or production key policy is enrolled by this slice.

The bounded Cargo JSON log must contain one matching baseline timing executable
with empty root features/release profile, then a successful final build-finished
record. Logs, lock inputs and all four raw P0 configurations must match their
fixed hashes. ELF64/x86_64 structural checks are not a proof of runnable code.
Positive fixtures use a fake ELF, synthetic log and test signing key explicitly;
they are never promotable build evidence.

Linux opens files relative to a pinned directory descriptor with O_NOFOLLOW and
nonblocking regular-file checks, refuses hardlinks/symlinks/special files,
wrong owner/mode, writable data and unexpected bundle entries, and hashes
bounded descriptor bytes with before/after metadata reconciliation. Production
inspection uses only the fixed root, root-owned non-group/world-writable
ancestors, a 0555 bundle, 0555 binary and 0444 data. The temporary local fixture
entry point uses explicit fixture UID/GID and cannot certify production ancestry.
Descriptors stay owned by the returned snapshot; revalidation checks names and
bytes against those descriptors. It detects replacement and in-place drift but
does not close a later systemd pathname-exec race. Future installation/start
must coordinate immutable generation and revalidation under the host fence;
the trusted root owner is outside the unprivileged mutation threat model.

Tests must cover signed-field tampering/domain separation, malformed/corrupt or
incomplete receipts, feature/source/lock/config drift, missing/duplicate/failed
Cargo terminal messages, binary/byte bounds, linked and writable files, extra
entries, ownership, replaced bundle/file and changed descriptor contents.
Windows proves portable parsing/crypto/content guards only; Linux adds actual
temporary-filesystem descriptor/mode/owner tests. No subprocess is spawned by
the verifier. Live cgroup backend/watchdog, raw timing output sealing and loss
fixtures remain subsequent work; host admission stays closed.
