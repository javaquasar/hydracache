# FD-free reply correlation prerequisite

The private exchange now separates correlation state from request ownership.
ReplyState stores only unique-name data, the original serial, phase and reply
latch. It validates a borrowed request with the old request_matches function;
the owning DispatchExchange retains the exact Message and delegates to the same
state machine. No Message clone, new writer alias, sender or original-start
interface is introduced. The [plan](../../reply-state-owner-plan-20261018.md)
records this prerequisite separately from later original-owner binding.

## Checks and observations

Both retained baseline images pass 22 ordinary dispatch tests. Final Rust 1.94.0
and 1.88.0 images each pass 27 dispatch tests, 15 manager-channel units, five
original-message units and ten worker-context controls. Their separate manual
private-bus matrix passes all nine cases, including lost/delayed acknowledgement,
actual foreign sender, wrong serial and retained writer aliases. The existing
test-only atomic receiver and private fault policy are unchanged. None of these
cases connects to the system bus or a real systemd manager.

Five new ordinary tests cover the borrowed API, rejection without taking request
ownership, no retained writers in ReplyState, unchanged request ownership in
DispatchExchange and a fixed-seed model comparison. Each compiler runs 512
histories: 16,384 generated operations plus 766 prefix operations, totaling
17,150. Both return outcome counts `[360, 1714, 117, 1636, 13265, 58]` for arm,
close, job observation, already attempted, rejected reply and manager error.
The independent model checks phase and reply-open state after every operation;
the owning delegate and standalone state must match it. These are deterministic
coverage counts, not performance or independent fault-recovery samples.

A standalone state can classify a reply even after the request is destroyed and
the test pipes reach EOF. This deliberately proves absence of request custody,
not permission to start or release. Conversely, the owning exchange keeps its
writers through job observation until the exact Message drops. The original
message owner, fixed channel, full validation frame and final custody are not
replaced or reconstructed by this refactor.

Three source guards restore only exact structural edits and test registration,
freeze all 358 older Rust sources, and compare the old arm/lose_reply/observe
block byte for byte. Their mutation controls reject changed predicates,
delegates and registration. All 256 source contracts and 44 Windows controls
pass, with affected all-target checks, strict all-feature Clippy, format,
registry, doc-check and the local non-promotable performance contract. The
missing-type compile proof is retained. Initial compile-only images are not
selected runtime cohorts; the final generator uses the old Header-based reply
flag construction before either selected test invocation.

## Identity and remaining work

`manifest.json` binds the committed six-file code slice, compiler images,
before/after execution hashes, raw logs, unchanged package inputs and qualification
manifest. The manifest itself is retained by Git. No ELF is committed.
`run.ps1` refuses to overwrite a receipt. `capture.ps1` produces the closed packet
once. `verify.ps1 -SelfTest` is read-only and checks source/image bindings, the
unchanged classifier block, source census, model totals, matrix results and
thirteen parser negative controls. `-SelfTestOnly` can check raw parsers before
capturing a packet. A full cargo xtask verify milestone is not claimed.

No throughput, latency or allocation improvement is measured. Next bind this
FD-free state within the original message's complete live validation round,
retaining the real fixed channel and same writer owner without Message cloning.
Production delivery/start, all-writer cessation, durable completion/later cells,
new-boundary SIGKILL/lost-ack, reconnect and production trust/enrollment remain
open. The 46 original effect brackets and 300/60/10-second bounds are unchanged.
There is no release installation, supervisor operation or server/product workload.
