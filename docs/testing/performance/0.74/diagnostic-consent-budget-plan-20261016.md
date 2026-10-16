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
