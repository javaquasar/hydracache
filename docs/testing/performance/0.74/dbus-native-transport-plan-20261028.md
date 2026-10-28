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
