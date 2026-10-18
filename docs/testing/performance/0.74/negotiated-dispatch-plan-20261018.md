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
