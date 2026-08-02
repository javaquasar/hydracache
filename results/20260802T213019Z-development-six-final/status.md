# Six-experiment run status: intermediate diagnostic

This is a preserved diagnostic attempt, not valid qualification or bootstrap
evidence. Bundle SHA-256: `c93f19a128a1c58401de525411ad96953582ba1ed4b845d64c977d019f463d78`.

CPU metadata and per-case summaries were corrected, and the profile was marked
`DEGRADED` when perf was blocked. The run still failed the strict reference
evidence tmpfs preflight because materialized `target/test-evidence/0.67`
directories from the preceding attempt were present. It is retained to show
why the cleanup/preparation hardening was added before the canonical run.

