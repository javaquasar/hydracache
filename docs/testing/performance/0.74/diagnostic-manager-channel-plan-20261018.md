# Fixed manager channel before the original message join

Baseline is 966f80472ec1133250c7a034afb7b306727f21e7. The previous exchange
classifies message/name claims without authenticating a live manager channel.
Add a separate private read-only producer owning its connection, retained fixed
socket ancestry, actual kernel peer credentials and pidfds, client unique name,
server GUID, kernel boot identity and selected manager unique owner. No caller
path, connection, peer, name or authority snapshot may construct it.

Open only /run/dbus/system_bus_socket through root-owned nonwritable nonsymlink
ancestry. Retain O_PATH anchors and compare them with fresh fixed-path walks.
The root-selected socket need not be served by a root-UID daemon: retain the
actual SO_PEERCRED tuple and peer pidfd, without confusing that bus peer with
PID 1. Obtain the systemd unique owner and UID 0/PID 1 from that same authenticated
bus connection, retain a PID 1 pidfd and bracket the lookup with owner, boot,
socket, connection and process-liveness observations. Unknown or changed context
poisons the owner; later observations cannot heal it. Obtain the peer pidfd
atomically with SO_PEERPIDFD, never pidfd_open on an observed reusable peer PID.
Retain procfs/nsfs anchors and require equal PID/user namespaces for the caller,
original bus peer and PID 1; retain the caller's time namespace as well. Bracket
namespace reads with original pidfd liveness checks. An inaccessible namespace
is refusal, not permission to trust a namespace-relative numeric claim.
No serialized snapshot or
injected test backend may mint the production owner.

Use an owned current-thread Tokio runtime and the already locked Tokio version
to cancel connect/authentication/Hello and all read-only queries on one absolute
CLOCK_BOOTTIME operation alarm. The two-second observation cap follows the old
manager helper's cap; it is not a lease or permission to exceed any original
Reserved/Starting/progress/challenge bound. Do not read, rearm or replace an
original alarm. Original-frame binding and uniquely addressed original-message
construction remain the next composition, before any execution can be connected.

The only dependency change is a direct Tokio edge for this Linux tooling package;
all locked package versions, source checksums and every other dependency edge
stay exact. Register a narrow textual exception for that edge and new module/
COMPAT/manual-gate registrations. Freeze all old manager, exchange, original
validation, child, watchdog, qualification and product bodies. No old guard or
time comparison is relaxed to admit the new producer.

Run the existing fourteen exchange units as baseline, then an expected missing
API compile failure. Unit-test path/identity refusal, actual socket credentials,
pidfd death without numeric-PID signalling, bounded pending/expiry, exact reply
classification and sticky refusal with fixed enumeration seed 0x0740c518. Keep
temporary-path/UID policies test-only and incapable of constructing the fixed
producer. Register a separate opt-in local read-only system-bus test:
capture the actual fixed channel, reobserve it, then refuse after connection
closure. It sends only Hello and the three fixed bus identity queries, never a
systemd mutation or unit lookup. Run nonroot first as an explicit missing-procfs-
access refusal, then a distinct local WSL-root positive case for the mandatory
kernel namespace observations; no privileged fault fixture or service operation
is permitted. These are separate declared cases, not a retry of failed positive
data. Retain a failure as failure; do not substitute
a session bus or widen the operation cap.

Run current GNU/MSRV tests, affected checks/strict Clippy, portable Windows
controls, source-freeze/mutation guards, format, registry/docs and the local
non-promotable contract. Retain exact source/build inputs, tested-image hashes,
negative outcomes and an offline verifier. Update design/readiness/article with
the established scope. No server SSH, installation, supervisor operation,
product workload, qualification, original send/start, writer cessation or durable
completion is part of this local package. Product admission remains false.

## First refusal and second candidate preregistration

The first selected cohort stops on Rust 1.94's root-positive Reply refusal,
after nine ordinary units and the explicit nonroot namespace refusal pass.
No MSRV live case follows that failure. Its eleven complete source capsules,
333 input bindings, both image hashes and exact local archived images are
retained under first-attempt. Separate test-only instrumentation reproduces the
refusal: the returned message has METHOD_RETURN, flags=1, and every other
envelope predicate true. This attribution is not a replacement passing cohort.

The [D-Bus specification](https://dbus.freedesktop.org/doc/dbus-specification.html#message-protocol-header-encoding)
defines NO_REPLY_EXPECTED as having no meaning on method returns. Candidate two
therefore permits only flags 0 or that single bit on the new producer's already
required METHOD_RETURN. Add both positive forms and negative other/combined
flags to the reply corpus. Do not change the old dispatch exchange's flags,
requests, identity/namespace checks, original windows or any execution guard.
Its separate interoperability review still precedes original delivery.

The first complete source suite also reports 230 passes/three refusals: three
legacy dependency-reference checks bypass the current restoration chain.
Add marked exact adapters only for Cargo.lock and this tool's Cargo.toml so
they restore the single preregistered Tokio edge before the old comparison.
Freeze every other dependency edge/version/checksum and every other old source
statement; test the adapters themselves against ff4f3b7e. No runtime guard moves.
Run a fresh complete current/MSRV cohort and all source contracts for the second
candidate; retain the first cohort, traced refusal and three static failures.
