# Isolated full-workspace D-Bus adoption review

`review.py prepare` creates an exclusive metadata/source projection, never edits
root product files. It reads the pinned qualified revision-2 consumer and exact
archive recipe, copies all tracked files under the 36 workspace members, then
projects the 22 already-qualified source migrations and 266 canonical vendored
members. Original source preimages must match the qualified baseline. Only the
two known source additions are permitted. Original deny/qualification inputs,
MSRV 1.88 and all 36 product members remain exact.

Conservative static closure includes normal/build/dev dependencies for every
target, inherited workspace aliases and local paths. It refuses incomplete or
external/ambiguous paths. None of the 23 publishable roots reaches the unpublished
supervisor, which is the only direct zbus caller. This permits examining vendored
sources for that tool without claiming registry qualification for published
HydraCache dependencies. Chitchat's rejected preview remains rejected.

The projected root excludes only the three vendored dependency crates from being
workspace test targets, retaining all product members. They must still resolve
and build through the supervisor's normal/dev edges. The direct Linux dependency
is exact zbus 6.1.0; only its test declaration enables hc074-unix-fixture. Two local
patches bind zbus_macros 6.1.0 and zbus_utils 6.0.0. All archive bytes retain their
explicit modified-backport provenance. No package is published or root-adopted.

After committing the inputs, `run.py metadata` runs exactly one offline full
Cargo metadata resolution on this copied workspace. Only its copied Cargo.lock
may change; the source, real root and HEAD are guarded separately. The original
Windows Job owner keeps the 1200-second outer, 1100-second child, 4 GiB floor,
five-second polling and retirement. Only metadata/version probes execute; no
compilation, build scripts, tests, product or host manager execution follows.
The fixed Cargo/rustc images and configuration are recorded; arbitrary caller
compiler wrappers/cache overrides and secret environment values are not forwarded.
Stdout/stderr/refusal receipts exist before preflight, including missing-freeze
errors. A consumed output is never reused.

Actual metadata must contain all 36 product members and exactly the reviewed
local packages, no old zbus backend, foreign registry or incomplete resolved
node/edge set. A metadata pass is not acceptance of newly resolved versions,
checksums, MSRV or feature effects: exact lock/source/effect review follows,
then compiler/formatting/policy/root package/workspace gates. No root or server
admission, duration change, new exception or performance claim follows.

Ten inert controls cover published reachability, path/manifest drift, the exact
archive inventory, command/environment bounds, capacity exception identity,
source-versus-lock mutations, complete graph membership and preflight logs. The
first LF-only marker control refuses the root's CRLF manifest before projection;
its source and observed summary are retained. The correction preserves existing
line endings and parsed original fields rather than normalizing root files.
