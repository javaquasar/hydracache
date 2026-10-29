# Exact Q compatibility: GNU/Windows and Rust 1.94/1.88

The isolated Q candidate passes its fixed **functional compatibility gate** on
GNU/Linux and Windows/MSVC with Rust 1.94.0 and MSRV 1.88.0. This does not adopt
Q into Root, close the generic stateful resource refusal, qualify CPU/RSS or
allocator retention, or admit a product/server launch. No new optimization
hypothesis is introduced. The previous [actual-caller resource result](chitchat-caller-result-20261029.md)
and [negative general component result](chitchat-stateful-result-20261029.md)
remain separate, unchanged evidence.

## Candidate and method

Restore the full Q fixture from actual-caller packet
`874ba6355b98229cbeab11c5b0742e439cbced847e6954478234e10e92d7ef56`.
Complete runtime/test fixture bytes are identical to that packet. The observer
relative path has the same depth and remains byte-identical. All platforms use
the same normalized isolated Chitchat 0.11.1 / Zstd 0.14.3 fixture graph, offline
lock and original request table
`4a3308a82d014fb58df808a9f3984922537a80c02219610cdf5ee6fa1ec4a1f9`.
Root's Cargo.toml/lock, deny policy, qualification manifest and ownership helpers
remain pinned and unchanged. This is neither Root's older dependency graph nor
a B73/C74 comparison.

Use the original portable System observer, SHA-256
`ab5cf5e12f5dfbb956ebcee59f8af554cf3030544eba07bd2e928ad2d0b109df`,
with all original control bodies intact. Append a facade that explicitly returns
native C **null/unavailable**, rather than manufacturing zero C allocations.
Append one semantic-only export to the original actual-caller test module; it
asserts that the allocation observer is inactive and exports all inputs,
admission decisions and complete wire bytes. CPU/resource observation tests are
listed in the full catalogs but never selected or run by this compatibility gate.
The old allocator golden control remains 2 alloc / 1 realloc / 512 bytes.

Each peer qualifies the actual compiler/host, builds its own release test image,
validates the complete catalog, executes the same 85 selected library cases, then
runs all-target/all-feature `check` and strict Clippy (`-D warnings`). Semantic,
roundtrip, state, migration, rollback/refusal, capacity, allocator and Q controls
are included. Every case name and command/image join is accounted from raw
streams; expected-panic case suffixes are preserved. The same 239 original
semantic cases and all 30 fixed-MTU caller rows match on every peer.

| Platform / compiler | Context | Selected passed | Whole catalog / filtered | Check / strict Clippy |
| --- | --- | --- | --- | --- |
| GNU / 1.94.0 | original | 85 | 132 / 47 | pass / pass |
| Windows MSVC / 1.94.0 | original | 85 | 133 / 48 | pass / pass |
| GNU / 1.88.0 | separate recovered context | 85 | 132 / 47 | pass / pass |
| Windows MSVC / 1.88.0 | separate recovered context | 85 | 133 / 48 | pass / pass |

There are **340 successful selected test executions, 85 unique cases**. Unselected
catalog entries remain explicitly outside this gate. No full package/workspace,
public-consumer or cross-platform native-C resource coverage is claimed.

## First host interruption and recovery

Both original 1.94 peers complete. During the original 1.88 peers, Windows System
Kernel-Power records lid-triggered Modern Standby entry at
**2026-10-29 16:06:15.1110859 UTC** and exit at **16:54:03.2022431 UTC**. Both
original outer Jobs reach their unchanged 1200-second timeout. Their complete
execution receipts retain SHA-256
`04c99c4abf6ac08bb12d417e62dff85a0f67280ca3f66590e9ff86ebe85da67d`.
Neither original overall result is promoted to pass.

The Windows 1.88 build has no completed inner verdict. GNU's completed inner
85-case/check/Clippy output is recovered read-only from ext4 into a separate
30-member diagnosis directory. Its original native-envelope/Windows handoff did
not complete: that inner pass is **not** used as an accepted full peer. System XML,
all original timeout streams and the separate recovery manifest remain retained.
The read-only process snapshot found no matching native context process; no
foreign or fixture process was signaled by the diagnostic recovery.

After confirmed standby exit, a distinct r1 context runs only the two failed
1.88 peers, once each, preserving the complete original 1.94 references. Complete
Q fixture, observer, caller and request-table bytes match across contexts. Source/
target directories differ; checks, compiler identities, selection and bounds do
not. The recovery postflight finds no Kernel-Power standby events between
17:15:19 and 17:19:06 UTC. No power request, setting, service or clock is changed.
This is recovery from a recorded host interruption; it does not overwrite the
first outcomes or graft partial pass sets into a full peer.

## Source/effect review and offline checks

The runner admits only fixed preparation/format/stage/four-peer labels, exclusive
outputs, exact offline manifests and disjoint compiler/platform targets. Root,
source frame, actual registry files/checksums and private Cargo configuration are
checked afresh before/after each command. Environment names and hashed values
are retained; ambient compiler/native overrides are refused. Git HEAD is read
only by the outer coordinator, not inside runtime validation guards. Test code
is restored as data; the byte verifier executes no archived producer or image.

Existing Windows Job/LinuxGroup ownership implementations remain exact. Each
peer retains the 1200-second outer and 1100-second whole-session limits, a single
shared inner deadline, four-GiB physical/native floors and five-second poll/
retirement. No new process-control, network or credential path is introduced.
The diagnostic scripts only read bounded event logs or copy existing owned
context files. Build/cache outputs are excluded from the sealed packet.

Seventeen offline source/fault controls pass: missing/changed case, catalog,
image, retirement, nonzero/error, deadline reset, bound/poll/cleanup drift,
caller input/seed/scope drift, resource promotion and old-inner-pass promotion
are refused. One first offline controls result fails on an import collision:
the inherited helper inserts its own directory into sys.path, so `import prepare`
selected the older module. Explicitly loading the exact compatibility module
fixes that harness defect under a distinct controls-r1 label; the original error
and producers stay intact. No peer is rerun to repair the offline check.

Rustfmt, byte-only verification, doc-check and the unchanged local
`performance-contract-check --release 0.74` pass. The latter remains local and
non-promotable. Product live-check frequency/order and original 300/60/10,
tests 5/10/45, package 780 and workspace 4800 limits are unchanged. No server,
supervisor, installation, product workload or frozen 0.73 artifact is touched.

## Evidence and next boundary

The [compatibility packet](local-runs/chitchat-compat-20261029/index.json) contains
759 logical members / 377 unique objects, index SHA-256
`865f98e6b82240f06b71cf7b316b441766a6653947fa6d8899fa8540f2fb9bfc`.
It retains both original source capsules/frames, exact fixture/observer/table,
all first and accepted command streams, ownership receipts, System XML, recovered
inner data and offline source/fault checks. Original frames are
`65d0fd2b7de4d85fd387844790484dd238b0e1a157035534a402101d9a3d92d8`
and `551354dfe1754b6bbfab7071a747b1c39584903a585a87f83e23e0452c01f63c`.
Compiled images/build caches are excluded. Final document/byte receipts are
retained separately in the [verification packet](local-runs/chitchat-compat-verification-20261029/index.json).
The reproducible current tools live in `tools/chitchat-compat-074/`; captured
historical producer versions remain evidence, not instructions to execute.

This closes functional GNU MSRV/Windows compatibility for the exact isolated
candidate. Next qualify the same source's CPU/end-to-end and RSS/allocator
retention boundaries without reopening rejected hypotheses or hiding the generic
component refusal. Maintained source/backport distribution, public consumers,
actual Root graph/security and full package/workspace gates still follow.
Original-delivery/all-writer retirement, durable lifecycle, reconnect, production
trust/enrollment and admitted host rehearsal remain independent launch blockers.
