# Fresh kernel and authority wall attribution

The new offline method separates actual fresh Boot and Namespace intervals
from the independently observed ActiveAuthority calls. It runs no new timing
profile and changes no runtime or observer. Capture source is
`d17179f32aa0adcaa05ac47ec88d4a1020189762`; the measured source remains
`c1aa639990c4d2434a896ab7e90d93a0f8ce4b8a` in the sealed single-session
packet. The [method plan](../../fresh-kernel-authority-plan-20261021.md)
freezes inputs, interpretation and controls.

## Recorded fresh operations

These are instrumented wall totals from the complete 46-round outputs
ReservedLive step in each original refused prefix. GNU and MSRV have one
observation each; the columns are not a compiler performance comparison.

| Recorded span | Calls per lane | GNU 1.94 seconds | MSRV 1.88 seconds |
| --- | ---: | ---: | ---: |
| Boot outside its Document child | 129904 | 1.053708 | 0.938471 |
| Boot Document child | 129904 | 0.124464 | 0.090995 |
| Boot inclusive of Document | 129904 | 1.178172 | 1.029466 |
| Namespace | 259808 | 0.935155 | 0.781214 |
| Other kernel intervals | — | 1.808547 | 1.435204 |
| All kernel intervals | — | 3.921874 | 3.245885 |
| Step wall outside kernel intervals | — | 2.426508 | 2.050431 |
| ActiveAuthority independently observed | 14812 | 0.993633 | 0.834129 |
| Authentication independently observed | 736 | 0.081994 | 0.082488 |

Boot outside Document plus Document equals Boot inclusive. Only Boot inclusive,
Namespace and other kernel intervals form the disjoint kernel partition.
Their total plus outside-kernel wall equals step wall. Other Document reads
are a subset of other kernel intervals, not an additional component.

The previous 1.988862/1.719685-second Boot-plus-Namespace outside-children
aggregate excludes Boot's Document child. Including that child yields
2.113327/1.810681 seconds. This refines accounting; it is not extra work added
by this analysis. All raw nanoseconds, counts and every phase/step remain in
[facts.json](facts.json), including unfinished final rounds.

Actual Boot opens four proc children with filesystem checks, reads the bounded
boot document positionally and validates its bytes. The Document timer covers
the bounded read, not the surrounding open/close, filesystem checks or decode.
Actual Namespace opens the current-thread fixed nsfs link and checks filesystem
and metadata. Retained namespace FD metadata checks remain in Origin's shell,
not Namespace. These are recorded fresh calls, not cached identities or a
standalone approximation of the syscall bodies.

ActiveAuthority wraps registry inspect and refusal handling, including live
directory/inventory/record checks. It is not the authentication timer or a
cryptography-only category. Cross-observer timestamps/edges were not recorded;
its intersection with kernel or authentication intervals cannot be determined.
In particular, outside-kernel wall is not an authority remainder. Do not add
these observers, infer exclusive CPU, subtract synthetic observer overhead,
or attribute the whole Boot shell to openat/fstatfs alone. Allocations and
individual syscall costs are not separated by these traces.

## Controls and unchanged admission

The method inherits all eighteen original reader checks and adds ten controls
on each of Windows and Linux. New controls cover the exact partition, original
refusals, a changed parent with conserved graph counts/times, authority child
and wall mutations, invalid numeric types, duplicate edges, empty steps,
zero-call timing, exclusive publication/reconstruction and numeric Boolean
gate substitution. Type-strict JSON reconstruction rejects `0` for `false`.

The GNU age of 10,175,057,064 ns and MSRV age of 10,006,497,560 ns still refuse
at their original sites. Neither is converted to a healthy admission. The
original runtime/observers remain byte-identical to `b074f8f9`; qualification
digest remains `11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
Original 300/60/10 windows and every live check remain. No clock, service,
server, supervisor, installation or product action occurs.

Format, docs, registry, local performance contract and diff checks pass. All
twenty-four preceding packets verify; this packet adds only derived evidence.
No Rust code changes, compiler test rerun or full release qualification is
claimed. New profile invocations are zero, and native admission remains closed.

After sealing, run `tools/local-profile-session-074/review_fresh.ps1 -Mode Review`.
Never rerun capture, replace observations or reseal this directory. The next
cost experiment should isolate fresh Boot setup/teardown versus Namespace
and the authority reader at their real callsites, preserving fresh reads,
descriptor lifetimes, first errors and the original validation frame. This
partition selects that question but accepts no optimization or speedup claim.
