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
