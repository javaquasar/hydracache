# Original tracing thread progress amendment

The first development six-test run passed. The subsequent repeated run failed
its healthy delivered-cut observer with a five-second receive timeout before
the first report. The source performed construction and all five advances
before reporting, so the receiver incorrectly applied a single stage stall
limit to the aggregate. Concurrent MSRV verification passed its six cases;
this does not excuse the failed current-toolchain run.

Retain `thread-exit-linux-final.log` as a failed attempt, not final evidence.
Before rerunning, add credential-checked descriptor-free ordinal zero before
construction and ordinals one through the selected cut after each successful
original advance. The observer requires those exact ordinals before accepting
the original report. At most ten received packets replace the earlier eight
packet budget, accounting for the additional progress records rather than
extra FDs or data. The five-second per-gap, thirty-second outer and 8192-byte
capture bounds, positive terminal tuple, no-EXITKILL control prediction,
workload and admission boundaries remain unchanged.

This amendment does not establish the precise scheduler cause of the stall or
promise that arbitrary load can meet the bound. A further stalled or malformed
ordinal still fails. Subsequent complete repetitions must have separate logs.
