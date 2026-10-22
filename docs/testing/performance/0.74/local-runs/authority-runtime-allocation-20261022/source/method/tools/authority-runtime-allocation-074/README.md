# Actual authority runtime allocations

Local, nonroot, bounded filesystem fixtures only. This is not a live supervisor,
product benchmark, original-start, fault campaign or server-admission tool.
Read the [preregistered method](../../docs/testing/performance/0.74/authority-runtime-allocation-plan-20261022.md)
before executing. Existing historical allocation images/features must stay off.

`counter.rs` forwards System allocation unchanged and counts successful requested
layouts on the current test thread. Reallocation releases its old layout in the
ledger. Signed lifetime net and logical peak are not allocator usable size, RSS,
CPU time, latency or process-wide allocations. Capture deliberately excludes
fixture production, JSON reporting, corpus/identity assertions and RSS sampling.

`run.py --attempt <new-name>` runs inside local Ubuntu WSL. It refuses existing
output/source roots, root identity, unreviewed build hooks and runtime/lockfile
drift. It generates complete source-verified baseline/installed overlays, builds
fresh release libtest images under Rust 1.94/1.88, lints and executes four exact
counter/semantic controls per image. Each attempt keeps its first refusal.

Targets are separate for each compiler and role. The selected Cargo artifact
must report fresh:false; paired executable paths and SHA-256 must differ.
The retained fifth attempt is not an accepted comparison: its shared target
reused the baseline image and was never signalled to measure. Current tooling
copies only verified dependency-cache files from that completed build into new
targets, preserving file modes without links or old-cache writes. Supervisor
package artifacts/fingerprints are excluded and its selected images rebuild.

After `ready.json`, the runner waits at most 600 seconds for `method.json` in
that attempt's output directory. Commit the verified method, require a clean
tracked checkout, then atomically publish `{"source":"<full commit SHA>"}` via
a temporary file and rename. Compiled probe/counter/driver bytes must match that
commit. Do not edit the checkout while the five BC/CB/BC/CB/BC pairs run. Missing
or changed method/context refuses instead of measuring. No measured-cell retry
or added warmup is provided. Do not modify original validation deadlines.

`python -B test_method.py` executes offline negative/source/receipt controls.
For formatting the unattached source template use
`rustfmt --check --config skip_children=true counter.rs probe.rs`: generated
child filenames exist only inside the overlays. Counter tests can also be
compiled independently with `rustc --test counter.rs` to a new owned target.

`review.py <attempt-directory>` requires all twenty fresh measured processes,
all eight ordered cells, matched canonical corpora, raw log hashes, sixteen
controls, zero ignored tests and exact owner-layout reconciliation. It retains
negative/zero effects without a win threshold. Raw rows additionally contain
cold-open/failure/drop counts, requested-layout peaks and maximum publication.

`collect.ps1` packages the specific retained development attempts and successful
cohort for this experiment, never overwriting a packet. Adapt its attempt list
before freezing another experiment; do not collect unrelated target contents.
`seal.py <packet> --seal` first-publishes facts and inventory. Later use
`seal.py <packet>` for read-only verification, including the predecessor packet.

Scope flags and host/config metadata are evidence guards, not permissions to
install a release, contact a server, change clocks/services or start a workload.
