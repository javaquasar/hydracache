# Ordinary supervisor regression: retain first failures and repair fixture scope

The complete non-ignored GNU 1.94 library suite at runtime candidate `81f4add2`
ran with four test threads and an outer 1800-second TERM/5-second kill bound.
It selected all 1622 tests: 1465 passed, 16 failed and 141 remained ignored.
Sources were unchanged throughout. This is a failed ordinary regression, not
an original-owner attempt, admission-window fit or product measurement. Do not
repeat it unchanged to obtain a green result.

## Separate repair hypotheses

1. Six IO-binding tests refuse in the same fixture `material()` at Contents.
   The fixture signs a modified historical hosted statement with its synthetic
   key and writes the current root lock, but leaves the historical lock digest
   in that statement. The hosted lock has 175300 bytes and SHA256
   `be46eaf8e97e507bda5e3f2b671d0d622e6a639ce1d4613769a4693723d62b19`;
   the current lock has 175330 bytes and SHA256
   `d2c7736ebaa8381cece9162bbbf384db60c46111051e4e78e51d4bae12feaf5c`.
   Git attributes the difference to three supervisor dependency edges, not a
   clock or parser change. Bind the synthetic signed statement to the exact
   lock bytes the fixture supplies. Keep production verification, the historical
   hosted receipt, current lock and all six assertions unchanged. Add a negative
   fixture receipt/lock mismatch control before accepting the correction.

2. The held-material drop test's comment assumes no other FD-opening threads,
   but ordinary libtest executes it beside such tests. It observes 15 descriptors
   against an expected 19. The root-transport test's immediate EOF oracle sees
   -1 instead of zero after rejecting three writer aliases. A parallel fork can
   inherit unrelated CLOEXEC descriptors before exec/closure; this explanation
   is an inference until an explicit owned-process control demonstrates it.
   Preserve the exact old FD and EOF assertions. Test a narrowly owned isolated
   libtest process for these oracles, with exact case selection, bounded wait,
   owned cleanup and no retry. Do not serialize or filter the complete suite,
   replace EOF with timeout success or weaken closure semantics.

3. Eight remaining failures involve real kernel/controller input authentication
   or time, including explicit Authentication(Time). Their precise reasons are
   not all proven by the current log. Keep original first outcomes and all live
   predicates and 300/60/10-second origins/windows. Attribute only from saved
   evidence and separately scoped controls; no clock/service correction or
   repeated original-owner attempt belongs to these repairs.

Each candidate is separate from the others and needs its focused passing gate,
check/lint, source-preservation/mutation controls and evidence before commit.
Only afterward run a distinct complete regression of the changed candidate.
No ignored/manual gate, server dispatch, service fixture, installation, supervisor
stop, production enrollment or product load is authorized by this ordinary gate.
