# Original collector heartbeat preflight evidence

Baseline is b9fb32e1da407d16b7073fbc94820667f3bfdfab. The
[preregistration](../../diagnostic-collector-progress-plan-20261016.md) is
95f8b78b9e2b11bf39754fca9e5f35d1135d96b1. Final implementation source is
0ec52ddc7aa09b24ceda776f5ec4ca92cdd09a1a, tree 21c8ecb36f7948309c3aae6cf5af6a16bde02a83.
See the [design](../../diagnostic-collector-progress-design.md) and
[contract](../../diagnostic-collector-progress-contract.toml).

## Confirmed boundary

One consuming preflight retains the original collector reservation and one new
kernel packet, without changing Reserved or writing progress/Starting. A private
same-origin object check excludes other receivers even on the same host. Original
worker/authority/context checks bracket authentication with the original configured
controller policy. Actual sender credentials and kernel samples are used, not
caller peer/time assertions.

The signed request must be Heartbeat for the original controller, identity,
source, preset and revision 1, with a different request id and nonce. The packet
must have been received after the owner's retained observation. Its original
receive BOOTTIME anchors the exclusive ten-second freshness limit. Neither that
point nor the original 60-second admission hold/windows are renewed. Success and
failure retain the original fence; failure cross-refuses all retained owners and
already-refused observations return before further I/O. No result can accept a
second packet, clone/decode/extract a permission or start/publish anything.

This authenticates one bounded received message, not uninterrupted controller
presence, a committed heartbeat or revival of an expired execution lease. Receive
time is sampled at dequeue, not enqueue or signature generation. A previously
signed queued message is not proof that the controller observed durable Reserved;
future execution authority needs causal challenge and replay/commit semantics.

## Actual checks

The current compiler's default-parallel original-input repeat passes 95 tests,
with two existing manual matrices ignored. The MSRV focused group passes six:
three kernel checks, one seeded binding check, the API probe and the four-case
full composition. Those cases cover successful construction and later owned-file
refusal, an authenticated different controller run, configured-source drift and
an already-refused reservation. They check unchanged files before deliberate
faults, original fence lifetime and fresh-context refusal. They use private
nonroot roots and synthetic signing/issuer material, not production enrollment.

Both Rust 1.94 and 1.88 separately pass 29 kernel tests (one existing manual
case ignored), 30 signed-projection regressions and 21 compile-fail examples.
The new kernel cases use real messages, original receiver identity, signatures,
receive order and sticky refusal. A bounded real wait proves packet expiration
without receive-time renewal. Exact boundary arithmetic and twelve seeded
binding substitutions (0x0740c09c) are separate pure tests, not live-clock claims.
The negative API examples fail on move/trait/method/argument constraints rather
than unresolved/private imports.

Eight Linux/Windows current/MSRV check and strict-Clippy lanes pass for supervisor
and xtask with all targets/features. All 174 static contracts pass. Explicit
registration exclusions compare every prior guard body unchanged; the old
controller/Reserved crash sources, storage, lease/kernel parents, COMPAT, registry,
Cargo.lock and qualification manifest stay unchanged. Automatic registry discovery
still reports 176 registered gates. Format, documentation and the local
non-promotable performance contract pass. Ship admission remains expected-red.

The full MSRV original-input group and cargo xtask verify were not rerun in this
slice. Windows lanes are build/static checks, not Linux runtime proof. No new
write boundary exists, so no new SIGKILL matrix was run; previous process-loss
packets remain historical, not rewritten or relabeled as current execution proof.

## Negative results retained

The baseline-mistarget transcript selects zero tests because of an incomplete
exact name; it is not baseline evidence. The actual baseline runs the existing
Reserved positive and passes one test. The API probe then fails with E0599 before
implementation and passes afterward.

The first five-test composition group has four passes and one Kernel(Invalid)
failure when the test demands another successful full revalidation under the
same fixed freshness budget. That capture has no attributed site and is not an
explanation of the older historical refusal. No deadline was changed. The final
ownership test checks successful construction followed by an explicit storage
fault; a separate real-expiry test checks the attributed progress-age refusal.

The first full default-parallel run has 93 passes, two Authentication(Time)
failures and two ignored cases. Failures occur in the old Reserved positive and
the new composition; these are signed-window refusals, not generic Kernel(Invalid).
Whole-workspace formatting and read-only local gates overlapped that attempt.
An unchanged repeat of the identical command and ordinary test parallelism,
without those tasks dispatched alongside it, passes all 95 active tests.
This does not prove overlap caused the failures or establish shared-load
robustness. The first red remains part of the packet; no window, hold, freshness
limit or default test parallelism was widened or waived.

Initial strict Clippy rejects a needless reference and test-module placement;
both diagnostics are retained, with the final eight clean lanes separate.
A mistyped registry command is retained as a CLI selection error; the actual
gated-test-check succeeds. None of these development captures are final greens.

## Verification and remaining work

Run powershell -NoProfile -File ./verify.ps1 from this packet or another directory.
It checks the source tree and thirteen source blobs, twenty-three unchanged
inputs, exact artifact inventory and SHA256/byte lengths, command outcomes,
negative/positive counts and closed scope. There are 34 command transcripts and
36 hashed artifacts, plus manifest.json. Transcripts normalize line endings to LF;
an empty output is represented by a newline.

This is unsigned consistency evidence, not binary identity, a clean-build
attestation or trusted execution receipt. Starting needs a separate consuming
authority handoff, signed state, durable replay/uncertain-outcome handling and
SIGKILL/lost-ack tests. Root output creation, authenticated original executable/
streams/process/cgroup start, autonomous watchdog, all-writer cessation, real
enrollment/installation and host rehearsal remain open. No product, server,
workflow, qualification or performance claim is admitted. No push was performed.
