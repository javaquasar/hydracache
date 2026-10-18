# Two phase launch requirements and held barrier evidence

This local package adds separately signed v2 expected requirements and private
held-barrier mechanics. It does not build an executable systemd unit, send an
original message, prove watchdog readiness or admit a server. The
[plan](../../two-phase-launch-plan-20261019.md) starts from `db35fdc3`; plan commit
`c69e2a86` precedes implementation. The
[contract](../../two-phase-launch-contract.toml) keeps execution, enrollment,
whole-cgroup revocation, completion and qualification closed.

## Data and kernel boundaries

The new outer domain binds all original consent claims, three artifact roles
and deterministic expected launch semantics. Old v1 readers reject its envelope;
old consent and unit-projection bytes remain unchanged. Supplied-key inspection
returns claims only. No artifact is opened and no signed timestamp becomes a
live observation. The requirements document is not a TransientUnitSpec.

The private prototype owns one ephemeral seqpacket pair and expects one exact
32-byte nonce packet without ancillary data. A send attempt consumes its latch
before the syscall, including EPIPE and queue-pressure failures. ACK loss cannot
rearm it. The raw held fixture checks the original absolute BOOTTIME alarm and
two actual pidfds before and after reception. A ready guard wins over queued
release. Only after an accepted release does the fixture child write the
constant `effect` to its owned pipe; no workload executable or evidence file is
used. The lost-ACK case kills that exact child after its effect and before ACK.

The owner and guardian helpers are inert sleep processes. Their live pidfds do
not establish an independently armed watchdog. The fixture has no cgroup,
original authority, collector sink or fence, and cannot prove that death after
the last liveness observation revokes every writer. Production barrier delivery
needs a distinct reviewed protocol, actual guardian enrollment and the complete
original validation frame. All original 46 effect brackets and 300/60/10-second
origins remain unchanged in their existing source bodies.

## Retained failures and baseline

`baseline-windows.log` passes the original 44 controls. The old GNU image passes
36 controls with one ignored fixture in `baseline-gnu194-exchange.log`.
`baseline-gnu194-dispatch.log` used a filter matching zero tests and is not an
accepted baseline. `red-missing-module.log` retains the expected missing-module
compile failure before implementation.

The first healthy kernel barrier fails before ACK in `kernel-gnu194-first.log`.
This is a new implementation bug, not a deadline failure: recvmsg echoes the
requested MSG_CMSG_CLOEXEC policy in output flags. The independent socket probe
in `recvmsg-flags-kernel-probe.log` observes exactly `0x40000000`. The corrected
predicate permits only that bit or zero; every other bit, short/long packet,
wrong nonce and actual rights transfer still refuses. The fixed images also
test all 32 individual flag bits and send-pressure refusal. No existing delivery
predicate changes.

`source-two-phase-first.log` retains the new canary's incorrect payload-check
spelling. `source-contracts-first-full.log` retains a separate old lib.rs reader
that needed the exact new-registration adapter. The reader's assertions remain;
the adapter strips only the new module registration. Mutation controls and a
complete old-source/schema/registry comparison reject any other difference.
Preliminary logs are retained separately from committed-source cohorts.

## Verified committed source

Implementation is `fab1798c74f8a579184ec19893c2c76472421cbe`.
`gnu194-source/receipt.json` and `gnu188-source/receipt.json` bind that same
commit, nonroot UID 1000 and unchanged before/after test-image hashes. Each
cohort passes nine new ordinary controls, all twelve kernel cases, thirty old
exchange controls with one ignored private-bus matrix, and the separate
hidden-writer counterexample. The ignored matrix is not counted as executed.

Windows passes 49 portable controls and two compile-fail API controls. The final
registered-protocol source suite passes 262/262 and restores all 362 prior Rust
sources, old compatibility text/registry and the adapted old reader exactly.
Both GNU compilers and Windows pass affected all-target checks and strict
all-feature Clippy. Formatting, the 184 automatically discovered gate registry,
doc-check and the local non-promotable performance contract pass. The manual
barrier gate is explicitly registered, not an extra automatically discovered
target. The frozen qualification manifest remains byte-identical at
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

The packet inventory hashes UTF-8 text with LF newlines, so Git's Windows line
ending conversion does not alter the evidence comparison. Binary image hashes
in the cohort receipts are actual SHA-256 of the executed images. Preliminary
failed/empty-filter logs remain inventory entries, not accepted gates.
`./verify.ps1 -SelfTest` verifies the closed inventory and eight in-memory
negative controls without changing any receipt or granting runtime authority.

## Changed files

- Data and tests: `tools/long-run-supervisor-074/src/diagnostic_two_phase_launch.rs`
  and `diagnostic_two_phase_launch_tests.rs`.
- Private kernel mechanics and tests: `diagnostic_held_barrier.rs` and
  `diagnostic_held_barrier_tests.rs` beside the data module; one lib.rs registration.
- Source contracts: the new `performance_contract_074_two_phase_launch.rs` and
  exact adapters in `performance_contract_074_original_reply_binding.rs` and
  `performance_contract_074_start_consent.rs` under `crates/xtask/tests/support`.
- Compatibility/registration: `docs/COMPAT.md`, the gated-test registry and the
  new local contract. The plan, collector design, optimization article and this
  evidence packet record scope and remaining admission gates.

## Reproduction

From a clean selected source on this Windows and nonroot WSL Ubuntu workstation:

```powershell
./run.ps1 -Compiler 1.94.0 -Label fresh-gnu194
./run.ps1 -Compiler 1.88.0 -Label fresh-gnu188
```

The script refuses an existing cohort or dirty selected sources. It runs GNU
all-target checks, strict all-feature Clippy, new ordinary controls, the twelve
kernel cases, old exchange tests and the hidden-writer counterexample. It binds
the image hash before/after execution and requires unchanged source and frozen
qualification bytes. Reproductions create new untracked cohorts; do not treat
an extended directory as the original closed inventory.

Windows portable controls, compile-fail API tests, formatting, full source
contracts, registry, documentation and local non-promotable performance checks
have separate receipts. These are correctness/admission-boundary checks, not
latency, throughput, allocation or whole-workspace milestone measurements.

## Remaining integration

Pre-arm an actual independent guardian, bind original sinks and the whole owned
cgroup, and join the new artifacts to the original manager/frame before actual
delivery or barrier release. Both protocols must consume the same durable
attempt, never parallel v1/v2 replay slots. Revoke every external manager/creator
writer copy and observe both actual EOFs before collector completion and fence
release. Durable completion, later cells, complete new-boundary SIGKILL/lost-ACK,
reconnect and production enrollment remain later gates. No release installation,
supervisor operation, system-bus send, product workload or dedicated-server
operation occurred in this package.
