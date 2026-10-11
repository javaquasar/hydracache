# Original signed guard / authority journal: local refusal bridge

This slice binds the existing original signed task guard and authority journal
exclusively. It is not a launch capability or start/restart lifecycle. Baseline:
`e515d264df47af9694c500d94fab483963441bd6`, tree
`54d39f6e784dc7a29d6764e2653d586534340660`.
The [contract](diagnostic-signed-authority-local-contract.toml) is preregistered
before implementation; the historical fixed-backend contract is unchanged.

## Check and failure rules

The wrapper saves only the external trust and asserted host already retained by
the original checked guard. Caller-supplied replacement pins cannot select the
journal to which a failure is written. Fixed and fixture origins have distinct
typed constructors. Neither exposes a path, owner, PID, replacement policy,
epoch transition, clone, or reset. Both original objects remain exclusively
borrowed; no fresh guard is constructed.

Each check is journal / original signed tasks / journal. A journal error refuses
the original guard and journal in memory without appending to unsafe or foreign
storage. An already committed refusal stops before the signed step, without an
extra record. A signed failure refuses the whole original chain, then inspects
the journal again under the saved original pins. Only a safe matching active
journal receives an append-only refusal. The first typed failure is preserved;
an unacknowledged append additionally reports the persistence failure. There is
no retry, rollback, repair or adoption after a failure. Later calls do no I/O.
Explicit operator refusal follows the same persistence rule. Healthy drop does
not write. Failed drop does not silently retry a write.

## Required local evidence

Unchanged signed-task and journal tests run first. A missing-public-API red
precedes the implementation. Tests then cover exact check ordering, healthy
drop, explicit refusal/reopen, original policy/context drift, foreign binding,
already refused originals, stale epochs, task churn and unsafe journal changes.
Four existing publication-stage fault points exercise the real append engine:
partial pairs refuse reopen; a complete but unacknowledged refusal reopens as
refused. Fault injection is private and test-only, not a public selector.
Fixed dispatch can be modeled locally but is not root-owned host enrollment.

## Limits and next boundary

If storage cannot safely accept a refusal, an in-memory latch is **not** proof
of a durable refusal. A crash before append, or a storage error leaving an
otherwise intact active journal, is not covered by this runtime bridge. A
separate reviewed lifecycle intent protocol is needed before restart can treat
uncertain execution as durably refused. No account, issuer, initial namespace,
server, production workload or physical power-loss test is introduced here.
Qualification remains closed, including the inherited frozen-plan digest
mismatch; this slice does not refresh its manifest or thresholds.

The subsequent [fixture recovery slice](diagnostic-fixture-recovery-local-design.md)
now composes existing diagnostic state/IPC intent checks with this original
fixture bridge. It durably refuses only safe exact-bound original authority and
never replays or repairs lifecycle state. This later local addition is not the
authenticated production intent protocol or controlled fixed-root enrollment
required above; the original preregistered contract remains historical.
