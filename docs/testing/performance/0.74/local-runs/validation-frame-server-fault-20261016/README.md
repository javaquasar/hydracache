# Isolated Linux fault matrix completed

Both previously built normal-release executables passed the exact full
validation-frame matrix once on the dedicated server. Each compiler covers
forty cut/kind mutations, five independent old-reference refusals and three
controls: healthy order, failed construction retaining its fence, and concurrent
trust mutation. Rust 1.94 reports 417.61 seconds and Rust 1.88 reports 417.39;
these are test execution durations, not product performance measurements.

The preregistration is 51dccae3. The accepted isolated frame implementation is
7b7992701b9d5b69f8d8fca27aa9e016b8c80f2c, tree
daf94165fa9169bd1a28ce6ec634c644c9db84b2. Its nine selected source Git blobs and
23 frozen lower inputs match the earlier working-copy packet. The executables
were built before that commit; observed hashes and content agreement do not
constitute a signed build or custody receipt.

## Exact host scope

Strict known-host SSH used hydracache-ax42 as nonroot hydracache-admin, UID/GID
1000. Both binaries, runner, logs and separate private TMPDIR directories are
under /tmp/hc074-validation-frame-20261016.htNCLN, owned by that user. The root
is mode 0700, binaries/runner 0500 and logs 0600. Both scratch directories are
empty afterward; binaries and logs remain for inspection, about 34.3 MB total.
No release was installed and no test ran through sudo. Sudo inspected only fixed
marker absence and installed binary/receipt hashes. The production worker UID
986 was neither enrolled nor impersonated by these fixtures.

The runner rechecked quiescent marker/context conditions before and around each
lane. The stable supervisor stayed active/running, PID 8839, NRestarts zero,
same start timestamp and boot e81671d6-bf23-4ebf-b8fe-5dad1f45866f. Independent
postflight reproduces its installed binary and receipt hashes and absent active
campaign. Its source remains 543108f1. These sequential observations are not an
exclusive campaign reservation or physical-host attestation. No production lock,
trust, clock, service configuration, qualification or product workload changed.

## Interpretation and next boundary

This closes the full live-fault/reference/concurrent gate for the isolated frame.
The earlier dev expirations, mixed WSL release chains, partial WSL matrix and
failed timesyncd-stop preflight remain unchanged. They are not retried into this
result, and no publication-free budget cohort ran on the server. A different
environment passing these tests does not uniquely diagnose the WSL clock issue.

The Linux test selected only validation_frame_live_drift_at_every_cut_and_reference_refusal,
with an exact full test name and a fixed 900-second external stop per lane.
No actual process-kill crash matrix or new Starting publication was exercised.
Original fake-input byte construction and temporary original-owner mechanics
are not production credentials, authenticated process creation, all-writer
cessation, later-cell execution or permission to begin a product campaign.

The next isolated local slice is the strict Starting byte record, then the
consuming original-storage phase and exclusive write/seal/fsync/ack path with
real SIGKILL and lost acknowledgement. Signed installation and the full
nonproduct lifecycle rehearsal remain later gates.

## Verification

The read-only verifier checks raw transferred log hashes, exact forty pairs and
five references per lane, each final control and test result, wrapper and
independent postflight, source tree/delta and the original selected/frozen Git
blobs. It reads the pinned Git version, not a later working copy extended by the
next work packet. Other JSON captures retain tool-returned text, not byte-exact
process streams. A verifier pass is evidence integrity, not release admission.
