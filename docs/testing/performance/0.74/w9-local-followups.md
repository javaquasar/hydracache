# 0.74 W9 local follow-ups

Status: **non-promotable local evidence**. This file records terminal local decisions; it does not
authorize release claims or expensive infrastructure.

## W9a atomic ordering audit

The source registry classifies every `SeqCst` atomic in the measured RESP and direct client-surface
paths. Diagnostic counters were the only group eligible for a `Relaxed` screen. Request/message
identity, subscription lifecycle, monotonic time-floor and expiry-claim atomics retained their
existing ordering because they need separate publication, lifecycle or happens-before proofs.

The preregistered isolated screen used commit
`ce7c5df1b7275b5a2d0183dca71afde9e023f2ba`, seven counterbalanced pairs, exact operation counts,
process CPU/op and a 2% minimum effect with at least six of seven pairs favoring the candidate.

| Cell | SeqCst median CPU/op | Relaxed median CPU/op | Relaxed-faster pairs | Decision |
| --- | ---: | ---: | ---: | --- |
| concurrency 1, 20,000,000 increments/sample | 7.03125 ns | 7.03125 ns | 0/7 | reject |
| concurrency 8, 8,000,000 increments/sample | 142.578125 ns | 142.578125 ns | 4/7 | reject |

Both cells missed the frozen floor. Windows process CPU accounting also quantizes the single-thread
samples, but that limitation cannot turn zero median improvement and zero favorable pairs into an
admission. Because the isolated screen failed, no product candidate was created and no RESP/native
end-to-end percentages are claimed. All product atomics remain unchanged.

## Remaining W9 decisions

- **W9b:** connection-local reuse remains open only after a measured buffer owner survives the W3
  rejection; there is no cross-connection pool authorization.
- **W9c:** syscall/socket tuning remains blocked locally because kernel syscall attribution is not
  available on this Windows/WSL setup. Platform defaults remain unchanged.
- **W9d:** durable group commit remains unauthorized pending a separate durability/COMPAT decision.
- **W9e:** allocator work remains deferred until a supported dedicated-Linux owner profile exists.
