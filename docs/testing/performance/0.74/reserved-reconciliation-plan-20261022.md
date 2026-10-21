# Offline reconciliation of retained ReservedLive allocation candidates

## Scope and acceptance

Starting tree: `1e5f31d9d443bf6c9b1d901781986501425bf9e9`. Reconcile the
five retained allocation leaves against the sealed remaining-leaf audit and
its original material trace. Use a new offline tool and packet; do not update
the older source-pinned auditors to accept a successor runtime.

Acceptance requires raw verification of all eleven input inventories and their
491 members, exact current source witnesses, reconstruction of the old round
facts, and separate GNU/MSRV allocation cohorts with baseline/candidate source
identities. Keep the eight unresolved owners distinct from the five retained
allocation candidates. Do not sum different corpora, extrapolate old call counts
to current savings, or infer CPU, RSS, retention, latency or readiness.

Pure mutation tests must refuse changed source, input seals/membership, unsafe
paths, ambiguous witnesses, changed cohort identity/counts, changed first
refusals/windows and promoted gates. Run the existing arithmetic/reader tests,
focused Windows source canaries, xtask check/strict clippy and structural gates.
Retain the first result of every check; fixes need separate output identities.

No runtime, observer, workload, validation count/order, 300/60/10-second window,
qualification manifest, compatibility or release-state change is included.
There are no new measurements, WSL profiles, host clock/service changes,
server actions or product workloads. Material, cleanup/EOF/fence, authenticated
delivery/all writers, SIGKILL/lost-ACK, reconnect and server gates stay closed.

## Follow on

Choose the next isolated pure baseline only after reviewing the unresolved
owner's lifetime and first-error contract. A full-path timing comparison and
fault gate remain separate admitted work, not an outcome of this reconciliation.
