# Stack pathname candidate for the repeated fresh pidfd check

Test one hypothesis: replace only the temporary heap-formatted pathname in
`diagnostic_kernel_origin::alive_read` with bounded stack ownership of exactly
the same `/proc/thread-self/fdinfo/<signed i32>` bytes. This check is reached
repeatedly below ReservedLive. Existing attribution identifies repeated kernel
cost, but does not measure this allocation's exclusive time or predict a gain.

First retain a default-runtime baseline with an opt-in allocator unit control.
The control formats nine descriptor edges 256 times each and counts only Rust
allocation requests; it opens no FD, samples no clock, and runs no live fixture.
Then implement one stack helper using the existing signed PidName formatter,
add seeded differential, lifetime, boundary and allocator controls, and wire
only that one call site. Do not mix boot, namespace, sample, authority or parser
changes into this candidate. If the allocation hypothesis fails, retain it and
revert only this candidate; no retry of the original owner is allowed.

Preserve byte-for-byte the surrounding poll/open/read/procfs/PID checks and error
order, all validators, deadlines, original origins and observer instrumentation.
Source canaries must reconstruct the entire old origin and allocation-test files
using exact substitutions, not exclude a function body. Deterministic fresh-read
and refusal tests must keep the original Alive/Document counts and first error.
No caching, retained extra FD, root fixture, cgroup, systemd or manager operation.

Run only selected ordinary unit/source-contract tests and allocation controls,
offline/locked checks, strict lint, formatting and local contracts on Windows
and the GNU 1.94/MSRV 1.88 toolchains as applicable. No ignored/manual test,
original-owner profile, qualification, clock/service/idle-policy change, server,
installation, supervisor or product workload. Keep the first input refusal and
attribution packets sealed and unchanged. Pure allocation success does not open
the full material gate or prove a timing/native throughput improvement.
