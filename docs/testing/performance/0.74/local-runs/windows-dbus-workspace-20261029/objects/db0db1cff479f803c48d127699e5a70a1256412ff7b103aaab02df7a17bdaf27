# Local zbus source package and consumer gate

This packages the already tested standalone zbus 6 / Rust 1.88 backport. It is
not root dependency integration, registry publication, a signed build receipt,
production enrollment or original-manager/all-writer admission.

`patches/recipe.json` identifies the exact three upstream archives, every base
and final member, each whole-preimage replacement and each deterministic source
tar. Readable per-crate diffs accompany content-addressed replacement objects.
`package.py` rebuilds in memory only: no archive code is imported or executed and
no archive is extracted to caller-selected filesystem paths. Links, aliases,
duplicate members, stale pins and foreign entries refuse. Canonical archives
have fixed sorted names, UID/GID/mtime zero, empty owner names, mode 0644 and no
compression input. Two reconstructions must match byte for byte.

The maintenance boundary is HydraCache's local backport source revision, recorded
in every added BACKPORT074.json. Upstream names/versions/licenses remain visible;
these modified archives are never represented as pristine crates.io packages.
The retained upstream VCS and packaged locks are provenance/data only. Standalone
authoring manifests mirror the exact previously tested normalized manifests.
No version/feature/compiler-manifest or Rust body changes in this packaging step.
Publication naming/source identity and registry-consumer approval remain separate.
`python -B tools/dbus-source-package-074/run.py rebuild-only` reconstructs the three
source tars in the exclusive `target/dbus-source-package-20261028/export` directory
using retained repository inputs only. It does not need old build/target/native
trees, contact a registry, invoke Cargo, install anything or execute archive code.
The unsigned receipt distinguishes this local export from published packages.

The local consumer receives only the reconstructed archives' verified source
members plus the prior independent test overlays. All 67 prior tests remain,
including original ReplyState, runtime, FD, framing and ancillary controls. Its
complete 62-node lock is unchanged. The independently pinned existing CI owner
keeps offline/locked commands, original 1200-second outer/1100-second inner
origins, 4 GiB floors, five-second polling/retirement and all existing frame guards.
GNU Rust 1.94 and 1.88 use separate target trees and complete actual catalogs.
No clock, services, bus manager, server or product workload is involved.

The fixed preparation is `python -B tools/dbus-source-package-074/run.py prepare`;
the fixed session is `python -B tools/dbus-source-package-074/run.py session-io-safe`.
Outputs are exclusive: preserve first results; do not delete a receipt to rerun.
Inert controls use `python -B -m unittest discover -s tools/dbus-source-package-074
-p test_controls.py -v`. Source package verification is not a new performance
hypothesis and does not reopen Chitchat's declined preview candidate.
