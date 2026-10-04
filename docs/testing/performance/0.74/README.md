# HydraCache 0.74 RESP/native performance evidence

This directory began W0/W1 from the exact frozen 0.73 product candidate
`16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b`. Release 0.73 is now published: annotated tag
`v0.73.0` points to `d1db9937e61295341ac95f289bace275641b1650`, whose product runtime is
verified unchanged from the measured candidate. `B73` deliberately pins both identities plus the
immutable confirmation archive commit `570a5bcb6959ecc7f01f8c80d0fc32b719832ad9`; W0 is closed,
but local 0.74 numbers remain non-promotable.

The scenario matrix keeps RESP, HC/1, HC/2, direct `ClientSurfaceState`, raw embedded
`HydraCache`, and typed embedded `HydraCache` results separate. The three execution tiers are
also separate: `local-quick` is a developer smoke, `local-attribution` is non-promotable D1
evidence, and `release-qualification` requires explicit authorization on an admitted host.

Run the structural gate locally with:

```text
cargo xtask performance-contract-check --release 0.74
```

The gate validates the frozen dimensions, native non-regression rules, workload-equivalence
fields, proposal isolation, required metrics, and the published predecessor identities. It must
still fail if asked for ship admission while the 0.74 candidate, Redis provenance, dedicated-host
qualification, or release evidence is incomplete.

The first non-promotable local attribution and the rejected W2/W3 results are documented in
[`w1-local-attribution.md`](w1-local-attribution.md). Raw identity-bound receipts are retained under
`local-runs/`.

The standalone `tools/resp-stage-profile-074` tool isolates decode, translation-context,
command/translation and response/encode allocation owners. Its control stages permit only local
incremental attribution; they are not interchangeable with end-to-end product receipts.

The focused WSL2 portability result is recorded in
[`local-linux-sanity.md`](local-linux-sanity.md). It is a non-promotable `local-quick` sub-tier and
does not substitute for the admitted Linux release host.

The controller-resilience contract is frozen in
[`long-run-controller-resilience-contract.toml`](long-run-controller-resilience-contract.toml).
The first local implementation slice, `tools/long-run-supervisor-074`, writes and independently
verifies canonical hash-chained checkpoint envelopes, rejects identity/timestamp/hash drift, and
recovers at most one incomplete trailing line. This is not yet the live systemd supervisor: Unix
socket authorization, process/cgroup ownership, attach leases, provisioning and real host fault
rehearsals remain incomplete and release admission stays closed.

Offline packet verification is independently implemented in `xtask` (it does not call the
supervisor library):

```text
cargo xtask long-run-campaign-check --release 0.74 --manifest <packet-manifest.json>
```

The same local crate now contains the pure campaign state machine, request replay map and attach
predicate evaluator. Deterministic tests prove that attach changes only controller lease/revision,
cannot spawn or restart a role, and rejects host/boot, PID start, cgroup, checkpoint, lease,
revision, duplicate-executor, recorded-failure and durable-history drift.

The versioned request/response protocol is also locally implemented with the frozen 65,536-byte
limit, strict unknown/duplicate-field rejection, UUIDv4 and lowercase-digest validation,
operation-specific fields, the single exact staging path for `start`, and canonical response
digests. It intentionally exposes no argv, environment, shell, DBus or arbitrary-path field.

`scripts/perf/performance_long_run_074.py --prepare` builds only a create-new start manifest. It
uses the frozen campaign-id component order, removes the raw nonce after deriving its digest,
rejects unknown, floating-point and secret-bearing fields, fsyncs both files, and has no process
execution mode.

W11 is staged, but disabled, by `qualification-manifest.toml`. Its contract inputs are
content-addressed and its expensive phases remain `not-run`. The only currently supported action
is a no-execution validation:

```text
python scripts/perf/performance_qualification_dry_run_074.py --dry-run \
  --manifest docs/testing/performance/0.74/qualification-manifest.toml \
  --output target/performance-evidence/0.74/qualification-dry-run.json
```

The dry-run refuses digest drift, reordered phases, hidden blockers, or admission of an expensive
phase. It deliberately has no execution mode before the 0.74 candidate identity, admitted host,
Redis binary identities, qualification runner and explicit authorization are available. The
predecessor tag and confirmation are no longer blockers.

Numerical receipts must never be hand-edited into claims. Every receipt binds the trace, payload
and key corpora, seed, warmup, duration, offered schedule, concurrency, pipeline depth, security,
persistence and final-state digest. Errors, timeouts, rejections, late operations and incomplete
operations stay in the goodput denominator.

Local paired work uses `local-harness.toml` and
`scripts/perf/performance_local_pairing_074.py`. The runner fixes a five-pair ABBA order, requires
warm-up, applies affinity and process priority, rejects attempts that start above the frozen
background-CPU ceiling, and derives a minimum detectable effect from same-binary A/A deltas. Its
output is always non-promotable; a result below that A/A-derived floor is `inconclusive`, not a
product win. Unit-test the runner with:

```text
python -m unittest scripts/perf/test_performance_local_pairing_074.py
```

W10's checked-in `composition-ledger.toml` currently records zero accepted product candidates.
It therefore forbids a synthetic C74 freeze or addition of isolated percentage gains. Tooling and
evidence work can continue, but composition remains a no-op until an isolated proposal actually
passes its native, semantic and local performance gates.
