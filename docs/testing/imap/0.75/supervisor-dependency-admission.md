# Supervisor dependency admission for 0.75

## Scope

The integrated source exposed RUSTSEC-2026-0370/0371/0372 through the Linux campaign
supervisor's zbus 5 graph. Replace that client with `dbus 0.9.12` and `libdbus-sys 0.2.7`.
Both internal tools explicitly inherit the workspace's Rust 1.88 minimum and Apache-2.0 license.
No dependency-policy exemption, advisory ignore or coverage-threshold change is used.

The replacement preserves the fixed systemd destination and interfaces, `StartTransientUnit`
mode `fail`, empty auxiliary units, property types and argv/environment boundaries. Every
D-Bus method now has a bounded 25-second timeout; the former connection configured no method
timeout. Only the exact `org.freedesktop.systemd1.NoSuchUnit` error becomes an absent unit.
Other transport, permission and decoding failures remain errors.

The implementation uses the operating system's libdbus. Ubuntu builders need `pkg-config`
and `libdbus-1-dev`; isolated integration tests also need `dbus-daemon`. The shared CI setup
installs these packages on GitHub-hosted Linux and checks them on self-hosted Linux without
changing that host's packages. The prerequisite step is present in the workspace, MSRV,
registered-proof, coverage and IMap matrix jobs.

This dependency belongs to campaign tooling. The product daemon and map protocol gain no
D-Bus endpoint, operation, wire number or durable identity.

## Tests and local evidence

`tools/long-run-supervisor-074/src/systemd_unit_dbus_tests.rs` starts an owned, isolated Unix
D-Bus daemon and a scripted systemd peer. Its four tests verify actual IPC for all six property
shapes, argument bytes, object-path decoding, all five property interfaces/types, exact missing
unit classification, permission errors, malformed replies and unit-name rejection.

On Linux, Rust 1.88 passed 136 tests across the supervisor and integrated checkpoint writer;
one existing system-bus test was initially ignored. That test was then run explicitly against
the real local system bus and passed. Package `check --all-targets` passed on Rust 1.88, and
strict all-target/all-feature Clippy passed on both Rust 1.88 and 1.94.

The Linux checks also exposed existing warnings in Linux-only modules. Redundant error
wrapping was removed, the large admitted-start payload was boxed, the private ready/pending
observation uses `Option`, and unit-test modules follow implementation items. Existing lifecycle,
admission, replay, identity and failure tests retain their assertions.

`cargo deny check` passed advisories, bans, licenses and sources with the original policy.
Logs are retained in `target/imap-075/supervisor-dbus-20261125/`. The complete `verify` and
the four W12 lanes must pass before this preparation is treated as the new qualified baseline.

```sh
cargo +1.88.0 test --locked -p hydracache-long-run-supervisor-074 -p hydracache-performance-integrated-074 --all-targets
cargo +1.88.0 check --locked -p hydracache-long-run-supervisor-074 -p hydracache-performance-integrated-074 --all-targets
cargo +1.88.0 clippy --locked -p hydracache-long-run-supervisor-074 -p hydracache-performance-integrated-074 --all-targets --all-features -- -D warnings
cargo +1.94.0 clippy --locked -p hydracache-long-run-supervisor-074 -p hydracache-performance-integrated-074 --all-targets --all-features -- -D warnings
cargo +1.88.0 test --locked -p hydracache-long-run-supervisor-074 --lib systemd_unit::tests::reads_a_real_loaded_service_over_dbus -- --ignored --exact
cargo deny check
cargo xtask verify
```

## W0/D1 handoff

[ADR-0024](../../../adr/0024-imap-quorum-backed-read-barrier.md) records the quorum-backed read
barrier already selected by OD-1. Its implementation and per-read cost proof remain D2/D3 work.
The other owner decisions stay open. This admission does not select clock uncertainty, recovery
or latency targets, release fallback or dates, and does not enable production distributed IMap.

D1 still requires the mechanical local-backend extraction, asynchronous dispatcher, unchanged
local protocol/conformance results and matched measurements against the exact published B74.
The [B74 intake](d1-b74-intake.md) now retains a signature-verified public ELF and a passing
isolated Linux smoke check.
The current local/backend characterization tests are its guardrails. A dependency-policy pass,
model coverage or a W12 foundation receipt cannot substitute for the D1 performance gate.

Sources: [dbus API](https://docs.rs/dbus/0.9.12/dbus/),
[GitHub runner contexts](https://docs.github.com/en/actions/reference/workflows-and-actions/contexts#runner-context).
