# Reply correlation without a second request owner

Baseline is `926a3b6f`. The original message owner retains its addressed request,
fixed manager channel, original receiver and complete frame. The old private
DispatchExchange owns another request. Connecting the two by cloning or moving
the FD-bearing message would introduce extra logical writer custody or remove
the request from the existing live checks. First separate reply correlation
state from request ownership, without adding a sender or original-start route.

Introduce one private ReplyState containing only manager/client name data,
original serial, phase and reply-open latch. Preparation borrows and validates
the request with the unchanged request_matches function. Move the old arm,
lose_reply and observe methods unchanged into that state. The owning exchange
retains its exact Message and delegates to that same implementation. Its old
field observations remain available privately through Deref/DerefMut; neither
type gains Clone, serialization, a message/FD getter or a transport callback.
ReplyState itself is not an original-owner capability or custody proof.

Freeze every existing request predicate and the complete old state-transition/
reply-classification method block byte for byte. A narrow static adapter may
restore only the exact declared structural edits and one test registration.
It must not normalize altered predicates, method bodies or delegates. Preserve
all original frame/channel/custody sources, 46 effect brackets, origin and
300/60/10-second deadlines. No durable or public wire format changes.

Before implementation, retain current/MSRV existing dispatch controls and an
expected missing-ReplyState compile proof. Add ordinary tests for borrowed
preparation refusal, absence of Message/FD retention, owning-exchange lifetime
and seeded transition sequences against an independent phase/latch model. Keep
all old tests unchanged. Replay the nine-case private bus matrix once on each
new GNU compiler image, using the existing test-only atomic receiver and private
fault policy; no system bus, original manager or service is contacted.

Run affected all-target checks, strict all-feature Clippy, focused GNU units,
Windows portable controls, source contracts, format, registry, doc-check and
the local non-promotable performance contract. Preserve first failures and
source/image/receipt bindings in a separate packet; never overwrite earlier
packets or waive a failed gate. Commit this prerequisite separately from later
binding of correlation to the original owner. No original send/start, release
installation, supervisor operation, server workload or qualification is allowed.

After this gate, bind the FD-free state inside the original message's complete
validation round. That subsequent change must retain the same fixed channel,
message, kernel receiver, custody, deadlines and sticky refusal; it must not use
this refactor's tests as proof of production delivery or all-writer cessation.
