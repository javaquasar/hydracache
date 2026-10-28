# Remove the last old D-Bus test backend

The [joined legacy caller gate](dbus-legacy-integration-result-20261028.md) passes
64 cases/check/strict Clippy on GNU Rust 1.94/1.88 but deliberately retains zbus 5
as a dev backend. This approved security/backport continuation removes that
remaining dependency in a separate local source copy. No new optimization
hypothesis, root dependency adoption or real execution admission is introduced.

## Exact implementation boundary

The two complete manual transport/DTO source preimages are pinned before edits.
All original cases, private-bus owner/peer UID/GID/PID assertions, wire capture,
17-byte partial writes, capability faults, atomic receive guard and three/two-
second harness/method limits remain. Reader/writer now delegate to the actual
zbus 6 registered Unix halves, with native peer credentials; no old Message or
credential bridge occurs on that path. The DTO alias control retains its source
checks and FD clone/drop/CLOEXEC semantics on one backend.

A nondefault source-backport fixture feature exposes the exact native runtime
selection and UnixOps registration already used by the connection builder. It
creates no authority, original context or authenticated-start token. Original
production default features and manager/frame/kernel/live predicates do not
change. Atomic fault injection uses an extra same-socket clone only in the test
wrapper; it does not add a payload/production-writer alias. This modified source
updates its unsigned backport provenance/manifest digest and is not represented
as an unchanged canonical source package or trusted signed product build.

## First gate is offline lock resolution, not execution

The source candidate removes the old zbus 5 dev requirement and adds the same
zbus 6 path as a dev requirement with only the nondefault fixture feature. The
inherited lock is explicitly unqualified. Under the original owned local CI
envelope, run one offline generate-lockfile. Resolve only from the 156 already
retained primary archives and path backports; do not fetch or introduce a new
registry identity. The source graph and exact command are reviewable before
dispatch. Allow only native fd/Cargo.lock to change and preserve all old inputs.

Semantic lock review normalizes ambiguous Cargo dependency spellings. Require
the same identity/version/source/checksum/metadata for every retained package,
the same edges except removal of the old root test dependency, no new package,
and complete absence of zbus 5. Keep first resolver/audit refusal independently;
never broaden this policy to accept unrelated resolver drift. Unused baseline
locks/archives remain historical data and are not selected Cargo workspaces.

Five inert controls check whole-body drift refusal, preserved transport/DTO
semantics, real runtime/UnixOps delegation, dependency normalization and six
negative lock mutations. Direct installed Rustfmt checks the new fragment; the
first sandbox rustup-shim canonicalization AccessDenied is an environment refusal,
not a native compiler/test verdict. Native compilation and real private-bus
runtime still need actual separate gates; no claimed numerical performance gain.

Keep original Windows Job/Linux pidfd-subreeper containment, 1200/1100-second
origins, 4 GiB floor, five-second poll/retirement, live 300/60/10 windows and all
original checks/count/order. No service/clock/server/product activity or new
qualification follows. After accepted lock/source closure, run GNU/MSRV and
the original complete selected cases, then actual private-bus partial/FD/peer/
lost-ACK faults, source reconstruction/distribution and final policy/root gates.

## First resolution and source-audited feature effect

The first actual offline resolver succeeds, produces a 135-package lock from the
156-package predecessor and introduces no package/checksum change. The original
strict audit refuses a second changed retained edge, Tokio→tracing, so its native
result remains failed. All other source/boot and CI-retirement checks pass. No
compiler, test or bus runs. Do not rerun Cargo or rewrite that first refusal.

Primary normalized manifests prove the cause: zbus 5.19.0 explicitly requests
Tokio's tracing feature, while the qualified 6.1.0 manifest does not. Pinned Tokio
1.53.3 declares tracing as an optional cfg(tokio_unstable) dependency. This is
neither a HydraCache live-check predicate nor its own validation observer. A
separate offline reviewer requires exact whole manifest hashes and the exact
candidate manifest's one empty nondefault feature, then permits only root→old
zbus and this exact Tokio→tracing removal. Every other package/checksum/edge
predicate stays strict. Three controls refuse source and unrelated graph mutations.
The old strict auditor is unchanged; its failure remains evidence.

The same first lock passes that explicit source/effect review with 21 removed
packages, no new nodes and unchanged retained checksums. It is not a full policy
or compatibility verdict. The first review loader also records a Python prepare
module-name collision before writing any result; the corrected unique module
loader performs the offline review without another resolver run.

A separate c/native-a2 compiler context freezes exactly that reviewed lock and
the same transport sources; only vendor paths relocate. Paired original library
check/strict Clippy and the exact preserved 64-case/test-cfg gate follow under
unchanged CI bounds. Native private-bus/FD/peer faults and source reconstruction
remain subsequent gates. No numerical claim, MSRV change or deny exception.

The first c compile envelope refuses at its original physical 4 GiB floor:
exit 125, retired=true, CapacityError retained, no native verdict or compiler
phase recorded. The inherited outer error label says retirement, but the raw
receipt proves successful retirement and the actual resource refusal. Nothing
is synthesized as a native result or replayed from the consumed output path.

Cleanup initially aborts while another Windows compiler is active; read-only
scope observations identify a 0.75 target. Its processes/files are untouched.
Only after a fresh census finds no Cargo/rustc/rustdoc do we remove 6,135 obsolete
0.74 .rlib/.rmeta files dated before 2026-10-27, totaling 15,034,771,967 logical
bytes. Source/evidence/executables/current GNU images and 0.75 are preserved.
Free space is 18,111,078,400 bytes at cleanup completion and later independently
29,314,158,592 bytes; the extra change is not assigned to that deletion.

A distinct r/native-a3 resource-repaired context preserves that first refusal and
all code/lock inputs, relocating only two vendor paths. Two controls verify no
other source change and no invented native verdict. Additional initial headroom
of 16 GiB is required before dispatch; the existing running 4 GiB floor,
1200/1100-second bounds, five-second polling/retirement and live limits stay exact.
No VHD compaction, settings, clocks, service change or foreign-process stop.

The repaired r source completes paired GNU 1.94/1.88 library check/strict Clippy,
actual full default test images and strict test-cfg/all-feature Clippy. Both
complete catalogs contain 1,593 identities; each exact original cohort passes
64/64 with 1,529 filtered, no failures/ignored cases. Source/boot and owned
retirement postflight pass. No system bus, private bus or real owner is executed
by that cohort; the native backend's live IO still needs its own gate.

The next single ignored matrix is the original nonroot private dbus-daemon case,
selected by exact full identity from the same actual source/image/catalog. Its
nine scenarios and 60-second matrix/three-second infrastructure/two-second method
bounds stay unchanged, under the existing original CI envelope. Private daemon
binary/source is recorded; no package or service is installed. Additional driver
pins precede dispatch. A source audit predicts a characterization boundary:
bare-receiver assumes the old non-CLOEXEC default, while actual zbus 6 uses
CMSG_CLOEXEC. Do not loosen writer_shape or change expected flags to conceal a
result. Keep the first runtime result; any follow-up fault-generation change
must retain the original negative guard and also test actual native positive IO.

That first matrix is consumed: exit 101, zero passed / one failed / 1,592
filtered, source/boot unchanged and owned retirement successful. It stops at
bare-receiver's old flag characterization (actual 1 versus expected 0), before
any scenario completion record. Native zbus 6's CMSG_CLOEXEC is a stronger default,
not a reason to relax writer_shape. The first result stays failed.

A distinct test-only candidate explicitly injects loss of CLOEXEC into the owned
received FD, executes the original negative assertions and restores the flag
even before resuming an assertion panic. All nine scenarios and original bounds
remain; healthy cases now use native read/credentials, while an additional
atomic-adapter case retains that former adapter's coverage. No production source,
manifest, lock, predicate, observation count or deadline changes. Four inert
controls cover this projection, retained first refusal and exact coordinator.
An initial f preparation stops at driver rendering before execution; the fixed
f2 context is separately frozen. Its fresh actual full test images precede a
65-case cohort on both compilers, including just this ignored private matrix,
then strict test-cfg/all-feature Clippy. The prior r normal-library proof applies
to the identical production source; image compilation is recorded separately.
No system bus, service, real owner, server or product workload is involved.

## Native source revision-2 reconstruction and consumer

The separately pinned canonical recipe reconstructs the exact three native
source archives from retained upstream bytes and 12 whole-preimage changes.
All 266 source members equal the qualified f2 inventory. The native factory is
explicitly nondefault and the zbus provenance records source revision 2;
normalized/authoring manifests remain identical. Four inert controls verify
determinism, exact member inventories, modified provenance, stale input/object/
archive refusal and exclusive unsigned export. The production selected features
stay exact; the private test consumer records its extra fixture feature separately.

An archive-only consumer excludes predecessor fd/source before materialization.
The only differing bytes afterwards are two explicit cfg-test formatting changes
and two vendor-path relocations. Its first inert string-literal control mishandles
Rust's escaped newline; source and an observed failure summary remain in the
unexecuted first consumer context. A separate consumer2/native-a7 uses the corrected
multiline-aware control. Three controls then pass, including exact source inventory
and nested 65-case driver rendering; rustfmt passes both projected test files.
No production behavior, test assertion, live count/order, origin or bound changes.

Fresh actual full default images, the same paired 65 cases and strict all-feature
test-cfg Clippy follow under the original owned envelopes. This is not a rerun of
the consumed r/f2 runtime candidates: it proves the archive-only distribution
path and separately formatted consumer. First failures remain. Root security/
policy/adoption, complete package/workspace and original execution are still
separate gates; no service/system bus/server/product or performance hypothesis.
