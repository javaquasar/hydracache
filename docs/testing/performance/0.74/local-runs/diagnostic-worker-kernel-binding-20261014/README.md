# Original worker policy and kernel context evidence

Implementation `37c22397402e71cbbf1ff0928c16ab2b4180bcad` adds the
read-only worker/kernel join against baseline
`aa7822798b57a9aa211f561d432db803edcefee1`. Preregistration
`7c22a02b` also records the [future library boundaries](../../test-run-control-library-boundaries.md)
without extracting crates. The [design](../../diagnostic-worker-kernel-binding-design.md)
and [contract](../../diagnostic-worker-kernel-binding-contract.toml) distinguish
current context observations from production enrollment and execution authority.

## Original inputs and bounded admission

verify_kernel_policy accepts original signed bytes, kernel packet and authority,
not a caller host assertion. Only the independently configured pinned signature
and active original journal accept the candidate context. The existing fixed
Probe retains machine/boot documents, typed user/mount namespaces and reading
thread; the packet retains receiver context, actual sender credentials/pidfd,
PID/time namespaces and kernel clock. The new guard keeps that Probe, owns
WorkerPolicyInput and borrows the packet mutably. Revalidation takes no arguments
and brackets context with the original policy/journal and packet. Errors latch
policy/config/authority/packet; restoration does not clear refusal.

Its exclusive 60-second packet hold is admission freshness, not a long-run lease.
Dropping a healthy read guard does not consume a nonce; observing the same packet
again is not replay/start permission. There is no policy/context/descriptor
extractor, fixture conversion, start, completion or durable refusal append.

Positive cases combine synthetic signed policy and owned nonroot config/journal
with real local Linux observations. Account-file hashes in those test policies
are model inputs, not real worker account enrollment. The actual private worker
issuer is never read or used to sign a policy, and actual root configuration is
never created or positively enrolled. Source opener and physical initial-host
trust remain separate from equality of current namespace objects.

## Checks and retained development failures

The prechange baseline covers trust/context/kernel paths; its broad kernel name
filter also exercises unchanged nested bindings while manual proofs stay ignored.
The missing public type/method produces expected E0432/E0599 and native 101.
Eight new runtime/kernel cases pass in each of three final repeats; five cover
the owned join and three cover typed receiver objects, sender/clock and refusal.
Seed `0x07522074` checks 32 signed wrong-context mutations and four independent
threads. Real current-context roundtrips and healthy drop succeed; signed wrong
machine/boot/namespaces, malformed/oversized/bad-signature inputs, source/policy/
packet/context faults and previous refusal reject and cross-latch all originals.

The new public API test passes; 68 doctests include five new no-start/extraction/
replacement/thread/packet-refresh compile failures. Trust/context/kernel and
fixed-authority regression passes, as do Linux policy/reviewed-trust and Windows
portable policy/reviewed-trust. Root suites pass 6/126/13. Current check/strict
Clippy and MSRV check pass; Linux Rustdoc warnings are denied. Linux MSRV Clippy
is unavailable and not claimed. Formatting, docs, local performance contract,
links/book pass; require-ship keeps expected native exit 1.

Three development failures are retained rather than reported green: the new
Linux target was initially unregistered; an inserted test-only hook preceded an
existing source guard's runtime boundary; and a regression command incorrectly
named fixed authority as an integration target. Registering the actual CI target,
moving only the hook below runtime and using the library filter fix these. No
old assertion, namespace rule, production restriction or threshold is weakened.

## Integrity and next work

The [manifest](manifest.json) pins source, raw logs, frozen qualification and
unchanged authority/protocol/key/product inputs. Run the read-only verifier:

```powershell
C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-worker-kernel-binding-20261014/verify_packet.py
```

No SSH, installation, service change, workflow dispatch, private-key access,
production-policy signing, product workload or numerical evidence is performed.
New SIGKILL proofs and full milestone verification are not claimed for this
read-only step. Real approved policy/body pin/current epoch and explicit journal
enrollment, admitted source opener/host, controller principals/provenance and
original replay/fence admission remain P2. Authenticated collector start,
watchdog/whole-writer cessation and completion/cells two through four remain P3/P1.
