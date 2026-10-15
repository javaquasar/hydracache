# Bounded setup progress instead of one aggregate preparation stall

Parallel current/MSRV suites expose a harness defect: the report's five-second
wait starts before compilation/root preparation and all requested child stages.
The first current repeat passes, its next repeat refuses one report; MSRV refuses
two. Preserve these pressure captures as failed runs, not three green repeats.
No product workload or qualification threshold is involved.

Do not increase the five-second logical stall or30-second outer bound. Before
construction and after each original boundary, the tracer sends one setup-progress
byte with kernel credentials and no descriptors. Freeze the exact sequence:
0 before construction,1 at original initial stop,2 rooted,3 original exec,
4 installed,5 READY,6 delivered. A selected stage0..5 requires exactly its first
stage+2 progress packets, then the unchanged32-byte/five-FD final report. At most
eight packets are read, each within the existing five-second gap, while the
whole outer subprocess remains bounded by30 seconds. Wrong/duplicate/out-of-order
progress, descriptors on progress, missing credentials or wrong sender refuse.

Progress is a diagnostic hint, never signer readiness or kill admission. Only
the complete final report and all original kernel/root checks admit fault
injection. Original child execution still follows the unchanged launch/handoff
owner; no delay, retry, replacement, compiler/profile/policy change or external
capability is added. Add a wrong-progress refusal control at installed, attributed
to report parsing, and retain the earlier five report faults. The fixed report
body and its zero reserved fields remain unchanged; the private compatibility
contract records the setup prefix separately. Source and frozen inputs remain
under the same test-only scope.
