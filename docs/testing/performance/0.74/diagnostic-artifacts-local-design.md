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

## Implementation and negative preparation observations

`diagnostic_artifacts.rs` implements strict canonical signed receipts, external
repository/builder policy, fixed-input checks, borrowed-byte content validation
and exact deterministic `CellIntent` binding. A checked statement alone does not
certify installed files. `diagnostic_artifacts_linux.rs` separately opens the
exact nine-file bundle through directory descriptors. Binary hashing streams
64-KiB chunks; dev/inode/owner/mode/link count/size/mtime/ctime are reconciled
before/after reading. The returned snapshot retains all descriptors and reopens
names for revalidation. Procfs enumeration refers to an owned directory FD;
data files are opened only with `openat`. No verifier executes a binary.

Cargo JSON is a projection, not validation of every Cargo field. Duplicate known
fields, invalid root target/profile/features/executable, compiler errors,
missing/duplicate/failed terminal records and tail junk are refused. Unrelated
fields remain opaque and signed by their raw-log digest. A trusted signer still
needs a separately reviewed real build procedure/environment: clean-source and
System-allocator assertions are not reconstructed from synthetic ELF bytes.

The initial red test failed E0432 because the module did not exist. An initial
fixture compile failed E0282 until its config map had an explicit String key.
The first root run passed 67 tests but failed two because W11 registration used
unknown `claims.tests` instead of `work_item.required_tests`. These errors were
fixed before implementation commit. Documentation patch-context refusals later
changed no files; patches were reapplied against verified contexts. These are
preparation observations, not numerical attempts; no floor changed.

## Clean-source repeat and remaining boundary

Implementation `70ca60eb3f2f75a326e9f6430467749b76440a04` was clean before and
after [the captured repeat](local-runs/diagnostic-artifacts-70ca60eb/manifest.json).
Windows passed nine artifact plus 42 IPC/lease/host-lock checks (51); local Linux
passed 15 artifact plus 70 IPC/lease/host/server/service checks (85). The root
passed 69 contract and 13 release-evidence tests (82). All-target check and
strict all-feature clippy passed for the supervisor on both OSes and xtask on
Windows. Formatting, doc/local-contract, 17 governance checks, links and book
passed. Ship check remains expected-red, exit 1, for unresolved C74/qualification.
This is focused evidence, not full workspace/release verification or timing.

Positive fixtures use a non-runnable fake ELF, invented Cargo log and test key;
only pinned raw config/lock bytes are real inputs. Linux exercises temporary
descriptors/modes, sparse size bounds and parallel inspectors; Windows proves
portable content/crypto policy only. Three PowerShell Tee captures retain exact
bytes/hashes/Git blobs, not native Linux stdout-byte provenance. Old packets,
both lockfiles and qualification manifest are unchanged. The frozen contract
stays preregistered; the separate packet records implementation completion.

Next local slice: bounded raw timing receipt/spool validation and sealing, then
live owned-tree backend, bounded DBus/watchdog and non-product loss fixtures.
Real clean Linux compilation, reviewed builder trust, immutable install/start
coordination, uncertain-ledger reconciliation and host installation/pilot remain
separate prerequisites. Neither signature nor snapshot opens full D3, supported
allocator retention, production execution or numerical admission.

## Pinned start material before backend enrollment

Baseline `4ec821588a56f3c6fc7b830b8c68ac84e9e26b24` passes 22 Linux artifact
tests. The next isolated slice connects the previously separate checked builder
policy, descriptor-owned bundle, diagnostic state and constructed unit spec.
The fixed-root inspector will require `CheckedBuilderPolicy`, not raw caller
`BuildTrust`. The existing raw-trust fixture reader remains explicitly synthetic.
No receipt, policy, lease, helper or observer wire format changes.

Preparation must validate the startable state before filesystem inspection,
verify the bundle against that identity and policy, bind its exact derived cell
intent, and retain its descriptors alongside the unchanged fixed unit spec.
Revalidation must refuse any state/revision/cell/budget drift or file drift.
Its first failure remains latched for that runtime object; a later matching
observation cannot revive it. A fixture-origin object cannot become production
origin. Dropping or inspecting the object must have no execution side effects.

Test-first checks will cover the production reader's policy type, four fixed
cells, remaining/expired budgets, wrong builder, binary/receipt identity drift,
nonstartable state, revision/cell drift, descriptor replacement, sticky refusal
and independently owned parallel preparations. Positive fixtures remain fake
ELF and synthetic signatures. This object is not a start authorization: the
future backend must still authenticate the current boot/clock and durable intent
under the host fence, bind the actual executable and original manager invocation,
retain tree/process identity, revoke writers, persist failures, run its watchdog
and reconcile uncertain requests. No observer, unit or host service is started.
