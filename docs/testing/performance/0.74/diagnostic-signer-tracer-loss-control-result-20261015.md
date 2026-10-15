# Failed exact exit prediction without EXITKILL

The first directory-correct ten-case run passes nine tests. Actual tracer SIGKILL
at initial/rooted/first-exec/installed/ready/delivered produces exact original
child CLD_KILLED/SIGKILL, and healthy/report-refusal checks pass. The remaining
control fails at installed: actual CLD_EXITED/74, predicted CLD_EXITED/73. Retain
that failed prediction, helper bytes and original log; it is not a live orphan
or a failed positive EXITKILL observation.

Removing EXITKILL allows the unchanged child to run while tracer death and
descriptor-table teardown progress. The READY send can succeed before its peer
is fully closed, followed by failed/EOF nonblocking read. No dummy is sent at
installed or ready. An exact natural-exit sequence is therefore not the new
control's invariant. The hypothesis to test is whether only EXITKILL yields
SIGKILL, while its absence follows bounded reviewed helper branches without a
direct child kill. This does not change a product threshold or qualification
oracle; it replaces an explicitly failed fixture prediction with a separate
source-derived control contract before its repeated observations.

Freeze the new closed natural-exit sets from the unchanged helper's only reachable
branches: installed73 (READY fails) or74 (no dummy read); ready74; delivered76
(ACK send fails) or0 (ACK and close succeed before peer teardown). Require
CLD_EXITED, never CLD_KILLED or SIGKILL. Reject every other status/code and test
the finite oracle exhaustively. Consume the exact observed terminal tuple and
require ECHILD on repeat, without introducing sleep, artificial peer lifetime,
new helper bytes, relaxed positive SIGKILL checks or automatic retry. Log actual
tuples separately from the superseded exact73/74/76 prediction. No alive-orphan
counterexample is claimed by a natural exit.
