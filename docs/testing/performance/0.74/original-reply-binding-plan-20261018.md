# Reply correlation held by the original message owner

Baseline is `ba017a7d`. Bind the FD-free correlation state to the existing
OriginalManagerMessage payload without moving or cloning its Message. Retain
the same fixed channel, original receiver, custody and complete validation
frame. This change prepares correlation only; it must not arm, send, classify
a received reply, start a worker, publish completion or release the fence.

The dispatch module will expose one opaque, preparation-only wrapper to its
parent module. It borrows a request to create ReplyState through the unchanged
constructor. Its check requires the original serial, manager/client names,
Prepared phase and closed reply latch and reruns the unchanged request
predicates. No Message, FD, connection, state getter, Clone or transport callback
is exposed. The original payload stores this wrapper next to its exact Message.

Add a sealed names check to ChannelChecks. The fixed implementation compares
claims to its retained manager/client binding and refuses a poisoned channel;
the existing live check still authenticates the connection and receiver on both
sides. Prepare the wrapper in the existing pure-preparation section. Require
the wrapper and names match before and after the unchanged complete frame.
Missing, foreign or advanced correlation must poison the same owner without
reconstruction. Do not add another full validation walk, cache authority, refresh
an origin or alter the 46 effect brackets or 300/60/10-second admission bounds.

Retain current/MSRV baseline tests before implementation and a missing-wrapper
compile-red receipt. Add ordinary tests for original payload retention,
repeated checks, missing/foreign correlation, serial and phase/latch drift,
channel-name drift and sticky refusal. Extend the distinct test-only frame
probe with faults at opening and closing correlation boundaries; it must not
construct a production owner. Preserve every existing test body. Exact source
adapters must restore only declared edits, preserve old predicates and reject
mutated adapters. No broad source normalization is allowed.

Run GNU 1.94 and 1.88 affected checks, strict Clippy, ordinary manager/dispatch/
pipe/worker tests and the existing private-bus matrix. Run Windows controls,
source contracts, formatting, registry, documentation and the local
non-promotable performance contract. A local full-frame fixture may expire
under its original 10-second bound; retain that failure without retrying it
into success or claiming a healthy full-frame gate. Physical-host testing,
production enrollment and delivery remain separately gated. Do not contact the
dedicated server, install a release, stop supervisor or run product workload.

Retain source/image hashes and raw receipts in a new evidence directory. Commit
the plan, implementation and evidence separately. No throughput, allocation or
production-readiness claim follows from this ownership change.
