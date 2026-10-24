# Separate transport EOF oracle scope candidate

The first changed complete MSRV ordinary cohort selects all 1623 tests with four
threads. It retains a failure in
`cgroup_root_transport_size_flags_missing_credentials_and_fake_roots_refuse`:
the unchanged `eof` assertion at line 40 reads -1 rather than zero. The GNU
cohort passes that case. Neither result is retried or promoted to original
writer-retirement evidence. This MSRV failure is separate from its eight saved
clock-order events, whose parallel log adjacency does not identify test owners.

The earlier owned counterexample demonstrates why another concurrent fork can
temporarily retain a CLOEXEC writer. It does not identify the historical holder
in this particular failure. Static audit finds three callers of the same EOF
oracle: byte mutation (already isolated), malformed size/credentials/roots and
right-count/truncation. All create private pipes, drop their writer and require
immediate actual EOF after rejection. Keeping the remaining two in a shared
forking libtest process preserves the same unowned observation assumption.

Use a distinct candidate that adds the already checked exact owned-case wrapper
to only those remaining two callers. Preserve all original bodies, EOF value,
negative credential/descriptor predicates, test membership and deadlines. Parent
test execution remains parallel; only each audited old body has its own fresh
test image/process group. Keep the helper unchanged and prove exact source delta
before historical preservation views. Run the complete transport module once
per compiler, followed by check/strict lint and current/source mutation gates.

Do not rerun the full kernel/time cohort merely to find a green interval. No
manual original owner, service fixture, server action, clock modification,
product workload, all-writer cessation claim or enlarged live window follows.
Record the first candidate outcome even if it fails. A further complete ordinary
cohort needs its own changed-candidate rationale, not retry-to-pass.

The first v10 transport-module run passes six cases per compiler; the unchanged
manual case remains ignored and 1616 unrelated cases are filtered by this
focused selector. These are not full ordinary cohort results. The source map
stays unchanged. Both compilers pass all-target check and strict all-feature
Clippy; current source/mutation controls and xtask check/strict Clippy pass.
The complete earlier v9 ordinary refusals remain unchanged and authoritative.
