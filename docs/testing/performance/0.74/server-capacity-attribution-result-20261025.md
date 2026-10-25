# Dedicated server capacity and preparation attribution

The dedicated server now supplies an eight-CPU native compilation gate and a
separate first attribution profile. Both all-target/all-feature GNU check and
strict Clippy pass. The original preparation still expires, and the first
ordinary GNU library cohort remains red: 1348 passed, 82 failed, 122 ignored.
More available CPU is not an admission result.

## Frozen inputs and resource boundary

The existing protected default-feature library image has SHA-256
`6640e1b9b31ae7c87137b335be4f2665d729ea7b4324ee0e874bdbedc346c520`,
built from source `ee9692b672be093716822db13a22f5a63960f928`. The following
documentation-only HEAD is `333a547c81a35b0aa9ebea4601fe4a756a79e3f5`.
The [preceding server session](server-kernel-gate-result-20261025.md) fixes the
2558 source inputs, nine read-only bundle inputs and first full holder refusal.
None is replaced for this profile. There is no runtime, dependency or policy
change in this continuation.

Read-only preflight observes eight online logical CPUs, about 60.8 GiB available
memory and 387.9 GiB free disk. The SSH process inherits affinity 0,5,6,7;
CPUs 1–4 are isolated by the existing host configuration. Cgroup ancestors
report no finite CPU quota and the root effective cpuset is 0–7. With the user's
full-resource permission, only new compilation/test children expand their own
affinity to 0–7; `CARGO_BUILD_JOBS=8`. No boot isolation, cgroup setting or
existing process affinity changes. The profile retains the original four-CPU
affinity and finishes before compilation or the ordinary test cohort starts.

## First attribution profile

One separately frozen healthy-case profile enables the existing
`HC074_CONSUMING_HELD_COST=1` observer. It preserves every fresh predicate,
observation order/frequency, original 300/60/10-second origins/windows and the
1200-second outer diagnostic bound. It is not another functional healthy gate;
the previous full matrix's first expiry at 10018691950 ns remains red.

| Phase | Wall seconds | Process CPU seconds | Returned |
| --- | ---: | ---: | --- |
| prepare consent | 0.581 | 0.581 | yes |
| accept consent | 0.303 | 0.303 | yes |
| outputs | 7.113 | 7.030 | yes |
| enrollment frame | 0.361 | 0.361 | yes |
| fixed manager message | 0.343 | 0.337 | yes |
| owned root preparation | 0.640 | 0.639 | no |

The profile exits 101 at the same original preparation boundary. Its first
causal refusal is `expired`: issued BOOTTIME 1639646691859120 ns, observed
1639656826184588 ns, age 10134325468 ns. Enrollment, placement, fixed-slice and
consuming phases are unobserved. No original message is sent.

Outputs contains 46 completed rounds at each of the eight original step slots.
ReservedLive alone records 6011487326 ns of inclusive wall time. The phase
records 153088 Boot and 306176 Namespace observations. Their corresponding
outside-children wall spans are 1411756020 ns and 1244839672 ns; KernelContext's
separate live view is 3956164649 ns inclusive. These views overlap and must not
be summed into disjoint costs. Authentication totals 28639653 ns across 782
calls; its 782 signature calls total 24458290 ns and are nested, not additive.

The process CPU observation is close to phase wall time, making fresh kernel
validation the next cost owner to investigate, rather than crypto replacement
or background disk load. CPU excludes children, wall-minus-CPU is not an I/O
measurement, and observer overhead is not subtracted. This one instrumented
attempt is not a repeated performance result or a proven optimization. The
historical inner `server=false` diagnostic label is retained verbatim; the
outer transcript establishes that this execution took place on the server.

## Native compilation and ordinary controls

Under UID/GID 1000, the private exact Rust 1.94 toolchain runs:

```text
cargo check --release --offline --locked -p hydracache-long-run-supervisor-074 --all-targets --all-features
cargo clippy --release --offline --locked -p hydracache-long-run-supervisor-074 --all-targets --all-features -- -D warnings
```

Check passes in 19.271 seconds. The first Clippy invocation refuses before lint
because the component is absent; that exit 101 remains retained. The exact
1.94 companion component is then downloaded from the official Rust distribution
over HTTPS, checked against its distribution-manifest checksum and extracted
only into private temporary tools. No installer runs and no system/shared
toolchain or Cargo dependency changes. The archive SHA-256 is
`935529b0b9150084fa9cc1734c6598efe86b6e7d76f31ad3f8d4bb79ebf6031a`.
This is HTTPS/manifest integrity provenance, not a separately verified signature.
The first actual lint with that component passes in 19.059 seconds.

The separate origin regression filter passes 35 cases; three manual measurement
tests remain ignored. These cases exercise fresh Boot/Namespace identity,
error/byte mutations and sticky refusals. They overlap the subsequent library
inventory and must not be added to it as unique cases.

One unfiltered ordinary default-feature GNU library cohort then runs with four
test threads, affinity 0–7, UID/GID 1000, no ignored-test flags and the unchanged
780-second outer bound. It exits 101 in 91.645 seconds: 1348 passed, 82 failed,
122 ignored, zero filtered. This is not the complete Cargo package/integration
gate and does not repair the earlier Windows package frontier.

All 82 failing names belong to signer fixtures. Saved first panics contain 55
ENOENT reports, 25 generic owned-root refusals and two concurrent/wrapped
failures. Source inspection identifies the fixture's explicit
`HOME/.cargo/bin/rustup which --toolchain 1.94.0 rustc` discovery; read-only
inspection confirms that path is absent under the selected account. This
explains a missing prerequisite, not every later kernel refusal.

A new private fixture HOME is prepared under the existing temporary upload
directory. Its exact discovery command and empty-environment compiler-version
control pass as UID 1000 and resolve the original pinned Rust compiler SHA-256
`103b60e1b1339968c1d74202ea1d45686037e82c4ea3e0569de24b18a1e6836a`.
The real account HOME is not modified. No signer or library cohort is repeated;
this setup control does not retrospectively pass the 82 cases.

## Preserved host and evidence

All 2558 source inputs match before/after native checks and the ordinary cohort.
Endpoint frames match across profile, check, lint, library and final export:
boot is unchanged; all nine campaign/lifecycle markers are absent; no owned
fixture cgroup remains; PID1 mount-table hash matches; installed binary/receipt
hashes are unchanged. Supervisor remains active/running, PID 8839, NRestarts 0.
The session performs no release installation, existing-service/clock operation,
product workload or qualification. These observations are not all-writer
retirement or exclusive host reservation proofs.

The [content-addressed packet](local-runs/server-capacity-attribution-20261025/index.json)
retains 266 members/133 objects, index SHA-256
`474cae994bbd36005e44adb14f38d17738d654bbe209a50b5adeddb725f59e1a`.
It includes 77 byte-verified final root records, exact coordinator/protocol
sources, the first missing-component refusal, all first GNU failures and offline
cost reconstruction. Eighteen local Python controls pass: ten existing reader
cases and eight new falsification cases covering changed bounds, CPU affinity,
host drift, weakened lint, compiler substitution and promotion of red results.

The [supplemental checks](local-runs/server-capacity-checks-20261025/index.json)
retain staged-byte reconstruction, doc-check and the local non-promotable
performance contract, all passing. Its 14 members/12 objects have index SHA-256
`7568619beac04cbc4a17d61696fb15f82949825c3e31bdd4bd95417585d8f053`.
Neither these checks nor native lint convert the first library failure into a
complete package or release milestone.

## Next implementation boundary

Use the now-verified private HOME in a separately frozen signer-fixture
compatibility control, retaining the ordinary cohort's red result. Attribute
the remaining fresh kernel leaf costs before selecting one optimization;
neither this profile nor extra CPU permits reducing observation frequency,
changing conditional-read order or extending an origin/window.

The host manager remains systemd 255 and outside the reviewed newer-manager
FD-delivery scope. A compatible-manager environment still needs an explicit
operator decision; more CPU does not authorize an upgrade or a weaker path.
Original delivery, all-writer retirement, later lifecycle cells, production
enrollment, the complete package and matched product qualification remain open.
The qualification manifest retains SHA-256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`
and remains non-admitting.
