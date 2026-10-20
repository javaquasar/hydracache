# Starting effects and surrounding validation cost

Baseline is `4d0f461c638d158ff62dbd3e0f8f9914d523581e`. Local observation only:
no server access, installation, service/clock operation or product workload.
Preserve all live checks, error/effect/checkpoint order and original 300/60/10
bounds. No fsync removal, batching, durable acknowledgement change or reuse of a
successful validation round is authorized by an elapsed-time observation.

Add inactive test-only boundary marks around encoding, begin, each opening
round, each storage step, checkpoint, closing round, final round and acknowledgement.
Inside storage separate opening inspection, phase checks, actual effect and
closing inspection. Keep exact source reconstruction to the baseline, including
all nine stage bodies and three sync_all sites. Successful observations must
contain each expected boundary in exact stage order. Refusals remain original
refusals; incomplete profiles are retained, not retried into a healthy cohort.

Run three serial actual healthy local profiles per GNU 1.94 and MSRV 1.88 lane,
bound to source, image and recipe. Verify unchanged parent kernel/live/auth
counts, byte-identical original synthetic bundle and qualification manifest.
Elapsed/process CPU includes observer overhead; process CPU excludes children.
These instrumented observations are not functional admission gates or disk
latency qualification. Distinguish direct sync_all intervals from the complete
storage step and the surrounding full validation rounds.

Select only a measured avoidable cost with a semantics-preserving mechanism.
Before any runtime adoption, freeze its control, add differential/fault tests
and require repeatable mechanism evidence. If effects do not own the cost or
the remaining owner is mandatory fresh validation, record that negative result
and adopt no speculative runtime candidate. Preserve old failures and packets.

Finish with exact-source guards, targeted storage/publication/crash/join tests,
both unchanged unprofiled local five-case matrices, strict affected-package
lint, formatting, docs, registry and local non-promotable performance contracts.
New boundaries do not constitute new SIGKILL coverage. Native composition,
authenticated delivery, cessation of all writers, later lifecycle cells,
reconnect and production enrollment remain separate closed gates.
