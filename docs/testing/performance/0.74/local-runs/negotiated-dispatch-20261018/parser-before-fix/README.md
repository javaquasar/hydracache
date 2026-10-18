# Negotiated private bus descriptor delivery

This component packet tests D-Bus negotiation, real descriptor transport and
one-shot reply correlation on an owned local bus. It does not send the retained
original message, construct a fixed manager owner or contact systemd. The
[plan](../../negotiated-dispatch-plan-20261018.md) preregisters the cases and
retains each stopped cohort separately. Source and binary identities are in
`manifest.json`; no executable is checked into Git.

## Test scope and conclusions

Each GNU compiler runs nine cases in one selected matrix: bare zbus receiver,
healthy delivery, 17-byte fragments, withheld then delayed acknowledgement,
foreign actual sender, wrong reply serial, receiver disconnect, correlated
manager error and local FD-capability refusal. The receiver is a same-user
fixture connection, never PID 1 or a production manager. EXTERNAL, Hello and
NEGOTIATE_UNIX_FD/AGREE_UNIX_FD are actual protocol exchanges over that bus.
SO_PEERCRED binds the socket peer to the exact owned daemon PID/UID/GID.

Positive FD checks retain exact pipe device/inode and order, blocking/CLOEXEC
flags, body bytes and original serial. Fragmented writes attach both rights
only to the first successful fragment. Real bus-assigned sender names, not
caller header assertions, enter the unchanged DispatchExchange classifier.
Malformed, foreign, disconnected and delayed outcomes cannot retry. A job path
still conveys no process identity or completion authority.

The stock pinned zbus receive path produces descriptors with FD_CLOEXEC zero.
The bare-reader case verifies that fact and the existing strict writer refusal.
Positive cases use an explicit test-only recvmsg adapter with atomic
MSG_CMSG_CLOEXEC; no later fcntl repair hides an exec-leak window. This is not
proof that a production receiver uses that adapter. Three ordinary units cover
zero/two rights, excess and actual MSG_CTRUNC with 253 sent rights, EOF after
all installed aliases close, WouldBlock, disconnect and unexpected credentials.

The private fault bus intentionally allows unsolicited replies in its send and
receive rules so that foreign-sender and wrong-serial faults reach the actual
classifier. That permissive file exists only inside its owned 0700 fixture
directory. It is neither the installed system-bus policy nor proposed enrollment.
No user/system bus discovery, service activation or systemd operation is used.

After the local exchange and both connections close, the received Message still
writes into the original pipes. EOF appears only after that final Message drops.
This is a concrete counterexample to early writer-cessation, completion or fence
release based on reply receipt or sender destruction.

## Retained development results

Baseline: nineteen existing dispatch units pass on each previously built GNU
current/MSRV image. The missing-helper compile proof and initial socket/sender
type integration errors are retained as separate logs.

Three stopped current-toolchain cohorts remain negative evidence, not healed
successes. First, default tempfile directory permissions under umask 0022 expose
0755 and fail before daemon creation; the later fixture explicitly requests
0700 at creation. Second, real negotiated delivery reaches strict writer refusal
on the ordinary zbus receive path. Third, atomic receiver units and the first
four matrix cases pass, but the daemon blocks an unsolicited foreign reply
before the classifier; the later private fault policy makes that input deliverable.
MSRV matrices are not dispatched in any of those stopped cohorts.

The subsequent private-policy cohort passes both matrices and receiver units.
Current Clippy then requests an equivalent is_multiple_of spelling in the new
test receiver. Pre-lint sources, image hashes and passing receipts remain
separate; final images repeat both matrices and units after that one predicate
spelling change. No runtime validation, original admission or deadline changes.

The 253 source contracts pass, followed by the two affected source guards after
the lint-only edit. Windows passes its existing 44 portable units. Affected
all-target checks, strict all-feature Clippy, focused Linux regressions, format,
registry, doc-check and local non-promotable performance contracts are retained.
This is a step gate, not a full cargo xtask verify milestone.

## Reproduction and remaining boundaries

Build the package's release libtest with Rust 1.94.0 and 1.88.0, then run the exact
manual gate registered as `tool.hydracache-long-run-supervisor-074.negotiated-dispatch`.
`matrix.ps1` records before/after image hashes and never overwrites a receipt;
`check.ps1` runs scoped checks. `capture.ps1` retains committed source capsules
and raw logs once. `verify.ps1 -SelfTest` is read-only: it checks the closed file
inventory, Git source binding and unchanged runtime/test bodies, image receipts,
all nine cases, receiver units and negative parser controls. It needs no daemon,
WSL root, SSH or live server. Retained ELF files stay under ignored local target.

No native/RESP throughput or latency improvement is measured here. The original
46 effect brackets, alarm origins and 300/60/10-second windows remain unchanged;
the frozen qualification manifest digest is unchanged. Still required are real
fixed-channel/original-owner delivery, authenticated executable/streams/process/
cgroup binding, active all-writer cessation, durable completion/later cells and
their full new-boundary SIGKILL/lost-ack matrix, integrated reconnect and real
production trust/enrollment. No installation, supervisor action or server/product
workload is part of this packet.
