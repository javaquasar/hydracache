# Private bus negotiated descriptor delivery

Baseline is `a36ac6e8`. The accepted preparation owner remains unsent. Direct
SCM_RIGHTS tests do not exercise D-Bus authentication, FD negotiation, bus sender
assignment or response routing. Close that component gap before connecting an
original message to a transport capable of execution.

Use a new cfg(test) module under the existing private dispatch tests. Launch an
owned, nonroot `dbus-daemon --session --nofork` with a socket inside one owned
temporary directory. Connect only to that explicit socket with EXTERNAL auth;
never use a session/system bus discovery API, a systemd service, installed trust
configuration or a privileged fixture. Kill and reap only this owned daemon.
The receiver is another same-user test connection, not PID 1 or systemd. Record
the actual bus-assigned unique names and kernel peer credentials. Do not
construct an OriginalManagerJoin from this fixture or call this production
enrollment, original delivery or authenticated start.

Before implementation, retain the existing current/MSRV dispatch test results.
Add the registration and a missing-helper compile proof, then implement the
fixture. Keep all old runtime and test bodies exact after removal of that one
registration. Register the ignored exact test and a narrowly bounded source
restoration adapter; do not weaken existing mutation assertions.

Exercise successful negotiated delivery, forced small post-handshake writes,
lost and delayed acknowledgement, a different real sender, wrong reply serial,
receiver disconnect after delivery, a correlated manager error, and local
transport capability refusal. Capture NEGOTIATE_UNIX_FD and AGREE_UNIX_FD from
the actual handshake. Independently compare pipe device/inode, blocking/CLOEXEC
flags, descriptor order, body bytes and original request serial after receipt.
Force partial writes only after Hello; record that rights accompany exactly one
successful fragment. Use the existing DispatchExchange arm-before-send and reply
classifier, not a reimplementation of its retry/uncertainty state machine.

Use bounded test-only timeouts for infrastructure/absence observations. No
original frame, admission clock, origin, live predicate or 300/60/10 second
deadline changes. A delivered reply is job-path data only. Keep a received
writer alive after exchange destruction and demonstrate that it can still write:
this is a counterexample to early completion/fence release, not revocation proof.
Unknown outcomes cannot retry even if the later reply is valid.

Run the isolated fixture once per GNU current/MSRV compiler, with first failures
retained separately. Run existing dispatch units, manager/message regressions,
affected all-target check and strict all-feature Clippy, portable units, source
contracts, format, gated-test registry, doc-check and local non-promotable
performance contracts. Preserve raw receipts, source/image hashes and a read-only
verifier. No host workload, installation, supervisor action, product benchmark or
qualification is part of this slice. Real fixed-channel/original-owner delivery,
all-writer cessation, completion, reconnect and production trust stay open.

## First local harness refusal

The first GNU current fixture stops before spawning its daemon: the default
temporary-directory creation exposes mode 0755, and the unchanged 0700
assertion rejects it. Preserve that image, source and log; do not run MSRV as
part of the stopped cohort. The pinned tempfile implementation defaults to
0777 for directories, masked by the observed local 0022 umask. A separate
fixture revision explicitly requests 0700 at creation and Linux `/tmp`
placement, retaining the same permission assertion and every transport
case. No product or admission predicate changes, and the stopped cohort is not
reclassified. The initial compile integration also retains missing-helper,
boxed-socket conversion and borrowed sender-type errors as separate receipts.

## Receiver descriptor flags attribution

The separate 0700-creation cohort reaches real negotiation and delivery, then
stops at the unchanged strict writer-shape predicate on the received FD. No
MSRV matrix is run in that stopped cohort. The pinned zbus Unix reader calls
recvmsg with empty receive flags; CLOEXEC is descriptor-local, not inherited
across SCM_RIGHTS. Preserve this second image, source and refusal.

Keep the strict writer-shape check. For a new, separate fixture revision, add
a test-only receiver using atomic MSG_CMSG_CLOEXEC, never post-receipt F_SETFD.
The positive eight cases must still pass every original strict assertion. Add
a ninth bare-zbus receiver counterexample asserting actual descriptor flags
zero and strict Request refusal, with no retry or promotion. Cover all installed
rights cleanup on excess and actual ancillary truncation in ordinary units,
plus no-rights receipt, WouldBlock and disconnect. The owned bus and new receiver
remain fixtures, not the fixed systemd channel or real manager implementation.
This adapter must not be silently described as the pinned library's default
receive behavior or as proof of production receiver CLOEXEC.

## Private policy must deliver classifier fault inputs

The first atomic-receiver cohort passes its three units, the bare-reader
counterexample and healthy, fragmented and delayed-ack cases. It then stops at
foreign-sender: the daemon rejects the unsolicited reply before delivery, while
the test incorrectly waits for that reply. Preserve the explicit daemon rejection,
timeout, sources and image; do not run MSRV as part of this stopped cohort.

For a distinct final fixture revision, explicitly allow unsolicited replies in
both send and receive rules of this owned private bus only. This intentionally
permissive fault harness lets foreign-sender and wrong-serial inputs reach the
unchanged exchange classifier. It is not the system-bus policy, enrollment or a
proposed production change. No production policy or classifier is weakened.
Use the same nine cases, strict descriptor predicates and time bounds; do not
reinterpret the daemon's earlier protective refusal as a successful matrix.

## Selected scoped result

Both final GNU images pass all nine cases and three receiver units. The earlier
passing private-policy images remain separate from a final lint-only change:
replace remainder comparison with equivalent is_multiple_of spelling in the
test receiver, then rebuild and repeat on both compilers. Strict Clippy remains
mandatory; its earlier current-compiler refusal is retained, not waived.
The final matrix and unit invocations each bind an unchanged before/after image
hash. These are component gates only, with no original message or fixed-channel
owner dispatched. All 356 old Rust source bodies remain unchanged after exact
removal of the test registration. Authenticated original-start, production
receiver behavior and all-writer cessation remain distinct next boundaries.
