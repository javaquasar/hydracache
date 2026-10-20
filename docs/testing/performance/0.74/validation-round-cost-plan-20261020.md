# Read only WSL time diagnosis and validation round attribution

Baseline is `8ecb405b4068236970bcf72563a72ee3e7ca9c2e`. Read only Windows/WSL
clock, boot, configuration and bounded journal observations are authorized.
Never set time, change configuration, stop/restart a service or WSL, or access
the server. New local synthetic fixtures remain separate from product workloads.

Record WSL running state before entering it, current/prior boot visibility,
time synchronization configuration/status, time-related journal/kernel records
and relevant Windows time-change events. Use one bounded persistent observer
process to sample Linux realtime/boottime/monotonic/raw clocks alongside Windows
UTC and monotonic arrival observations. Keeping that observer alive does not
change VM idle policy. Distinguish directly observed clock reversals/boot changes
from a possible actor; missing/volatile logs cannot attribute an old event.
Do not assume a read-only call to a stopped distribution preserves its old boot.

Attribute the eight existing ValidationFrameStep operations without changing
their order, live predicates, callbacks, error/effect order or 300/60/10 bounds.
Inactive test-only scopes take numeric snapshots of existing kernel/live/auth
counters and aggregate per-step elapsed/count deltas inside each preparation
phase. They never reset counters, retain a validation result, or add a guard
clock. Distinguish inclusive nested spans from wall time outside instrumented
children; never add overlapping live/kernel/auth spans as exclusive CPU.

Unit controls must preserve values/errors/panic identity, nesting and thread
locality, inactive behavior and parent counts. Exact source reconstruction must
recover all preceding bodies and old assertions, including the existing Starting
markers. No new runtime candidate or accepted allocation/latency gain is in scope.

Freeze source/image/recipes before three serial healthy profiles per GNU 1.94
and MSRV 1.88 lane. Compare all parent counts with preceding complete profiles;
per-step kernel nodes/edges must conserve counts and observed child time where
the snapshot stacks are empty. Require all eight completed steps in each full
round. Retain the first original refusal and stop that compiler's cohort; do not
retry or splice partial results into a healthy cohort. A bounded stable clock
observation is not a replacement for live admission or a proof of past stability.

Run focused tests, affected-package check/strict clippy, full historical source
guards, formatting, docs, registry and local performance contracts before cost
probes. Independently attempt each unchanged unprofiled five-case local matrix
once; refusals remain closed gates. Bind initial/final boot/configuration and
unchanged original bundle/qualification and six preceding sealed packets.
No server run, installation, supervisor/clock/service operation, full release
qualification or product workload is permitted. Keep evidence integrity separate
from completeness, time continuity and execution/trust admission.
