# Consuming join tests on the isolated Linux host

The unchanged GNU 1.94 and MSRV 1.88 images from source
f07dcca31e2e69796644d70fef695404b22d1519 pass all preregistered selections on
the dedicated Linux host. Each compiler runs two rounds of 17 consuming tests,
26 preceding trust tests, five preceding placement models and 38 held-build
integration tests: 16 selections and 344 test executions in total. No failed
selection was retried. Preregistration and runner are commit f13ec7b8.

The 17-test selection combines ten orchestration models, six actual
kernel/authority/sealed-FD bridge tests and one assertion-tracing wrapper.
Each wrapper invocation preserves eight executions of the unchanged preceding
mismatch assertion, with first=None in every sample. The complete privileged
FixedPlacement holder is not constructed by these tests.

## Host and custody observations

Strict known-host SSH used hydracache-ax42 as hydracache-admin, UID/GID 1000.
The host reports Linux 6.8.0-138-generic, x86_64 and glibc 2.39. No test executes
through sudo. All tests run serially, nice 10, one test thread and unrestricted
CPUs, from /tmp/hc074-consuming-server-20261020.j59XaL with separate TMPDIRs.
The four existing images and runner match their transfer hashes before and
after execution; the runner records library dependencies. These observed hashes
are not signed build receipts or production authority.

Thirty-four sequential preflight checks and an independent postflight find the
same boot e81671d6-bf23-4ebf-b8fe-5dad1f45866f, active/running supervisor PID 8839,
zero restarts, unchanged installed binary/receipt hashes and absent campaign,
diagnostic and lifecycle contexts. No installation, service/clock configuration,
original send, enrollment or product workload occurs. The checks do not provide
an exclusive campaign reservation or continuous attestation.

The isolated root is mode 0700; binaries/runner are 0500 and test logs 0600.
The outer SSH shell initially creates wrapper.log and wrapper.exit as 0664
inside that private root. Independent postflight retains this observation;
wrapper-permissions.log records their subsequent restriction to 0600 with
unchanged hashes. Scratch directories are empty afterward. Binaries and logs
remain in the isolated server directory; no installed path was changed.

## Evidence and interpretation

Individual selection logs, exit files, wrapper.log and wrapper.exit are raw SCP
copies checked against remote-hashes.log. Transfer, independent postflight and
permissions transcripts are PowerShell-captured text. checks.ps1 records local
source guards, predecessor contracts, strict lint, registry, docs, local
performance contract and formatting. Silent formatting succeeds before the
script writes its complete check receipt. inventory.json binds every packet
member except itself; verify.ps1 checks the closed member set, scope, images,
selected counts, assertion traces and negative summary metadata controls.

The preceding 201-member WSL packet remains byte-exact, including its failed
unbound, timesyncd-stop, held-session and CPU-affinity cohorts. Its integrity
verifier still reports local_join_gate_passed=false. A passing environment
comparison does not establish a sole WSL clock actor or repair that local gate.
Original live predicates, clock source, 300/60/10 origins, deadlines, assertions,
COMPAT, registry and qualification manifest are unchanged.

This closes only the isolated unrestricted-CPU Linux test comparison. Test
execution times in libtest logs are not product throughput or holder CPU/hash
cost measurements. Next construct the actual privileged consuming holder with
the same original frame and signed placement/role provenance, measure its
four-frame/full-file cost and verify refusal/retained custody. Joined real
SIGKILL/lost-ACK, original delivery, all-writer cessation, lifecycle, reconnect
and production trust/server admission remain independently gated.

Verify from the repository with:

```powershell
./docs/testing/performance/0.74/local-runs/held-consuming-server-20261020/verify.ps1 -Source f07dcca31e2e69796644d70fef695404b22d1519
```
