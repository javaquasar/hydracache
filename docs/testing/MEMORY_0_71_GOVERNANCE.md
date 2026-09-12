# Memory 0.71 governance contract

Release 0.71 separates structural readiness from numerical evidence. The
commands in this document are safe to run before a dedicated host exists; none
of them claims that D0, a candidate, or a release is accepted.

## Structural admission

Run the following from the repository root:

```text
cargo xtask memory-owner-inventory --release 0.71 --check
cargo xtask memory-contract-check --release 0.71
cargo xtask memory-campaign-check --release 0.71
cargo xtask canary-check --release 0.71
cargo xtask release-governance-check --release 0.71
```

`memory-owner-inventory --check` is read-only. It rescans publishable
production Rust and requires every conservative ownership candidate to be
closed by the reviewed registry. `memory-contract-check` aggregates the
decision, statistics, allocator, compatibility, release-policy, host-profile,
and ownership contracts. `memory-campaign-check` accepts an absent campaign
directory before ship, but validates every retained campaign receipt it finds.

## Ship admission

The two `--require-ship` commands are deliberately red until real evidence is
available:

```text
cargo xtask memory-campaign-check --release 0.71 --campaigns <external-campaign-root> --require-ship
cargo xtask release-evidence --release 0.71 --require-ship
```

The campaign check requires a ship-eligible successful candidate receipt with
immutable identity fields, complete job counts, and the M10 24-hour case. Only
an admitted evidence-mode candidate can emit that eligibility. Release evidence additionally
requires exact-commit fast, gated, and dynamic-canary receipts for W0-W13.
Missing files, malformed JSON, identity drift, a non-success result, or an
incomplete candidate remains a hard failure.

On Windows, `cargo xtask verify` formats packages separately to avoid the
command-length limit and runs Clippy for the default workspace plus the
supported mimalloc allocator feature. The jemalloc all-feature lane remains a
Linux CI responsibility because its native build is not supported on Windows.
Generated Python sources, retained Maven POM files, and every 0.71 contract
covered by an immutable digest are checked out with LF line endings. Byte-exact
generation and baseline hashes therefore remain portable when Git is configured
with `core.autocrlf=true`.

## Current boundary

The governance files and commands are implemented, while D0 remains `ready =
false`. The completed 0.67.1 archive is retained under
`docs/testing/perf-artifacts/0.67.1/`, but point 2 must still materialize its
historical mirror receipt, qualify a dedicated Linux host, measure
instrumentation overhead, and approve D0. No numerical memory claim is valid
before those receipts exist.

The protected workflow plans and builds the immutable B1 cohort before final
campaign admission because S5 must measure that exact binary. On the first
baseline dispatch it executes the daemon in `off`, `production`, and `profile`
modes across cold, small-hot, tag-heavy, HC/2-1000, and reset workloads. Mode
order rotates for at least three repetitions. The receipt is written only after
every sample succeeds; candidate dispatches cannot create or replace it.

Admission binds that receipt to the B1 source and binary, scenario digest, and
stable host fingerprint. The fingerprint contains held identity facts such as
hardware topology, kernel/OS, RAID, and configured policy. Free memory,
temperatures, process identifiers, and competing-load observations remain in
the preflight receipt for diagnosis but do not create false host drift on
resume.
