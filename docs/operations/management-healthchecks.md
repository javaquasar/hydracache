# Management healthchecks

HydraCache 0.72 evaluates Management Center health on the server from one immutable typed input.
The browser may filter and display results, but it must not own thresholds or derive a status.
`UNKNOWN` means that a required source is missing, stale, partial, modeled, truncated, or
incomparable. It is never an alias for `PASS`. `DISABLED` is reserved for an explicitly disabled
configured check and is not emitted for missing inputs.

The headline aggregate is the worst known status (`FAIL`, then `WARN`, then `PASS`). UNKNOWN and
DISABLED counts remain separate, including when the headline is PASS or FAIL. All thresholds report
their unit, reviewed source, and evaluation version. Status can improve only on a strictly newer,
same-epoch coherent observation; equal-sequence conflicts retain the already published verdict.

| ID | Meaning | Remediation code | Required evidence |
| --- | --- | --- | --- |
| `HC-AUDIT-001` | Configured audit sink accepts bounded events | `inspect-audit-sink` | Explicit enablement and sink health |
| `HC-AUTH-001` | Authority quorum and leader are accessible | `inspect-authority` | Quorum and leader observations |
| `HC-CLIENT-001` | Client/session and buffer ownership remain within limits | `inspect-client-pressure` | Active-owner counts and configured limits |
| `HC-EXPIRY-001` | Expiry backlog remains within its limit | `inspect-expiry-backlog` | Backlog and configured limit |
| `HC-FORM-001` | Every committed member completed authentication, admission, catch-up and serving transitions | `inspect-formation-blockers` | Complete same-epoch member formation rows |
| `HC-HISTORY-001` | Explicitly enabled history source is available | `inspect-history-source` | Enablement and adapter availability |
| `HC-MEMBER-001` | Reachability, protocol version and configuration digest agree | `inspect-member-skew` | Complete member compatibility observations |
| `HC-MEM-001` | Retained bytes and admission queue remain within limits | `inspect-memory-admission` | Owner bytes, queue depth and configured limits |
| `HC-PART-001` | Partitions are assigned, replicated and zone-spread | `inspect-partitions` | Current-epoch ownership and repair counts |
| `HC-PERSIST-001` | Enabled persistence has disk and fresh backup evidence | `inspect-persistence` | Explicit enablement, disk state, backup age and limit |
| `HC-RAFT-001` | Commit/apply lag is below reviewed entry-count thresholds | `inspect-raft-apply-lag` | Complete coherent commit and applied indexes |
| `HC-PLACE-001` | Requested replicas were selected and the placement was applied | `inspect-placement-trace` | Complete current-epoch decision trace and applied progress |
| `HC-REPAIR-001` | Repair debt is clear and degraded mode is inactive | `inspect-repair-debt` | Debt count and degraded-mode state |
| `HC-REPL-001` | Replication has no failures or backpressure | `inspect-replication` | Failure and backpressure counts |
| `HC-RECOVERY-001` | Latest durable recovery completed safely | `inspect-durable-recovery` | Complete live latest recovery attempt |
| `HC-RECOVERY-002` | No verified unaccounted corruption or loss remains | `inspect-corruption-evidence` | Complete unsaturated checked/discarded/corrupt counts |
| `HC-RECOVERY-003` | Authoritative desired/local reconciliation completed | `inspect-reconciliation` | Complete live recovery phase and repair result |
| `HC-RESHARD-001` | Active reshard backfill remains within its reviewed progress bound | `inspect-reshard-progress` | Active state, lag and configured limit |

The initial reviewed Raft lag thresholds are 100 entries for WARN and 1,000 entries for FAIL.
Crossing is inclusive. They are compatibility-visible defaults, not JavaScript constants; changing
them requires a reviewed configuration/default update plus boundary and browser evidence.

Operators should follow the remediation code to the corresponding read-only Formation, Consensus,
Placement, or Recovery view. The 0.72 console intentionally exposes no repair, retry, restore,
delete, drain, or “mark healthy” control.
