# Native historical blob transport controls

The isolated Rust Git transport matches 54 independent reference results across
six synthetic cases on both Rust 1.94 and MSRV 1.88. Nine decoder and six transport
tests pass on each toolchain; the Python suite passes 26 controls, and current
all-target check, strict Clippy and formatting are green. This is component
correctness evidence, not an optimization of HydraCache's native API, a measured
speedup, a full-package pass or server admission.

## Implementation and unchanged boundaries

Commit `da762d686ec585f872438b19d441ba01d2e5562a` adds `transport.rs`, `oracle.py`,
`test_oracle.py` and `crates/xtask/tests/historical_blob_transport_074.rs`, extends
the decoder and its controls, and updates the tool README. The original Python
reader, existing source guards, runtime and Cargo dependency baseline are unchanged.

The transport uses file-backed standard streams rather than pipe buffers. Input
is at most 256 requests/256 KiB, captured stdout/results at most 16 MiB and stderr
64 KiB. Sampled file sizes cause refusal, not a kernel-enforced disk quota.
Owned output vectors copy blob bytes; this is not a zero-copy or allocation claim.

One caller-supplied deadline covers batch, validation, fallbacks and closing checks.
The fixture uses a thirty-second Git-phase bound; cleanup has a separate five-second
reap bound. Input/pin identity refuses before dispatch. A failed or diagnostic batch
process, bad framing or changed pin refuses without retry or partial results.
Missing/non-blob results use the original `show` command with the same deadline
and replacement policy, retaining stdout, stderr and exit code.

Replacement policy is explicit. Default sends no replacement-disabling flag;
disabled sends `--no-replace-objects`. Short queries are sent unchanged, not
resolved and represented as full immutable IDs. A separate `ShortMissing` decoder
variant permits exact short-hex missing records. The original full-only `Missing`
still rejects short records, and a dedicated regression checks that boundary.

This transport is restricted to synthetic local Git context: no lazy fetch,
repository/config-override environment refusal and isolated system/global Git
configuration. Fixture-local configuration remains active. It is not equivalence
for arbitrary existing-loader environments or configuration. No existing loader
imports the transport, and no production pin authority is supplied.

## Independent Git and Python comparison

The oracle accepts only an existing empty test-owned root and creates two
disposable repositories. Commit timestamps are fixed fixture metadata; no host
clock changes. Separate Git commands obtain object type, ID and raw body before
native batch execution; body SHA256 pins are not derived from the native stream.
Independent `git show` supplies exact expected stdout/stderr/status. The unchanged
Python SHA1 prototype additionally checks the full-ID, replacement-disabled cases.

| Fixture state | Query form | Policy | Native/reference results |
| --- | --- | --- | ---: |
| Plain | Full ID | Disabled | 9 matched |
| Plain | Full ID | Default | 9 matched |
| Plain | Short ID | Default | 9 matched |
| Active blob replacement | Full ID | Disabled | 9 matched |
| Active blob replacement | Full ID | Default | 9 matched |
| Active blob replacement | Short ID | Default | 9 matched |

Every case includes duplicate binary lookup, empty bytes, CRLF/NUL/Unicode-path,
256-KiB value, symlink object, missing path, tree fallback and another commit.
The replacement control must change default-policy bytes while disabling
replacements must preserve original bytes. Missing diagnostics retain the original
query spelling. All 54 results match per matrix; the two Python-prototype cases
account for 18 of those results, not a separate 54-case prototype parity claim.

The native byte pins do not replace Git object identity with a trusted caller
assertion. Default replacement behavior is also not the Python prototype's
replacement-disabled SHA1 contract. Real source provenance, changing refs/object
database/configuration during observation and ambiguous abbreviations remain
unproved. The temporary oracle is a diagnostic producer, not production enrollment.

Git reports `2.51.0.windows.1`; the retained command path is
`C:\Program Files\Git\cmd\git.exe`, SHA256
`34a408843194be320d8a87a3c12cd5c7d2e08d03b24567a41db32e21d12569d2`.
This records the command shim, not every internal Git executable/library.

## Failure and cleanup controls

The native target has six tests, one being its exact child-fixture entry. The
other controls exercise the real comparison, preflight, batch refusal, explicit
nonzero/stdio/spawn results, and actual owned-child timeout/output overflow.
The nine decoder tests retain all eight older controls and add explicit short
missing syntax without weakening the full-only variant.

The actual cleanup fixture publishes a logical started-PID marker, then parks
or exceeds stderr/stdout budgets. Timeout and both output-budget refusals reap
only the owned child; an unrelated same-image child remains alive under its own
retained handle. These are Windows leaf-child controls, not Linux SIGKILL, process
tree/FD-alias revocation or original all-writer retirement proof. Cleanup errors
cannot publish a successful transport result.

A deliberately wrong native byte pin must produce `Protocol(Content)` rather
than an empty source or refreshed pin. Other controls retain missing executable,
nonzero exit, exact binary stdin and root-override refusal. Two Python scope
controls reject nonempty/file/missing roots before fixture mutation or Git execution.

## Evidence and disposition

All sixteen captured phases finish with zero exit and unchanged captured source
and HEAD. Exact source snapshots precede every phase. The first build and first
native matrix remain retained; subsequent controls add cleanup receipts, pin drift
and explicit byte-wise short syntax validation. Semantic repeats across versions
and toolchains are not independent speedup samples or retry-to-pass measurements.

The [sealed packet](local-runs/historical-blob-native-20261024/index.json) contains
328 logical members in 91 objects. Index SHA256 is
`c449aa67c4a4c2f3449e9bc4e27cd507d124e425c0faceae8db0063297e3c26b`.
It retains raw oracle/native outputs, cleanup receipts, all first source versions,
phase inputs/results/logs and coordinators. Every native result is rechecked
against its oracle during sealing. Raw bytes are not normalized.

Before integration, test mutable refs/config/object state and ambiguity, establish
independent source-pin provenance, validate supported platforms, and account for
reference preparation, spooling and copies. The oracle intentionally executes
extra reference commands; its cost is not free and is not measured here. Then
select one genuinely eligible existing caller and verify its fresh-observation
order/count/error semantics before any unchanged-bound package or effect screen.

The [previous call-site audit](historical-blob-rust-audit-result-20261024.md) remains
unadmitted; six synthetic cases do not approve its six real source sites. The old
interrupted package, original delivery/writer/lifecycle/trust barriers and release
qualification remain open. ReservedLive frequency/order, original 300/60/10-second
windows, frozen 0.73 and qualification manifest digest
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc` are unchanged.
There is no server, service, clock, original-owner, product workload, installation,
qualification, numerical performance claim or push action.

The later [mutation controls](historical-blob-fault-result-20261024.md) preserve
these first outcomes but expose a fallback ABA after successful batch validation.
Its consistent-view admission gate is false. Latest combined regression commands
also retain full-oracle deadline failures; this earlier complete component pass
must not substitute for those later verdicts or admit a real caller.
