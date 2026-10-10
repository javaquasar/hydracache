# Original-process credential observation

## Scope and sequence

Baseline: `6c0191e2fb2a2108e5b0ae1502b6fc5ac4e031c0`. This W11 slice
adds a Linux read-only guard over the original retained process, not a new PID
lookup or authenticated worker-account enrollment. Record the complete local
supervisor baseline, missing-API tests, implementation and repeatable results in
that order. No product workload, host installation or qualification is enabled.

## Hypothesis

A bounded `status` document opened relative to the original retained proc
directory can check kernel credentials against an explicitly asserted numeric
policy without accepting a replacement process. Revalidate original generation,
read status, revalidate generation, read status, revalidate generation. Both
selected projections must match each other and the original projection. The
first error permanently refuses this guard; no subsequent observation or refresh
is permitted. This sequence is not an atomic snapshot or continuous observation.

## Numeric policy and parsing

Require a non-root, non-sentinel UID/GID and at most 32 supplementary groups.
The input policy must be ascending and unique. Empty supplementary groups are
allowed; primary group membership is not inferred. Parse status as bytes, so an
unrelated non-UTF8 process name cannot invalidate numeric credentials. Require
exactly one each of Pid, Tgid, Uid, Gid, Groups, CapInh, CapPrm, CapEff, CapBnd,
CapAmb and NoNewPrivs. Reject missing/duplicate/malformed protected fields,
truncation, oversized documents and noncanonical or overflowing numeric tokens.
Use the existing single positional read budget of 65,536 bytes without retries.

Pid and Tgid must identify the original leader. All four real/effective/saved/
filesystem UID and GID values must match the policy, and supplementary groups
must match exactly. Inheritable, permitted, effective and ambient capabilities
must be zero; NoNewPrivs must be one. Retain and compare the bounding-set value,
but do not require it to be zero: it is not the effective capability set and the
existing unit does not prescribe an empty bounding set. Fail closed on drift.

The [kernel proc documentation](https://docs.kernel.org/filesystems/proc.html)
defines these credential projections. [No new privileges](https://docs.kernel.org/userspace-api/no_new_privs.html)
limits privilege gains through exec, not every possible privilege transition.
The observer does not change credentials, capabilities or the process itself.

## Tests and boundaries

Cover strict parsing, all required fields, numeric overflow, group bounds,
each UID/GID position, capabilities, leader mismatch and seeded mutations.
Exercise observation order, every failing stage and sticky refusal. Use only
owned non-product cat helpers for real retained-proc tests, including a helper
with NoNewPrivs set before exec, a normal helper refused without it, wrong
assertions, repeated/concurrent positional reads and refusal after exit.
The existing test-only process seam bypasses the fixed diagnostic cgroup;
successful cat observations therefore do not prove production worker identity.

No raw status, descriptors or observation-reset API is exported. No NSS lookup,
account creation, user namespace enrollment, IPC/helper mode, unit mutation,
output provisioning, launch, signal or cleanup route is added. The original
process and numeric policy remain caller assertions. Account-name mapping,
production composition, authenticated original start and launch authorization
remain separate prerequisites. The bound of 32 groups is an explicit local
refusal budget, not a change to host account configuration or qualification.

## Disk maintenance before baseline

Six ignored Cargo caches under this worktree's target directory were cleaned
with Cargo after checking exact paths, absence of reparse points and absence of
active compilers. Their inventory contained 19,703 files and 13,204,308,432
logical bytes. C: free space changed from 1,577,193,472 to 14,032,842,752 bytes;
the difference is an observation, not an exact physical deletion measurement.
Current caches, evidence and all worktrees were retained. Removed caches can be
rebuilt, not restored from trash. The preceding 27-capture evidence packet still
passed its offline verifier. This maintenance is not performance evidence.
