# Local signed intent projection evidence

Baseline: `1b45dc2b9ee2df21ecd244361b0ec471313f9516`.
Preregistration: `8cba5f18`.
Implementation: `6f40dba19a8bcd4c2d373dfa31e4dbec3ce5345c`.
This working-tree packet is local safety evidence, not a release receipt.

The [contract](../../diagnostic-intent-projection-local-contract.toml) and
[design](../../diagnostic-intent-projection-local-design.md) define the exact
signature/domain, canonical budgets, original-worker borrow and refusal order.
The [manifest](manifest.json) hashes every closed raw log; run
`C:/Python313/python.exe verify-packet.py` from this directory for read-only
verification. The verifier checks source trees, test rows, repeats, failed
attempts, registry wiring and unchanged frozen qualification inputs.

Reproduce the Linux scopes with Rust 1.94.0 and locked dependencies:

```text
cargo +1.94.0 test -p hydracache-long-run-supervisor-074 --locked --lib --test diagnostic_intent_projection --test diagnostic_worker_policy --test diagnostic_task_api --test diagnostic_lease --test diagnostic_ipc -- --test-threads=1
cargo +1.94.0 test -p hydracache-long-run-supervisor-074 --locked --test diagnostic_intent_projection -- --test-threads=1
cargo +1.94.0 test -p hydracache-long-run-supervisor-074 --locked --lib intent_projection
cargo +1.94.0 test -p hydracache-long-run-supervisor-074 --locked --doc
```

The serial baseline is the first command without `--test diagnostic_intent_projection`,
at the baseline source. Root contract/evidence/governance tests and packet
verification run natively on Windows, without Git metadata overrides, because
some tests create independent temporary Git repositories. This is scoped local
verification, not a new full-workspace milestone receipt.

Results: 16 new integration and two new unit tests, three serial integration
repeats, 451 scoped Linux passes with one existing ignored, 195 portable Windows
passes, five compile-fail doctests; root contracts/evidence/governance 106/13/23.
Strict supervisor and xtask lint, Linux/Windows all-targets checks and MSRV 1.88,
Linux rustdoc, formatting, docs, local performance contract and book build pass.
Windows excludes Linux-only behavior; its success is portability evidence only.

Negatives retained: initial parallel baseline has an existing `Text file busy`
failure (371 passed, one failed, one ignored); serial baseline passes 433 with
one ignored without implementation changes. The API-absent test fails before
implementation. Initial root governance rejects an unregistered Linux target;
the first registration attempt uses an unsupported `mode` field, followed by
the corrected existing CI `step`. A prior packet verification attempted before
this README existed rejects the then-missing link; the final retry passes.
These are not successful proof rows. Genuine qualification dry-run refuses the
inherited plan digest mismatch, and ship admission remains closed.

No durable intent, anti-replay, fresh kernel peer/clock, original runtime/files,
production issuer, host, start/restart, workload or performance claim is proved
or introduced. All keys and host contexts in these tests are synthetic. Next
work is fenced durable intent/one-start recovery and original runtime composition,
not promoting this projection into a start permission.
