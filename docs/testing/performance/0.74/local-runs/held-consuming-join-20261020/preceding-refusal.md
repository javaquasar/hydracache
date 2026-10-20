# First frozen cohort refusal remains unclassified

Source 26d88288 used first-checks.ps1, SHA256
a782f74fcb92448b1f57b6d216e5a92fcb06f1d8528b04e89e64496d252462c6.
Both new 16-test GNU rounds passed. GNU's second preceding 26-test trust round
failed 25/26: the existing signed-other-request test received a result other than
its expected HeldTrustError::ContextMismatch. The old assertion did not retain
the unexpected value, so that log cannot establish its exact error variant or
cause. The first frozen overall GNU cohort is not accepted.

The original assertion and every runtime check/bound remain unchanged. A new
test-only wrapper invokes that exact old function under the existing first kernel
refusal recorder, without swallowing or retrying any failed assertion. Eight
fresh diagnostic samples passed with no recorded kernel refusal. This does not
reconstruct the first failure and does not prove a clock/timing or product cause.
The development log's accepted=true describes the assertion wrapper, not start
or runtime admission; the confirmed wrapper labels it assertions_ok instead.

The final source/recipe and confirmed/ cohort are separate. First logs, recipes
and completed Windows/MSRV/repository receipts remain untouched. Repetition does
not erase the unclassified refusal; diagnosis and the privileged full-holder
healthy/fault/cost gates remain open. No threshold, assertion, origin or deadline
is relaxed, and no product/server claim is made.

## Subsequent recorded clock refusal

At source 21369d07, confirmed/188-new-tests-1.log records clock-order, same boot,
before/after boottime 80156580664/80298384117 ns and realtime
1792480673413969387/1792480666771715299 ns. Boottime advances 141803453 ns while
realtime rolls back 6642254088 ns. The unchanged guard must refuse. This
attributes that later refusal, but cannot recover the first unlogged error.

Source eeca0d5e tries the previously approved exact local WSL timesyncd
stop/restore. stable-clock/194-new-tests-1.log still records clock-order with
realtime decreasing 6647001310 ns. The monotonic journal places the service stop
at 2.613 s, rollback observations at 23.999/24.142 s and restart at 24.877 s.
Therefore stopping that service alone is insufficient; its receipt complete=false
and restored active state remain evidence, not a healthy result.

A continuous 60-second read-only observation with the service active records no
backwards samples. The next held-session/ experiment keeps one WSL process alive
through the fresh tests, without changing clock services or clock values. Its
preparation phase precedes every new origin, and any test-phase rollback refuses
acceptance. This does not diagnose all possible clock corrections or qualify a
production time source. All earlier recipes and logs remain unchanged.

## Single-session observation is not authority

Source 542face4 retains one observer process through preparation and tests.
held-session/194-new-tests-1.log nevertheless records clock-order. The observer's
100 ms samples see no backwards value, which does not invalidate the original
guard's finer observation. This cohort is not accepted. Independent libc,
direct-syscall and Python samples, including forced CPU rotations, do not
reproduce the discrepancy. A global rollback, CPU-migration cause or sole clock
actor is therefore not established by these observations.

The same 17-test image passes an initial CPU-0-affined invocation. A separate
source-frozen affinity-smoke/ cohort records that restricted fixture context;
it is not a fix for unrestricted execution. No check/expiry/assertion is removed,
no clock source is replaced and no failed attempt is retried in place. The
unrestricted CPU and privileged full-holder qualification gates stay false.
