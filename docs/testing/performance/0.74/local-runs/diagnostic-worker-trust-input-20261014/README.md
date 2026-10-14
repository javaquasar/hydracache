# Fixed worker trust input and original authority evidence

Implementation `3123349d9cb7704616292fc65c766d87080d6b2b` adds read-only fixed
configuration and an owned policy/journal consistency join. Baseline is
`a4bb2cb0d34814492ace996d1bda07452e97480d`; preregistration is
`ef1d44cca102c92a731f359cde74a0e16dff3513`. The
[design](../../diagnostic-worker-trust-input-design.md) and
[contract](../../diagnostic-worker-trust-input-contract.toml) preserve closed
production admission. No actual trust file, approved worker policy or epoch is
created or selected here; no private key is read or production policy signed.

## Implemented input and ownership

The no-argument Linux constructor reads only
`/etc/hydracache-perf/worker-trust-074.json`, after preliminary effective UID/GID
zero. Original no-follow root/directories, root-owned single-link non-executable
file, metadata and exact bounded bytes remain retained. The canonical document,
limited to 2048 bytes, selects no verifier: its key must match the reviewed worker
issuer. Explicit policy-body pin and nonzero epoch are required; no defaults,
environment values, selfcheck or signed envelope can fill them.

Verification is config/journal/policy/journal/config, with the same active
authority revision/epoch. WorkerPolicyInput owns the checked policy/envelope and
asserted host context while borrowing both original config owner and authority
journal mutably. It accepts no replacement values during revalidation and exports
no worker policy, trust, descriptor, fixture conversion or start permission.
Failures latch all original runtime inputs, without a durable refusal append.
The existing policy verifier, generic trust API and authority journal are unchanged.

This requires an already enrolled original authority journal; it does not perform
bootstrap. Host context remains asserted, and scalar root ownership is not proof
of an initial physical-host namespace. Positive cases below use private nonroot
models and synthetic signing keys, never the real private key.

## Local checks and retained negative result

Baseline passes eight reviewed-public-trust, fifteen Linux worker-policy and
thirteen fixed-authority checks. Missing worker_issuer_key API produces the
expected compile refusal. First implementation tests compile but five of ten
fail before the join: the authority fixture was incorrectly nested rather than
using its existing exclusive `/tmp/hydracache-authority-*` entry. Correcting only
the fixture passes ten; no production restriction or old fixture reader changes.

Reviewed coverage adds coarse-stamp/mixed-read rejection, FIFO refusal and fixed
path drift protection. Eleven tests pass in each of three final repeats, with
seed `0x07512074`, 256 invalid byte mutations and four independent parallel guards.
Coverage includes strict documents, pin/epoch/key refusal, fragmented/short/error/
interrupted reads, owner/mode/type/size/link/name drift, safe-but-changed metadata,
root/ancestor/file substitution, wrong policy/host/authority and sticky refusal.
The actual public constructor refuses the nonroot local caller before file I/O.

Linux regression passes 8+15+13; 63 doctests include five new extraction/start/
original-borrow/replacement-argument/thread compile failures. Windows passes eight
reviewed-trust and fourteen policy cases; Linux source coverage is not claimed on
Windows. Current/MSRV all-target/all-feature checks pass on both, strict current
Clippy passes on both, and affected xtask check/lint passes on both. Linux MSRV
Clippy remains unavailable and is not claimed. Root suites pass 6/125/13, followed
by the scoped production-label contract recheck. Format, docs, local performance
contract, links/book pass; require-ship refuses with expected native exit 1.

## Integrity and remaining gates

The [manifest](manifest.json) pins source, raw captures, unchanged public key/
authority/Cargo inputs and frozen qualification inputs. The read-only
[verifier](verify_packet.py) also checks that the old worker-policy source differs
only by the new child-module declaration. Run:

```powershell
C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-worker-trust-input-20261014/verify_packet.py
```

No actual server, root configuration, authority bootstrap, production policy,
private key, workflow, product candidate or expensive workload is touched. Full
release verification and manual crash proofs are not claimed for this read-only
slice. Actual approved policy/body pin/current epoch, enrolled kernel host/namespace
and controller provenance/replay/fence joins remain P2; original start/root
collector, watchdog/writer cessation, completion and later cells remain P1/P3.
