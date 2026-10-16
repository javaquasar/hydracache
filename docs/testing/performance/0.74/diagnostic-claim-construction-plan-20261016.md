# Campaign claim construction ownership

Baseline is eaebeb8f. The short transaction owner is already corrected; this
separate candidate concerns only acquire_inner_with and recover_active_with
before a HostExecutionClaim exists. A raw File can retain its flock through an
owned pre-exec fork alias on errors or recover_active returning None. Historical
natural Host(Busy) owners remain unobserved, not attributed by this experiment.

## Causal controls before correction

Reuse the exact private owned-fork/pipe/pidfd fixture from the short-fence tests,
without changing its controls or child syscalls. Add a separate private test
module and invoke its existing one-shot post-acquisition checkpoint at both
campaign acquisition sites under cfg(test, linux). No callback or public factory
is added in production. Retain expected-red logs before modifying runtime.

Cover factory and legacy-admission errors, missing/malformed/conflicting markers,
fixed-context refusals, recover_active returning None, and late admission failures
after marker creation/recovery. Preserve the exact outcome and surviving marker
bytes/inode/mode; never delete or repair uncertainty. Probe the lock with a new
independent open description while the old child is ready and still retaining
its alias. Then acquire a new owner and prove the old child's late unlock/close
and failed contenders cannot release it. Success controls cover Created,
Recovered and recover_active ownership: the successful claim must keep the lock
until its existing Drop, not release at constructor return. Use deterministic
enumerated cases, pipe ordering and bounded fixture watchdogs, no workload.

## Conditional private correction

Only after reproducing the defect, add a private pending owner for a successfully
locked File. Install it before fallible admission; explicitly unlock on early
return, including Ok(None). On success transfer that same File exactly once into
the unchanged HostExecutionClaim. Do not reopen, clone, relock, expose the file,
change public types, remove markers, alter admission ordering/predicates, or
change release_after_terminal/Drop. Keep the short owner unchanged. A private
Option<File> with consuming transfer avoids unsafe extraction; no production
callback, retry, sleep, deadline change or unlock of failed contenders is allowed.

Add an exact-source restoration static contract and negative canaries, layering
only that normalizer into the existing short-fence static helpers. Preserve the
prior short-fence assertions, worker-proof contract and old integration tests.
Qualification, compatibility, wire/durable state, authority/epoch/kernel/time,
worker validation and prior evidence manifests stay frozen.

Run the new controls and old host/fixed-admission/preparation tests on Linux
Rust 1.94/1.88, old IPC/lease/original-fence/host integrations, worker-proof and
ordinary original-input regression. Run affected supervisor/xtask all-target,
all-feature check and strict Clippy on Linux/Windows both compilers, portable
Windows integrations, formatting, static/docs/registry/local contract checks,
and expected-red ship admission. Preserve every negative invocation without
retry-to-green or changed old test selection. Record source and evidence in
separate commits, update the article and live readiness documents. No paired
timing screen, all-writer cessation, original-start, server, product workload,
qualification, full release verify or push is authorized by this slice.
