# Fresh Boot and Namespace versus authority attribution

Separate the existing fresh Boot and Namespace wall intervals from
ActiveAuthority without another live preparation or a changed observer.
The measured source remains `c1aa639990c4d2434a896ab7e90d93a0f8ce4b8a`;
the implementation remains `b074f8f9f3d8f9766b3b04c59a8291941a7e3b1d`.
Read only the sealed `single-session-rounds-bound-20261021` packet, inventory
`91a6e589235f2552ae9335680f47d4a20a418efeefcc3bc9fd2a05ba96f12afe`.
Verify the preceding observer packet and its chain before deriving new facts.

The method reuses every existing prefix, scope, refusal, completion and graph
check, then requires Boot and Namespace to have Origin as their only parent.
Boot has exactly one Document child per call; Namespace and Document have no
kernel children. Split Boot inclusive into outside-document and Document time.
Partition kernel time into Boot inclusive, Namespace and other kernel time.
Report ActiveAuthority and authentication independently. Cross-observer
intersections were not recorded: never subtract authority from the kernel
remainder or add its time to kernel spans as exclusive cost.

Source inspection binds these labels to actual behavior: Boot performs four
fresh openat/fstatfs pairs and a bounded positional boot document read;
Namespace opens the fixed current-thread nsfs link and checks filesystem and
metadata. Origin's retained-descriptor checks remain in its own shell.
ActiveAuthority calls inspect, including fresh authority ancestry/inventory/
record checks, rather than measuring only signature verification. Per-syscall,
exclusive CPU and observer-free costs remain unknown.

All recorded phases/steps are retained, including incomplete last rounds.
GNU and MSRV each have one refused prefix, not repeatable performance cohorts.
Use the complete 46-round outputs subphase for clearly scoped illustrations;
do not replace or repair ten-second expirations. Preserve the original
300/60/10 windows, every live predicate and source bytes. No clock/service,
server, supervisor, installation, product workload or qualification action.

Controls use actual sealed rows and mutations: conservation, changed parent
with conserved graph, authority child/time changes, invalid numeric types,
duplicate edges, empty step, zero-call timing, exclusive output creation,
full reconstruction and false admission gates. Run Python tests on Windows
and local Linux, plus the existing eighteen input-reader tests, format,
documentation, gated-test registry, local performance contract and diff checks.
Freeze method before capture; seal a distinct derived packet exactly once.
No Rust package changes or new runtime candidate are part of this step.
