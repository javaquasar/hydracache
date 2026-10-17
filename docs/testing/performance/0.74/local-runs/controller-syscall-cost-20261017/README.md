# Original controller syscall attribution

The unchanged component-control family performs about 606 thousand syscalls
under nonroot summary tracing on each compiler. The largest call counts are
statx, openat, fstatfs and close, totaling 544,762 calls per family. This supports
investigating repeated metadata and object walks before immutable cryptographic
verification. It does not identify one exclusive validator owner or an accepted
optimization.

The [protocol](../../diagnostic-controller-syscall-attribution-plan-20261017.md)
is preregistered in fc3c19b1. Source is b05bb9a2; both binaries are reused
byte-for-byte from the [previous component packet](../controller-component-cost-20261017/README.md).
No runtime, test, registry, observation, deadline or qualification input changes.
The summary covers fixture setup, all separate controls, full opening/closing
OIDC validation and teardown, before any reservation or execution.

## Observed syscall counts and tracing perturbation

| Syscall | Rust 1.94 calls | Rust 1.88 calls |
| --- | --- | --- |
| statx | 145,473 | 145,473 |
| openat | 127,037 | 127,037 |
| fstatfs | 140,808 | 140,808 |
| close | 131,444 | 131,444 |
| pread64 | 27,767 | 27,767 |
| ioctl | 7,350 | 7,350 |
| fsync | 18 | 18 |
| All syscalls | 605,852 | 605,851 |

Each invocation contains three fresh original objects and thirty component rows,
not one ReservedLive call. All four invocations complete with unchanged original
requests and fixture bytes, no refusal, and the original IPC counter expectations.
The packet retains 120 numeric rows and twelve completed original controls.
There is one access syscall error per traced family; the aggregate summary does
not record its arguments or errno. The successful protocol checks establish
that it did not cause a refusal; do not infer a security defect from this count.

| Normal GNU release family | Plain seconds | Traced seconds |
| --- | --- | --- |
| Rust 1.94 | 0.72 | 8.62 |
| Rust 1.88 | 0.70 | 8.64 |

Order is 194 plain, 194 traced, 188 traced, 188 plain. This small fixed-order
cohort checks tracing perturbation, not a product effect. `strace -f -c -w`
reports wall-clock time inside syscalls across threads, not CPU samples.
Its futex entry accounts for about 68 percent of aggregate syscall wall time;
that is not proof that application locks consume 68 percent of CPU. Aggregate
syscall seconds exceed end-to-end family seconds because thread intervals can
overlap. Calls served by vDSO, including some clock reads, need not appear.
Neither summary times nor traced component spans are baseline latency or a
speedup forecast. Count agreement across compilers narrows the next experiment
but does not justify dropping observations or caching live authority/context.

## Provenance and verification

`host.log` records exact binary hashes, tracer version 6.8, unchanged
perf_event_paranoid=4, invocation order, exits and nine host pre/post guards.
The four `run-*.log` files retain every original numeric row. `syscalls-*.log`
retain the argument-free summaries. No process other than the exact libtest
executable and its descendants is traced.

`verify.ps1` checks the exact file inventory, source trees, prior binary
provenance, original row order/counters, summary counts/errors/rounded totals,
host completion and independent postflight. Its parser self-test accepts one
valid summary and refuses eight malformed variants. The qualification manifest
raw SHA-256 remains
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.

All 193 static performance contracts pass. Documentation and the local
non-promotable performance contract pass. Windows workspace formatting refuses
with OS error 206, the command-line length limit; its raw result stays in
`fmt.log`. The unchanged workspace formatting check passes through WSL instead.
No Rust source changes require a new compiler/check/Clippy claim: the existing
compiler and integration results remain attributed to the prior packet/source.
This is not full milestone verification or release admission.

## Isolation and next decision

All server writes are nonroot under
/tmp/hc074-controller-syscall-20261017.b2iAl8, owner 1000:1000 mode 0700.
Independent postflight finds no remaining fixture executable, including deleted
images. Boot, supervisor PID 8839/start/NRestarts 0, installed binary and receipt
hashes remain exact. Campaign/fixture/authority contexts and the diagnostic
installation remain absent. No release install, service/clock change, root
mutation, workload, observer or expensive Actions occurs.

The next architectural experiment is grouped nonexecuting material/message/alarm
preparation, subject to separate explicit agreement because intermediate full
observations would change. It must retain every live predicate and original
deadline, start/send prohibition, and pass healthy/refusal/fault tests before
acceptance. The rejected alarm remains test-only. Independent guardian lifetime,
authenticated original start, all-writer cessation, completion, later cells and
their new SIGKILL/lost-ack matrix remain open; syscall attribution does not close
those execution gates.
