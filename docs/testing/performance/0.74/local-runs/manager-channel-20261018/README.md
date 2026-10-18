# Fixed live manager observations before original delivery

Source 3007e7df98e35aed301e81fafb467024110ed4c2 implements the private
read-only producer under the [preregistered plan](../../diagnostic-manager-channel-plan-20261018.md).
Its direct parent 27192139 records the second candidate before its results.
The first stopped cohort is retained separately, not resumed or reclassified.
All tests run locally in WSL Ubuntu/Windows. No server SSH, installation,
supervisor operation, clock change, product workload or qualification occurs.

## The owned context

FixedManagerChannel owns its current-thread Tokio runtime and zbus connection.
Only /run/dbus/system_bus_socket can construct it. Root-owned nonwritable
nonsymlink directory anchors and the root-selected socket are retained and
compared with fresh fixed walks. The actual daemon UID may be nonzero: the
root-selected route, SO_PEERCRED and atomic SO_PEERPIDFD bind the real peer.
There is no pidfd_open interval on a reusable observed peer PID. The distinct
PID 1 pin is opened in the caller's namespace, where PID 1 cannot be replaced
while that caller remains alive.

Retained procfs/nsfs handles require matching caller/peer/PID-1 PID and user
namespaces and preserve the caller's time namespace. Original pidfd liveness
brackets numeric proc lookups. The same EXTERNAL-authenticated connection
provides its client unique name, GUID and the systemd owner/UID/PID responses;
UID 0/PID 1 is interpreted only after those namespace checks. Boot, root-selected
route, actual peer and client/manager context are reobserved. Missing access or
changed/closed context refuses; revalidation failure latches permanently.

That selected kernel view is not the original admitted host/authority context.
The producer has no snapshot constructor, connection/name/FD export, sender or
execution API. The original full frame, signed material and uniquely addressed
original message still need a consuming join. Unix-FD negotiation/delivery,
unit/process/cgroup binding and whole-writer cessation are not proved here.
The old well-known-addressed original message and strict old exchange remain
unchanged and must not be dispatched as if those gates were closed.

Connect, EXTERNAL authentication, Hello and all fixed bus identity queries share
one absolute CLOCK_BOOTTIME two-second observation alarm. Pending work loses to
alarm readiness; the counter is not read/consumed. A direct post-work clock
check also rejects a late ready result. Per-method timeout remains 250 ms.
This is an observation cap, not an original lease or renewed progress. Original
Reserved 300 s, Starting 60 s, progress/challenge 10 s and origins stay exact.
The only dependency change is this Linux tool's direct edge to already locked
Tokio 1.52.3; all versions/checksums/other edges stay unchanged.

## Stopped cohort and protocol attribution

The previous fourteen exchange units pass as baseline; the missing producer
then gives the expected E0425 compile refusal. The development check's unused
readiness-guard warning is retained and corrected before strict checks.

The first selected live cohort passes nine ordinary units and the nonroot
namespace-access refusal, then stops at the 1.94 root-positive Reply refusal.
No MSRV live case runs. Its eleven complete capsules, 333 build-input bindings,
both images/hashes, raw partial cohort and source-suite 230-pass/three-failure
result are under first-attempt. All three static failures are legacy dependency
references that had not restored the single declared Tokio edge. The old
comparison remains exact after the new marked adapters restore that edge only;
their complete contents and surrounding old statements are frozen by canaries.

Separate test-only attribution retains two full changed source files on that
first capsule. It reproduces exit 101 and prints bounded header predicates,
never message/error bodies: METHOD_RETURN, flags=1, all other fields matching.
The [D-Bus specification](https://dbus.freedesktop.org/doc/dbus-specification.html#message-protocol-header-encoding)
makes NO_REPLY_EXPECTED irrelevant on replies. The second candidate admits only
zero or that bit on an already required METHOD_RETURN, while rejecting other
known flags and preserving correlation, body, FD, peer and time checks.
The 5.19 builder helper refuses that reply flag, so units use its safe typed
Header/Builder API to model the real wire form, with no unsafe byte decoder.
The old dispatch/request flags are deliberately not changed by this correction.

The original first images and traced image also remain as hash-named files in
the local WSL cache; archive receipts record their exact hashes. Binaries are
not committed into Git. The attribution is not a passing replacement cohort.

## Fresh selected result

The fresh complete cohort passes on Rust 1.94.0 and MSRV 1.88.0. Each exact
image runs nine ordinary manager units, the explicit nonroot namespace refusal,
one distinct root read-only fixed-bus positive/closed-latch case, fourteen
exchange controls, eight descriptor/notice controls and thirty-two original
storage/fence units. Each compiler separately passes two private compile-fail
doctests, affected all-target check and strict all-feature Clippy.

The nonroot case explicitly refuses missing namespace access; it is not a
failed positive retried with privileges. The root process runs only the exact
read-only test, never the entire suite. It observes the real fixed bus and
then closes only its own client connection to test sticky refusal. No daemon,
PID 1 or service is signalled. A separate ordinary unit kills only its own
unreaped sleep child through its owned pidfd; it is not the active original
writer/process SIGKILL matrix.

Windows passes 44 portable tests. All 233 source contracts, the final four
focused canaries, affected Windows checks/strict Clippy, formatting, registry,
docs and local non-promotable performance contract pass. The final canaries
add explicit manager UID/PID/name mutation coverage after the full source run.
Full workspace milestone verification is not rerun for this isolated slice.
No throughput, allocation, CPU or wall-time improvement is claimed.

## Reproduction and receipt verification

Build --release --lib --locked with explicit +1.94.0/+1.88.0 and separate
CARGO_TARGET_DIR values. discover-images.ps1 uses Cargo's compiler-artifact
records to select each exact test executable; the new dependency edge changes
its filename, so an old hard-coded libtest filename would be unsafe attribution.
run-local.ps1 checks image hashes before/after each complete compiler cohort
and stops at its first unexpected result. check-gnu.sh and check-local.ps1
record the remaining local gates. They are not installation commands.

Fourteen complete source capsules reconstruct the accepted commit on its exact
parent. The verifier requires its entire tree diff to equal those paths.
The 333 bindings inventory top-level package Rust sources plus root/package
Cargo manifests and Cargo.lock. Other tracked assets inherit the unchanged
base; registry sources and host/compiler environment are not inventoried.
Compiler identities, lockfile and tested-image hashes are retained separately.
This is source/image attribution, not a bit-reproducible build claim.

Run `pwsh -File verify.ps1 -SelfTest` here for closed-inventory, source/git,
build-input, image, selected-row and eleven negative receipt controls. It reads
files/Git only: no root operation, process test or host connection is replayed.
The first capsule's 333 inputs and stopped/attribution outcomes are also checked.
Frozen qualification SHA256 remains
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.
Original join/start, admitted host binding, all-writer cessation, installation
and product admission remain false. Next compose the live channel with original
material/message under the complete frame, then prove activation and cleanup.
