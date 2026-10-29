# Exact inventory after the approved Tokio update

The current test-source inventory has 545 files. Fixed producer
`2a4a771d48ed6ad54288fd15ea7c75b56f4bd80d` changes only
`diagnostic_manager_channel.rs` and adds `diagnostic_manager_alarm_tests.rs`
relative to `958b3135a15c00b9f6aad41522cef9d1f6608d3a`. The signed-off Tokio/FD
ownership implementation is not changed by this adapter.

The descriptor SHA256 is
`1f41899edf172c4ae0a29f17a5fd8a1cee0fd8bf1047c5da5c4ea94b1dca962b`.
`generate.py` reconstructs it from those two fixed Git trees, not HEAD, current
index or working files. It refuses every other delta and never writes files.

The Rust integration retains the original source-name, index mode/blob/stage,
filesystem inventory, alias/type and fresh canonical-body checks. Only after all
545 current bodies match can it release predecessor names through
`545 -> 544 -> 530 -> 446`. The immutable 530/544 descriptors and all physical
checks starting at `artifact_inventory_input::root` stay byte-identical.
The prior 544 tests still check their complete independent tree; their live
observations now include all old names plus the one new file exactly once.
Stale 544 input is not accepted as a current observation.

Seven new Rust controls check all current modes/blobs/canonical bodies against
the fixed producer, repeated real physical reads, stale input, changed/added
body failures, index corruption, inverse metadata mutation and preservation.
Git archive is read only as data. Its one global PAX header must contain exactly
the producer commit comment; every source entry must remain a regular file.
No archived source is compiled or executed.

```text
python tools/inventory-545-view-074/generate.py
cargo +1.94.0 test --offline --locked -p xtask --test artifact_inventory_phase_545_074 --test artifact_inventory_phase_544_074 --test artifact_inventory_reader_074 -- --test-threads=4
```

See the [scope and first outcomes](../../docs/testing/performance/0.74/inventory545-current-phase-result-20261027.md).
This repairs one preservation boundary. It does not turn the incomplete red V8
package receipt into green, admit a product candidate or start server activity.
