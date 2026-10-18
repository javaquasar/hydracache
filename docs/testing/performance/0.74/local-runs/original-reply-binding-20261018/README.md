# Original owner reply binding evidence

This is a local, preparation-only ownership change. OriginalManagerMessage
retains the same addressed Message, fixed manager channel, original receiver,
writers and custody. Its payload now also holds an opaque OriginalReplyState.
The wrapper borrows the Message and cannot arm, observe a reply, export a
Message/FD, send, start, complete or release the fence.

## Validation boundary

Preparation creates the state through the unchanged ReplyState constructor.
Both payload checks require the same serial and names, Prepared phase and
closed reply latch, and repeat the unchanged request predicates, including
live pipe/descriptor flags. The existing canonical body and pinned descriptor
identity checks remain. Both channel/context checks still authenticate the
same fixed connection and original receiver; a sealed additional comparison
requires the claims to equal that channel's retained names and refuses a
poisoned channel.

The wrapper stays beside the Message inside the existing single complete round.
Nothing moves it to another exchange or reconstructs it on failure. Missing or
inconsistent state poisons the same original owner. No full validation walk,
authority cache, origin refresh, durable format or deadline is added. Exact
source adapters restore all 359 prior Rust sources and the old test registry.
The 46 effect brackets and original 300/60/10-second bounds remain unchanged.

## Retained development failures

The expected missing-wrapper compile proof is in `missing-wrapper-red.log`.
Initial GNU images pass ordinary controls and the private-bus matrix, but the
first full-frame test has an incorrect harness expectation: the healthy case
passes its frame and then requires a refused owner. The raw assertion is kept
in `initial-full-frame-194.log`; the correction changes only the new test's
expected refusal to `case != "ok"`. Later test images are separate cohorts.

The first generated source adapter contains surplus registry context. Its
focused and in-flight full-suite failures are retained, not accepted as gates.
The corrected adapter removes only the exact newly registered gate, with
mutation controls and full prior-source restoration. No runtime predicate or
old test assertion is relaxed to fix these harness/adapter failures.

## Selected runtime controls

The corrected GNU 1.94 and 1.88 images each pass 30 dispatch, 18 manager,
five original pipe-message and ten worker-context ordinary tests, plus the
existing nine-case negotiated private-bus matrix. Six new ordinary tests cover
FD-free wrapper lifetime, repeated original payload checks, exact serial/names,
phase/latch refusal, missing/foreign correlation and poisoned channel names.
The unchanged seeded classifier model still covers 512 histories and 17,150
operations per compiler; this is deterministic coverage, not crash trials.

Both new nonroot full-frame invocations pass all eleven scenarios without a
retry: one healthy control, four opening correlation faults, four closing
correlation faults and name refusal at each channel boundary. Each closing
scenario performs exactly 34 authentication calls, 34 signature calls and two
channel/context checks. Opening refusals perform no full-frame authentication
and one channel/context check; every failed attempt refuses a subsequent pass
before any channel check. The original origins, admission offsets and journal
bytes remain exact; custody/fence retention survives payload drop.

The selected test-image hashes are GNU 1.94
`0fa1486acbd8612171d8ebab30caff13debc97e86bbb1e20d8f21eff2110d718`
and GNU 1.88
`171607157d6556626b916bdf3da724398042fd37dd9ba2e7feab74a45da66218`.
Before/after hashes match for each ordinary/matrix cohort and full-frame
invocation. All nine inert bundle files also retain their before/after hashes.
Windows portable controls pass 44 tests. The qualification manifest remains
byte-identical at
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

These are ownership and admission-boundary checks, not throughput, latency or
allocation measurements. The original production fixed-channel constructor is
not replaced by the test probe and has no new positive enrollment claim.

Implementation source is `dc1ca02651eec2865318c903edfaeee38ac2b576`, immediately
after plan commit `41de7e4a`. All 259 source contracts pass on the final adapter;
the full suite uses four independent test threads, without changing any
assertion. The new exact restoration and all old mutation canaries pass.
Affected checks, strict all-feature Clippy, direct leaf formatting, the 184-gate
registry discovery, documentation and local non-promotable performance contract
also pass. No full workspace milestone or ship qualification is claimed.

## Reproduction and scope

The [preregistered plan](../../original-reply-binding-plan-20261018.md) precedes
implementation. `run.ps1` records GNU current/MSRV checks, strict Clippy, units
and the unchanged nine-case private-bus matrix. `run-frame.ps1` selects the new
nonroot, test-channel frame gate using retained signed bundle data as inert
fixture input; the bundle executable is never started. Each script refuses to
overwrite its receipts. `capture.ps1` binds code sources, test images and inputs
to a committed source; `verify.ps1 -SelfTest` validates the closed packet and
exercises receipt-parser refusals.
Large failed source-assertion logs are retained losslessly as gzip, with both
compressed inventory hashes and the original raw SHA-256 in the manifest.
Test images and the inert bundle executable are not copied into Git.

Original production delivery/start and executable/streams/process/cgroup
authentication remain separate gates. ReplyState data and healthy fixture
frames cannot prove PID 1 authority or all-writer cessation. Completion/later
cells, new-boundary SIGKILL/lost-ack, reconnect, actual enrollment and installation
remain closed. No dedicated-server contact, installation, supervisor operation,
product workload, clock change or qualification run is part of this packet.
