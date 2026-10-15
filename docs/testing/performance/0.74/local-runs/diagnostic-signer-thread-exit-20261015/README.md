# Original tracing thread loss after options

The local Linux x86_64 fixture proves original child SIGKILL at six fixed cuts
after actual exit of its creating and tracing thread, while its original TGID
still remains nonterminal. Initial overlaps PDEATHSIG; the later five cuts
follow its existing clear and isolate EXITKILL. This is not a production launcher
or cache performance result.

## Independent observation and control

Before fault release, the isolated observer checks the original pidfd, retained
creator proc object, parent TGID, tracing TID, fixed child roster and four root
directory objects. The keeper waits for creator death, then reports WNOWAIT
without reaping the child. The independent observer requires child pidfd
readiness while the original parent pidfd is still nonterminal, permits normal
group exit only afterward, and consumes that same terminal tuple to ECHILD.
Positive cases never signal the child. Only the delivered cut has one dummy send.

The rooted control removes only EXITKILL. It observes the original child still
nonterminal and detached while the original TGID remains nonterminal, then
explicitly SIGKILLs that validated original child for cleanup. Its terminal
record has `positive=false`; cleanup cannot prove EXITKILL. This is a point
observation, not a general orphan survival or runtime custody claim. Healthy
controls at all cuts complete the same original ACK, close and zero exit.

## Reproduction and retained attempts

Use `cargo test -p hydracache-long-run-supervisor-074 --lib --locked signer_thread_exit -- --nocapture`
on Linux x86_64 with the existing unprivileged namespace capability. Missing
capability or a stalled stage refuses without fallback. Rust 1.94 is the current
parent toolchain and Rust 1.88 is the parent MSRV; the unchanged GNU helper remains
Rust 1.94/O3/panic-abort in both. The platform is local WSL Ubuntu, UID/GID 1000,
kernel 6.6.87.2 x86_64. No key, token, server or product workload was used.

The manifest pins source, baseline, preregistration and the separate progress
amendment. `thread-exit-baseline.log` retains eight old creator tests.
`thread-exit-missing.log` is the expected E0425 test-before-implementation failure.
The first development six-test run passed. `thread-exit-linux-final.log` is a
failed attempt: five passed and the healthy delivered-cut observer stalled
before its first report. Its construction and five advances had been treated
as a single five-second gap. The separately committed progress amendment adds
exact credential-checked ordinals, without changing per-gap/outer/capture bounds
or terminal predictions. The original log and pre-amendment MSRV/check captures
remain historical, not final-source evidence.

`thread-exit-linux-final-2.log` must contain the exact complete suite vector
`[6,6,6,8,7,11,13,13,11,7,10,9,11,8,18,12]`: three new repetitions and all 138
old signer regressions. The repetitions contain 54 positive terminal records,
three distinct no-EXITKILL controls and 18 healthy completions. The seed is
`0x0740d073`, with eight fresh cases and four independent concurrent observers.
The final MSRV capture requires six passes, Windows nine portable tests, and
the final source-contract capture 148. Current check/strict lint, scoped format,
doc/169-gate registry/local non-promotable contract and normal-library symbol
exclusion are checked separately. Require-ship remains expected red.

Run `python verify_packet.py` and `python test_verify_packet.py` here. The
read-only verifier checks raw log hashes, exact suite counts and terminal
records, baseline/source Git objects, unchanged qualification and helper bytes,
the old observer body apart from its two-line module declaration, and false
admission flags. `--committed <evidence-SHA>` also compares every packet byte to
Git objects without line-ending normalization. Nine mutation tests cover log
drift, rehashed wrong deaths/lifetimes/zero suites, promoted cleanup, source and
production-flag forgery, missing captures, duplicate JSON and committed bytes.

The first Windows and Linux mutation runs each had eight passes and one harness
error: their temporary packet was outside ROOT, so the committed-byte probe raised ValueError
from `relative_to(ROOT)` before reaching the expected byte mismatch. The probe
now supplies its temporary root only inside that mocked committed-byte call;
the actual verifier and source/terminal predicates are unchanged. The corrected
nine-test suite passes on Windows and Linux; packet verification passes on both.

## Remaining boundary

Real launcher/runtime/job/issuer custody, credential production and integration,
authenticated original worker start, revocation of every P2/P3 writer and
completion cells 2–4 still require their own implementation and proof. No server
entry, qualification, ship admission, arbitrary-thread support or forced TID
reuse claim follows. The frozen qualification manifest is unchanged.
