# Local PID pathname allocation control

Baseline: `b63a0e81ad808d6f65e8c8d818ee4c090c3718ff`. Test one hypothesis:
format the numeric process-directory key on the stack on every call, instead
of allocating a decimal String and then a CString. Keep every fresh open,
filesystem/object/liveness check, read, error order and original deadline.
Do not cache a PID pathname or kernel value and do not combine other hypotheses.

Before adoption, measure the existing open and a test-only stack prototype on
the same proc root and PID. Use 32 warmup calls per arm and three alternating
1024-iteration allocation pairs per compiler (GNU 1.94 and MSRV 1.88). Record
the actual PID and decimal length: requested byte sizes can depend on these
and the standard library. Accept only repeated removal of Rust allocation
requests/bytes, never a latency threshold adjusted after measurement. Separate
uncounted timing uses 4096 iterations per arm, without a numerical speed claim.

Compare all signed formatting edges plus 16384 seeded i32 values against the
complete decimal representation. Differential open tests compare identity,
descriptor flags, filesystem and missing/non-directory/symlink refusals. A
test-first runtime-binding assertion must refuse before adoption and pass after.
The prototype and adopted formatter must be exact source matches, not two
independently weakened models. Reverse only exact new helper/hooks and tiny
call/method substitutions in historical source guards; retain all old assertions.

Run both compilers' kernel, join, trust and observer regressions and strict lint;
format, docs, registry and local performance contracts. Read-only source/contract
tests may run with four test threads after auditing their shared-state behavior;
all cost probes remain serial and outside every own build/test/lint job.
Use the unchanged fresh-kernel phase probe to assert exact counts before/after.
Do not retry failures into passing cohorts.

Retain raw receipts, image/source/recipe hashes and refusals in a new
`local-runs/pid-path-cost-20261020` packet. Rehash prior sealed inventories without
modifying them. Product code, lock/COMPAT, qualification and existing registry
stay unchanged. This step does not rerun the full functional/native matrix or
SIGKILL/lost-ACK and does not establish server readiness. No server, release
installation, supervisor/clock service change or product workload is authorized.

## Accepted local outcome

Runtime `b9770cc2`, reviewed source
`b5f1b8fce6b3046239087873cb85ff1bfc557f0f`, adopts the formatter byte-exactly
from the measured prototype. Baseline PIDs 367/393 and post-candidate PIDs
382/408 are three-digit keys. All 12 allocation pairs remove two Rust requests
per open, with requested bytes 14 to zero on GNU and 12 to zero on MSRV and no
reallocations. Both baseline runtime bindings refuse as intended and both
post-candidate bindings pass. All 72 phase reports retain the original counts.

All 350 source/contract tests, both compiler lanes' targeted functional checks,
strict lint, formatting, documentation, registry and local performance contracts
pass. No numerical time/CPU/API improvement is accepted. The full local/native
composition and joined SIGKILL/lost-ACK/reconnect gates remain separate and are
not rerun or opened. The [packet](local-runs/pid-path-cost-20261020/README.md)
preserves all invocations and prior seals.

The packet seals 116 hashed members plus inventory, SHA256
`982d9598426830f6b3e44f9a701220b5778ceb3e6b568922d3b2ee405a0e6b05`.
Its verifier refuses 41 summary mutations. Qualification remains byte-exact at
SHA256 `11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
