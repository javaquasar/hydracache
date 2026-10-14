# Original controller and reviewed builder receipt local evidence

This packet retains the read-only binding implemented in
`f63116df863161bfecb64691bacbeada171a869c`, preregistered in
`1d33e2f6` against baseline `5c84fb7f15077ec5841be4e6d8e4a314a710e575`.
The [design](../../diagnostic-controller-build-input-design.md) and
[contract](../../diagnostic-controller-build-input-contract.toml) keep independent
controller provenance, installed executable contents, replay/fence and execution
admission separate from the verified builder statement.

## Checks and retained failures

Baseline has eight controller cases and eight reviewed-trust cases. Final checks
have eight new cases in three repeats, 78 doctests including five new compile-fail
constraints, three Linux API cases and 147 root tests (6 registry, 128 contract,
13 release-evidence). Linux regressions cover controller 8, worker trust 11,
worker/kernel 5, kernel 18 with the existing manual SIGKILL case left ignored,
artifacts 29, reviewed trust 8 and worker policy 15. Windows portable regressions
cover artifacts 15, reviewed trust 8 and worker policy 14.

Current check and strict Clippy pass on Linux/Windows for the affected supervisor;
xtask strict lint and scoped formatting pass. MSRV 1.88 checks pass on both
platforms; Linux MSRV Clippy is unavailable and was not run. Linux Rustdoc with
warnings denied passes. Documentation, gate registry and local performance
contract checks pass; `--require-ship` retains its expected native exit 1.
No full release milestone gate or new manual process-loss proof is claimed.

`missing-api.log` retains E0599 before implementation. Its shell wrapper also
failed to preserve the native status variable across PowerShell/WSL quoting and
returned 2; it is not a green functional check or an asserted native-status
measurement. `development.log` retains test-only u64/usize length conversion
failure (native 101). `development-fixed.log` is the subsequent seven-case/API
pass before adding the eighth prior-refusal case. `linux-regression.log` retains
successful unit regressions followed by an invalid nonexistent builder target
(native 101). `linux-final-gates.log` uses the correct artifacts target, which
includes builder checks. These corrections change neither product semantics nor
production admission rules. Repeat labels lost their shell variable value, but
the raw repeat log contains all three complete eight-case results.

## Evidence interpretation

Positives reuse the public historical builder receipt, synthetic controller and
worker keys, private nonroot config/journal models and actual local Linux messages.
Seed `0x07542074` covers 32 signature mutations, including recomputed hashes in
synthetically signed controller requests; four independent bindings exercise
separate original owners. No actual private key is read, production policy signed
or configuration installed. Runtime refusal is not durable revocation.

Run `python verify_packet.py` from this directory to verify raw captures, source
and unchanged/frozen input hashes. The verifier is read-only; it opens no host,
keys, services or workloads. Raw logs/manifest are retained byte-for-byte in Git.
The manifest documents all closed admission claims and native/wrapper failures;
these test counts are not performance or server-readiness measurements.
