# Census source pins and local Linux component audit

Independent test-source pin integrity and local GNU/Linux component behavior are
confirmed. **The selected real caller is not admitted for integration**: its
successful reads ignore stderr, while native batch admission rejects nonempty
stderr. Child-local Git tracing reproduces that difference without changing the
caller or repository configuration. Production pin authority also remains closed.

Code commit: `acbbcbf98247f615255d07e35445eab2486a7e90`.
The [evidence index](local-runs/historical-blob-caller-linux-20261024/index.json)
retains exact inputs, source versions, first refusals, raw child outputs and
Linux session receipts. No real loader is switched.

## Existing trust anchors for the selected caller

The selected function is
`crates/xtask/tests/support/census_provider_input.rs::frozen`. It accepts only its
two existing full commit constants, explicitly disables replacements, checks
success, decodes UTF8 and normalizes CRLF. Its original/acceptance/view paths
remain conditional. Preparing two references does not authorize batching or
eagerly moving those reads ahead of current-source checks.

`caller_pins.py` verifies the earlier census packet index at SHA256
`86946490836001a8f887220767b3d3677a050bbf94664370b09fd548ff8a1251` before
using its members. The archived caller must match; the complete existing
material-lock contract must match the sealed successful census intent. The
independent inventory member is separately fixed at
`cfd5e7f1c153f5214dd71da196dfdbac2d1c05806fa14865605e9ce0776bc9c0`.

The BASE body must match the pre-existing contract SHA256
`ede23782d4ff6a5ae052ca542998c3ebd8968af2ad678f67b179af4fdb05d828`.
Because that old contract normalizes checkout line endings, this producer also
requires raw canonical LF bytes before treating its hash as a raw pin. It does
not silently promote a normalized hash to authority for arbitrary raw bytes.
The REVIEWED body must match the earlier inventory's exact blob
`e4111fdbbb90e0c06a271617da8b3781aa25d58e`, 35108 bytes and SHA256
`6b646ab26eff108b751316512dfacff3580cae89dd399366011c8a448fe2ed5f`.

Only after those independent byte checks does the producer derive a raw blob ID;
the native batch cannot refresh either reference. Both files are blobs, so this
caller needs no fallback output pin in the checked success cases. These are
established repository test-fixture trust anchors, not production signatures,
issuer policy, epoch pins or server enrollment.

## Actual caller shadow results

The new `historical_blob_caller_074` target invokes the actual unchanged caller
in child processes. The clean profile matches both native byte results and the
old caller's original/acceptance/scoped-view behavior. Unrelated inputs retain
their old values. The shadow is additional diagnostic work, not a replacement
for any existing fresh observation.

With only child-local `GIT_TRACE=1`, the old caller still accepts both source
views: successful `git show` writes tracing to captured stderr, which it ignores.
The native reader returns `Process` for both cases because its batch requires
empty stderr. The passing characterization target records this mismatch and
`caller_admitted=false`. No global Git setting or environment is changed.

The three new target cases pass on Rust 1.94 and 1.88; one is the child-fixture
entry. MSRV also passes both unchanged role-census reader regression cases.
Current all-target check, all-target/all-feature strict Clippy, focused rustfmt,
39 Python controls, Python compilation and tabnanny pass. A clean shadow match
does not approve ambient config/environment, failure/panic behavior or scheduling
of conditional reads.

## Local Linux results and first refusal

Only the already-running Ubuntu WSL is used; the stopped diagnostic distro is
not started. Read-only preflight records kernel 6.6.87.2 WSL2, installed toolchain
locations, boot and clocks. Native execution runs as UID 1000 in one private
temporary fixture with exact copies of the three component test targets and
their source modules. The minimal fixture manifest is separate from the real
workspace manifest and is not a Linux package-gate claim.

The first seed command, `generate-lockfile --offline`, selects a bitflags identity
outside the original lock. The strict dependency subset gate refuses before
compilation. Its input, source, command output, refusal and cleanup are retained.
That first version did not export the rejected resolved-lock bytes before
cleanup; the corrected runner exports them before validation. No missing first
artifact is reconstructed or represented as captured.

The separate corrected candidate resolves metadata from the supplied old lock
and checks every resolved version/source/checksum against it. All 33 package
identities pass without fetching/installing dependencies or modifying the root
lock. Both existing GNU toolchains, 1.94 and 1.88, compile and pass 9 decoder,
5 fault and 6 transport cases each. Both six-profile/54-result oracle matrices
match exactly, including missing/tree output bindings, replacement policies,
timeouts and owned-child cleanup controls.

The session retains source hashes, build/test outputs, resolved lock, clock/boot
observations and all raw component receipts. Boot identity and copied sources
remain unchanged. Both own temporary roots are removed; no harness process group
remains active. Existing caches, worktrees and services are not cleaned or changed.
MONOTONIC/RAW disagreement remains visible in the WSL observations. These are
Linux byte/process controls, not production clock admission, original-owner
execution, native-host qualification or Linux caller/environment equivalence.
The concrete caller shadow runs in the real Windows checkout, separately.

## Admission decision

| Boundary | Verdict |
| --- | --- |
| Independent integrity of the two existing census test-source pins | Pass |
| Local GNU/WSL component byte/process behavior on current/MSRV | Pass |
| Concrete caller clean-profile shadow | Pass |
| General caller environment/stderr/error compatibility | Fail, tracing counterexample retained |
| Production pin enrollment and original clock authority | Not admitted |
| Real caller integration | Not performed |

Next define a caller-specific environment/error contract or an adapter that
preserves the original successful-stderr and failure behavior, then test its
conditional observation schedule. Do not globally discard diagnostics, change
replacement flags, refresh pins from observed bytes or prefetch conditional reads
to make this counterexample disappear. Reference preparation and full-path cost
also remain unmeasured.

All original live checks, order/frequency, 300/60/10 windows, the old 780-second
package bound and first failures remain. Cargo manifests/lockfile, dependency
policy and qualification manifest are unchanged. No clocks, services, server,
product workload, original-owner run, qualification, installation or push occurs.

The subsequent [environment and error contract candidate](historical-blob-census-contract-result-20261024.md)
preserves successful stderr in an isolated caller adapter and tests the original
conditional read schedule. Its finite Windows profiles do not erase this batch
counterexample or imply Linux caller, arbitrary environment or real-loader admission.
