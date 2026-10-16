# Original consent publication budget

Before adding durable Starting to source 4fbc4f3b, run three independent local
original-consent chains with no publication beyond Reserved. Use the already
declared correctness build: Rust 1.94 and 1.88, dev profile with only
ed25519-dalek and curve25519-dalek at opt-level three. This is an attribution
fixture, not an installed-profile or product performance result.

After admission, simulate nine publication boundaries with two unchanged
OriginalStartConsent::revalidate calls per boundary and one final check.
Observe CLOCK_BOOTTIME without supplying it to admission. Report elapsed time,
actual age of Reserved and Heartbeat, completed checks, original authentication
and signature counters, and the first refusal. The test must preserve the
two-document inventory and byte identities, demonstrate sticky refusal and
Busy fence ownership, then fresh-context refusal. A refusal ends that sample;
there is no retry, renewed challenge, clock reset or relaxed deadline.

This experiment deliberately excludes create/write/fsync and independent signer
transport delay. Completing it is necessary but insufficient budget evidence for
publication. A failure establishes that this straightforward full-chain bracket
cannot be admitted in that local build. Preserve every sample, including failures.
Do not average away an expiration or infer server budget margin.

If this baseline refuses, preregister a separate immutable proof reuse candidate
before changing runtime code. Keep all live kernel credentials, original sender,
clock, namespace, trust-file, authority epoch, descriptor and inventory checks.
Pin immutable bytes and verifier inputs, test changed bytes/keys against the
functional reference, and measure the same fixture without changing its workload.
Only then return to the durable Starting implementation and its real SIGKILL cuts.

Authenticated output/process/cgroup start, autonomous watchdog and whole-writer
cessation follow durable publication and its crash proof. Signed installation
and bounded nonproduct host rehearsal follow integrated local verification;
this experiment opens none of those routes.

## Installed profile control

The first six dev-profile samples refused: Rust 1.94 completed 0, 1, 1 checks;
Rust 1.88 completed 1, 1, 1. Recorded IPC signature time is only part of total
validation cost, so this does not yet justify signature caching. Retain those
samples before any further work. As a separate build-only control, run the same
unchanged three-sample fixture with cargo test --release on each compiler, with
no per-dependency overrides. The reviewed provisioning workflow already builds
the supervisor with --release. This local test profile is not an installed
binary attestation; it determines whether a runtime optimization is necessary
before attempting publication. Neither a release-profile success nor a debug
failure changes the fixed windows or creates admission.

## Inclusive owner attribution

If release also refuses, use a separate three-sample, publication-free diagnostic
to time the original worker, signed controller, reviewed build and OIDC owners.
Construct fresh original inputs for every sample. Check each owner three times
and record inclusive wall time, current-thread user/system CPU, IPC authentication
and signature counts. These nested spans overlap conceptually: never add their
durations or call the difference a verified exclusive owner. Retain healthy
owners, original trust and descriptors; create no intent, output or process.
Use the declared optimized-crypto dev lane on both compilers. This classification
must precede a runtime proposal; do not infer that caching signatures alone
can close the measured budget gap.
