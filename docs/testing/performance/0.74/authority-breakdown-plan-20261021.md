# Remaining authority request attribution

Baseline is `ad5d8ad07707cbe5f76d9f01b6d05cf41129adb6`, after the accepted
stack expected-name comparison. This packet adds test-only attribution, not
a runtime candidate. Preserve every runtime byte after removing only exact
test hooks, all live checks, original 300/60/10 windows, qualification inputs
and fourteen sealed preceding packets. No Before/After win is claimed here.

Use the existing synthetic checked policy and identical 1/7-generation journal
sequence (initial epoch 7, alternating Operator refusal and advance through
epoch 10 at depth seven). Record payload/name sizes and hashes. Keep the
original owner, entries, pins and context alive. Measure actual full inspect,
one directory revalidation, one inventory, all record revalidations, all marker
revalidations and the final bind/view. Full request and read counts must equal
twice directory plus twice inventory plus records plus markers plus bind.

Separately measure a frozen copy of inventory's scan through parity validation,
the existing expected comparator on a prebuilt observed set, and fd-path
formatting. Scan plus expected must conserve inventory's requests. The prebuilt
set is a pure comparison control, never a fresh observation or admission; test
that removing a marker makes actual inventory/full inspect refuse despite the
old prebuilt expected set still matching. Full inspect must latch refusal.
The path-format control is a subset attribution, not another additive phase.

For GNU 1.94 and MSRV 1.88 use one capture per compiler, 32 warmups and three
rotated 1024-operation samples of each bounded phase at each depth. Capture
successful Rust allocations/reallocations/requested bytes separately from
retained /proc/thread-self/io read/write observations. Keep observers outside
allocator capture. Require per-phase repeatability, cross-compiler equality,
exact additive conservation and the unchanged eight-case full-inspect control:
25/0/1718 at depth one, 101/0/8500 at seven, with read requests 4/28. These are
allocator requests, not CPU, latency, copied bytes, retained memory, all
syscalls, full ActiveAuthority or cache API measurements.

Freeze controls, non-overwrite recipes and an exact source-input adapter before
capture. Preserve old assertions and reject malformed/duplicate hooks while
retaining unrelated drift. Unit-test every phase, stale-control refusal and the
bounded I/O parser. Run both compiler authority suites, ancillary controls,
affected all-target checks and strict all-feature lint; all historical/current
source contracts, repository format/docs/registry/local performance gates must
pass before capture. Retain any authoring or measured refusal; never replace
or retry a failed measured invocation into a healthy cohort.

Review factual JSON structurally, including scope-negative controls, source
input blobs, compiler images and native exits. Seal once, rehash every member
of all fourteen preceding packets, and verify Git preserves raw packet bytes.
Only after attribution choose a separate runtime hypothesis with its own
Before baseline; do not combine directory, observed-set and read-buffer changes.

Local nonroot synthetic controls only. No new server/live matrix, installation,
supervisor, clock/service, product, qualification or push operation. Old native
clock-order refusals and independent original-delivery, writer-cessation,
lifecycle, SIGKILL/lost-ACK, reconnect and trust/enrollment gates remain closed.

## Local outcome

Capture source `20e6fc1217420d02dee91d2ac7ec9bae766a3273` has one invocation
per compiler, with all three samples agreeing. The
[packet](local-runs/authority-breakdown-20261021/README.md) conserves full
inspection exactly: 25 allocations/1718 requested bytes/4 reads at one
generation; 101/8500/28 at seven; reallocations stay zero.

Two inventories contribute 16/68 allocations and 924/3316 requested bytes;
records plus markers contribute 4/28 and 666/5056; two directory checks and
bind contribute 5/5 and 128/128. Inventory equals its scan; prebuilt expected
comparison costs zero. Fd-path formatting is an included subset, measured at
one allocation/28 requested bytes per inventory, not an extra additive phase.
The unchanged eight-case full control and cross-compiler payload hashes agree.

Both lanes pass 199 authority tests, ancillary controls, all-target checks and
strict all-feature lint; all 377 source assertions and repository gates pass.
No recorded check or capture fails or is replaced. Runtime bytes are
unchanged after removing exact test hooks, as are workload/qualification input
blobs. This is attribution only, not a new reduction or CPU/latency result.

The next isolated hypothesis is fd-path String ownership, preserving the same
fresh read_dir and all guards. It needs its own preregistered paired baseline;
no observed-set, read-buffer, cache, deadline or observation-frequency change
is authorized by these request counts. All earlier live refusals remain.

Evidence `c3e9fb5b5b7b1b1f571a2ed150f153cebaf08fb3` seals 51 members plus
inventory SHA256
`c758e549722903dd77d20e20da00347a3a950964ee8bb682ad43a10a6782003d`.
All fourteen prior packets and their members remain intact; eight structural
JSON and twelve scope mutations are rejected, and all 52 Git entries preserve
raw packet bytes. This packet is immutable and does not authorize a new run.
