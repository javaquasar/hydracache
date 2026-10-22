# Owned inline boot prototype experiment

## One hypothesis and boundary

Starting tree: `a6f95e4e1245b52334956ab835bdea4b1f6f2332`. Test one owned storage
prototype, not a runtime integration: at most 36 UTF-8 bytes in local zeroed
storage with a private length, otherwise an owned String. No unsafe conversion,
shared/borrowed authority, decoder narrowing, truncation or cached observations.
The checked UTF-8 getter and larger value may cost more than the saved request.

First retain a reference-only cost preflight on installed Windows MSVC Rust
1.94.0 and 1.88.0. Then add the candidate and compare the same frozen reference
body and nine-case/256-call corpus from the preceding boot baseline. All four
fallible calls keep their order before boot construction. Reuse the exact
synthetic receivers, decoder, old baseline controls and isolated allocator.
Runtime, native API, observer and original windows remain unchanged.

## Controls and fixed cost protocol

Require exact source restoration for both reference and candidate frame/type/
getter changes, both error prefixes for all sixteen masks, decoder identity,
owned values after Origin mutation/drop, generic UTF-8/NUL/escape boundaries,
independent Clone and exact normal/alternate Debug output. Inline at <=36 bytes,
generic String fallback above that boundary. Short/fallback request counts stay
separate; the nine-case baseline rows must reproduce their previous sealed rows.

Reference preflight records point size/alignment, 256 independent point clones,
allocation-free getter control, and seven getter intervals. Getter input is the
same valid 36-byte UUID. Warm up 32768 calls; each interval runs 262144 calls
through black_box references and returns a 9437184 length checksum. Instant is
used only in this explicit pure cost control, never as admission time.

The candidate phase uses equal warmup and seven paired intervals, alternating
reference-first/candidate-first, with the same input/work/iterations/checksum.
The allocator capture is inactive during timing; both getter loops must be
allocation-free. Keep the standalone preflight separate, never subtract it from
paired observations. Record all first samples, with no retry or noise pruning.
Size/alignment, drop layouts and clone requests are not measured physical copy
traffic, RSS, retention or full-path CPU.

## Decision fixed before measurement

If the candidate getter median is greater than the paired reference median on
both compilers, reject this candidate for adoption despite allocation savings.
Otherwise the result remains non-promotable pending broader cost/functional
evidence; this experiment alone cannot admit runtime. Do not tune a threshold,
change a window, or remove the checked getter to pass. Preserve rejected source
only as an isolated experiment; production was never changed, so no production
rollback is required. Any different representation is a new hypothesis.

Run focused tests/check/strict clippy/format, source and offline mutation controls,
docs, registry and local non-promotable contracts. Preserve fifteen preceding
raw seals and 856 members. No WSL, real kernel frame, original-owner profile,
clock/service change, server/install/supervisor action, product workload or
expensive qualification is included. All old Causal refusals and material/
fault/reconnect/server gates stay closed.
