# Isolated D-Bus 6 / Rust 1.88 preparation

No root dependency, supervisor caller, live frame, deadline or deny exception is
changed. Do not contact a bus/manager/service or server. This is local preparation
of the existing security migration, not a new performance hypothesis.

Use checksum-reviewed zbus/zbus_macros 6.1.0 and zbus_utils 6.0.0. Keep the
unmodified published 1.89 floor as U; record its real 1.88 refusal first. M changes
only those declared floors to 1.88 for an actual compiler compatibility probe.
That declaration change is not an accepted backport: require real 1.88 build,
tests, source/API/feature closure, GNU, caller/wire/FD/credential, runtime/fault,
distribution and final policy gates. Never use --ignore-rust-version or rename an
unmaintained package to bypass policy. Root Cargo.lock and deny.toml stay exact.

Keep identical probe source and lock across U/M. Enable only explicit tokio and
proxy features without defaults. This does not cover default-rt, service or all
optional wire types. Pin reviewed zruntime 0.2.0 identically for the probe; no
source patch is applied to it. Keep source/lock/compiler-specific target paths,
actual artifact SHA and complete native catalogs. Zero tests is a refusal.
Original local envelopes are 1200 seconds, 4 GiB floor, five-second capacity
polling and five-second owned Windows Job retirement. Keep every first outcome;
no retry-to-pass, service/clock changes or production manager traffic.

The portable probe exercises derives/wire, name validation, method/reply
correlation and wrong-signature refusal, plus the upstream synchronous runtime
entry's 250-ms pending-future timeout and nested-entry refusal. This is neither
authenticated delivery nor evidence of runtime lifetime compatibility with the
old blocking API. Windows cannot test Unix FD paths; do not call it that gate.

The first resolved U graph also selected UUID 1.29.0 (Rust 1.89). M preserves its
refusal after changing only the three declarations. P retains the exact source
but selects the existing product's UUID 1.23.4 identity in the isolated lock;
no product lock changes. P's actual 1.88 build exposes two E0716 temporary-lifetime
errors in MatchRule Display. Q passes format_args directly at those two calls,
without creating a String. A restores upstream declarations/source on the same
compatible lock and adds the identical quoting/path regression for a 1.94 baseline.

F/F2 are retained inert formatting-decoder refusals: Rustfmt emits separate
canonical Windows child/parent headers and adds final argument commas. F3 uses
that same single captured projection after precise section/known-call validation.
It does not rerun Rustfmt projection or any failed native test. Final source has
only three normalized manifest floor changes and those two formatting call sites.
Three projection controls reject a missing/duplicate/foreign section, swapped
child/parent or an unreviewed argument change. Portable patches reconstruct all
263 upstream members; the authoring workspace manifests still need packaging
materialization. This is not a distributable or consumer-qualified package.

Fixed phases: `prepare-U`, `resolve-U-194`, `fetch-U-194`, `floor-U-188`,
`prepare-M`, `check-M-188`, `prepare-P`, `pin-P-194`, `check-P-188`, `prepare-A`,
`build-A-194`, `list-A-194`, `test-A-194`, `prepare-Q`, `build-Q-188`,
`list-Q-188`, `test-Q-188`, `check-Q-188`, `clippy-Q-188`, `format-view-Q-194`,
then the retained failed F/F2 preparations and corrected `prepare-F3`.
Use `format-F3-194` and build/list/test/check/clippy on F3 with both 188 and 194;
`policy-F3-194` uses the unchanged root deny policy. Invoke fixed phases with
`python -B tools/dbus-msrv-backport-074/run.py PHASE`. Exclusive output paths
under target/dbus-backport-20261028 intentionally reject reruns.

The helper is an experimental historical runner, not an installer or production
authority. Source archives/primary preparation and inert packagers are retained
in the evidence packet; native binaries/targets are excluded. The local graph's
green policy does not clear the unchanged HydraCache product graph.
