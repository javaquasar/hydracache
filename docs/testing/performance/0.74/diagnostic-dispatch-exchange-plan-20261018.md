# One shot dispatch correlation before original start

Baseline is c688490b1bc922547039653a4f49f62797672f3a. The original pipe
message is deliberately unsent, not an unsafe existing execution route. Before
connecting real send, implement the separate private exchange mechanics: retain
the FD-bearing message, close the retry gate before transport entry, correlate
one reply and retain uncertain outcomes. This is a prerequisite within P3, not
authenticated original start or an alternative to whole-writer revocation.

The request must target the selected unique manager name, not the well-known
systemd name. Check distinct bounded client/manager name claims, exact manager
path/interface/member, reply-expected flags, two distinct original-shaped
blocking CLOEXEC pipe writers and no reply/error fields. Those name claims are
data, not independently authenticated manager or connection credentials. The
future fixed channel must produce their trusted binding before this component
can join original execution. Do not rewrite or dispatch the existing well-known
original message to make this test pass.

Preparation owns the supplied message; failure retains it in an opaque owner.
Neither result exports/clones a message or descriptors. Mark the exchange
Uncertain before any possible transport action. No reply, disconnect, malformed
reply, manager error or even a syntactically valid job reply permits a second
attempt. Require exact original reply serial, claimed unique manager sender,
client destination, reply type/flags/header shape, no returned FDs and an exact
canonical nonzero u32 systemd job path. A job path is data only, never process,
unit, start, cessation or completion authority. Keep error payloads out of logs.

Run test-first missing-module proof after the existing eight-unit baseline.
Cover every request/reply predicate, duplicate/early/delayed responses, missing
acknowledgement, manager error, job-id boundaries and retained pipe-writer EOF.
Use fixed enumeration seed 0x0740d518. Add real SCM_RIGHTS round trips over an
owned Unix socket pair; only zbus-built bytes verified equal after transfer may
enter the test decoder's unsafe from_bytes call. Validate actual credentials,
FD identities/flags and bytes independently of reply classification. The socket
pair is not an authenticated system bus or PID 1. No original fence or active
process cleanup claim follows from these isolated message-ownership tests.

Keep all old preparation, grouped enrollment, custody, raw child, manager,
systemd-spawn and watchdog bodies frozen except exact module registrations.
No new relative or absolute admission clock is introduced; Reserved 300 s,
Starting 60 s, progress/challenge 10 s and original origins stay unchanged.
Run both current GNU/MSRV fast tests, portable Windows controls, affected
all-target checks and strict all-feature Clippy, format, source-freeze/mutation
contracts, docs/registry and local non-promotable performance contracts. Retain
source/input identities, red/green logs and a read-only verifier, then update
the article and readiness checklist. Commit only a green isolated slice.

No system bus connection, StartTransientUnit transmission, service mutation,
release installation, product load, server workload or qualification runs are
part of this package. Still required: trusted fixed manager-channel ownership,
original unique-owner message construction inside full live validation,
authenticated original delivery/start, autonomous active collector/watchdog,
whole-writer enrollment/cessation and durable completion. Qualification inputs
and product admission remain unchanged and closed.
