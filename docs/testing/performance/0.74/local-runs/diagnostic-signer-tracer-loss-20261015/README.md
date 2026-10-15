# Actual original tracer loss evidence

Source: `5882caae03acc5b4a5b336d3f83a6bb2982b281d`.
Baseline: `ad5c4c8681eabde410d4195cdadb9bad33afa323`.
Preregistration: `8af96999b72816fb9ed9fa3fce2defafbea242cf`.
Pre-implementation corrections and separate control contracts:

- `f1abc0f2206ea49c0930890f0dd1985fba20ea04`: exact nested root;
- `b797aa4ddd7a604d6462f952a57a6ac653df8dcc`: private observer parent and actual container mode;
- `a14433d0ed39ab37e7e91fe282f9b3b51726285a`: failed exact natural-exit prediction;
- `d535639b752e3b376c300639e7c4c072b88a6f61`: bounded setup progress;
- `efcfc0933ce9a9f7a79c18c3118cd7046b38a76d`: post-exec FD-lifetime oracle isolation.

Local nonroot WSL Ubuntu UID/GID1000, Linux x86_64 kernel6.6.87.2, current 1.94
and parent MSRV1.88. The unchanged fixed GNU helper is compiled with Rust1.94,
O3/panic-abort. Windows tests portable behavior and fixture exclusion. No
dedicated-server, GitHub workflow, key/token or product workload operation occurred.

## Confirmed results

The successful path sends SIGKILL only through the original tracer pidfd. An
independent isolated subreaper holds the child pidfd, verifies its original
parent/tracer-thread binding, observes CLD_KILLED/SIGKILL and consumes that exact
terminal record. Repeated waitid must return ECHILD. Initial/rooted/original-exec/
installed/READY/delivered boundaries all pass; dummy sends remain 0 until delivered,
where they are 1. Only afterwards may the four original temporary directories
be chmodded and removed. Emergency cleanup cannot satisfy these assertions.

Three eleven-case repeats record 54 original tracer-loss child SIGKILL tuples,
nine bounded natural-exit controls, three healthy completions, six attributed
report/progress faults per repeat, finite oracle/frame mutations, eight seeded
fresh cases and four independent monitors per repeat. The 11 tests include the
fixed role entry. Pipe EOF proves received writer closure after refusal/drop;
that oracle runs after exec in a separate role without further fork. Ordinary
test-process subreaper flags remain unchanged.

The final aggregate log passes all 112 earlier regressions: handoff13,
namespace13, rooted11, root7, runtime10, joined9, first-exec11, seccomp8,
bundle/executable18 and producer12. MSRV11, Windows portable9, current package
checks/strict lint, 145 source contracts, scoped format, docs and the unchanged 169
registered gates pass. Normal libraries contain no fixture symbols. The local
performance contract stays non-promotable green; require-ship stays expected-red.

## Negative observations retained without promotion

The missing observer fails E0425 before implementation. Initial3pass/7fail
captures locate directory admission failure, not EXITKILL failure. The observer
first assumes a direct root, then incorrectly assumes temporary-container 0700.
The diagnostic records the actual nested `.tmpName/root` and mode 0755. The final
observer encloses the unchanged root in its own retained 0700 parent and checks
exact descriptor-relative ancestry. The first coarse negative tests passed under
an unrelated setup error; final faults require their intended refusal reason.

The next run passes9/fails1: without EXITKILL, installed exits74 instead of the
predicted73. That failed prediction is not erased. A separate preregistered finite
control oracle permits only unchanged-helper branches installed73/74, ready74,
delivered0/76, always CLD_EXITED. Positive SIGKILL remains exact. The final raw
tuples show natural-outcome variation; no live-orphan counterexample is claimed.

Concurrent current/MSRV preparation exposes an aggregate five-second report
timer. Pressure captures retain current9pass/1fail and MSRV8pass/2fail. Fixed
credential-checked progress separates setup stages without increasing either
the five-second gap or 30-second outer bound. A later MSRV10pass/1fail EOF oracle
is confounded by unrelated test forks temporarily inheriting CLOEXEC writers.
The dedicated post-exec oracle removes that confound without sleeps or weakened
EOF assertions. A rustfmt syntax failure is also retained. Earlier green logs
before FD isolation are historical development observations, not the final suite.
Failed feasibility cleanup can leave owned temporary forensic files; those failed
runs are not cleanup or production-survival proof.

## Verification and limits

The manifest binds 16 source files and 55 unchanged files, 38 original raw captures,
the unchanged qualification digest and 17 closed production/admission claims.
The verifier also proves that removing the two-line loss-module declaration
restores the baseline handoff body exactly. Old helper, root/exec owner, policy,
production ancillary receiver and reviewed trust inputs remain unchanged.

Run `verify_packet.py` and `test_verify_packet.py`, then
`verify_packet.py --committed <full evidence SHA>`. Packet-local no-text
attributes preserve raw bytes; ten mutation/control tests run on Windows/Linux,
and committed verification compares every packet file with its exact Git object.
Hash checks and test-only mechanics do not enroll a production launcher.

Next close the constructor window before EXITKILL installation as a separate
original-owner hypothesis. Independent runtime/job/issuer custody, real token
producer and authenticated receiver/replay/fence, worker original-start and P2/P3
all-writer cessation, completion/cells 2–4, server installation/rehearsal and release
qualification remain separate. No performance, mapped-memory, complete-sandbox,
multithreaded-signer or production-custody claim is made.
