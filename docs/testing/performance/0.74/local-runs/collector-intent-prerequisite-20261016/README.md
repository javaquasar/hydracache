# Collector intent prerequisite baseline and environment refusal

The [preregistered next slice](../../diagnostic-collector-intent-plan-20261016.md)
starts from `c34add1ba481b889c5dc46b571501d77d7b4aa5a`; the plan-only commit
is `ae45a09d3728212850a78e4f6389fed5da0de65c`. No collector-intent runtime
implementation is present in this packet.

The first overly broad Cargo baseline was deliberately interrupted with SIGINT
to its exact owned Cargo process. It had selected unrelated integration targets
for linking. Its exit 1 is an operator interruption, not a failing test. The
subsequent exact diagnostic_intent_projection integration target passes all
16 existing tests on Rust 1.94 using locked offline dependencies.

The missing-API probe is retained as a patch, not in working source. Its execution
failed before Rust started: WSL reported getpwuid and locale I/O errors while
creating a process. This is not an observed E0433 test-first compiler failure
and cannot count as a successful test of the proposed API. Rerun the probe after
environment recovery before implementation.

Windows observed only 28,536,832 free bytes on C: at the first space inspection,
then 23,490,560. Disk exhaustion accompanies the WSL failure; the underlying
Linux filesystem health has not been established. No WSL restart, mount repair
or server operation was attempted. A scoped PowerShell incremental-cache cleanup
was rejected by the execution policy before launch; no deletion is claimed.
The cache inventory identifies reproducible Windows target/debug/incremental
entries in this worktree, not source, evidence, qualification artifacts or VHDs.

Original-start, all-writer cessation, lifecycle progression, installed production
trust and server admission remain open. This is an incomplete local prerequisite,
with no performance result or product/qualification execution. Captures are
UTF-8/LF tool output, not signed execution attestation. Historical evidence and
qualification inputs are unchanged. The manifest hashes these retained files.
