# Original collector pipes before authenticated start

Baseline `a72a1b6a84373f5006504de458599613b01d6327` acknowledges durable
Starting but owns no original worker pipes. This implements the first kernel
object preparation step of the approved collector architecture, not a start
producer, root sink constructor, production enrollment or writer cessation.

Consume OriginalCollectorStarting, retaining its complete original authority,
receiver, signed consent, storage descriptors and common fence. Create two
distinct anonymous pipes, original close-on-exec descriptors, nonblocking read
ends and blocking write ends. Keep every descriptor private; no caller path,
FD, Stdio, replacement owner or trust/time input is accepted or exported.

Check FIFO type, single link, owner/mode and retained dev/inode identity, matching
read/write pipe identity, distinct stdout/stderr, exact access/status flags and
close-on-exec. Bracket kernel observations with the original full Starting
revalidation. Freeze no new clock origin and cache no successful authority
proof. Construction checks before allocation, after each pipe initialization
and before returning; partial owned pipe sets survive failure with the refused
original owner and fence until explicit drop. A primitive syscall failure before
ownership transfer may close an unexported local FD; no worker can have received it.
Errors are sticky; consuming once cannot yield a retry, extraction or start API.

The object does not claim that kernel pipe capacity is an artifact byte ceiling.
The existing collector's 8 MiB/16 MiB ceilings remain necessary at the later
sink/pump integration. Blocking writers preserve backpressure. Queue emptiness,
leader exit and hangup are not EOF or all-writer cessation.

Retain an E0599 missing-entry test before implementation. Add actual Linux pipe
identity/flag, binary IO/EOF and full-queue backpressure tests, seeded flag
mutations and negative ownership/FD-export APIs. Run a separate joined manual
matrix on both normal GNU release compiler lanes: six construction interruptions,
eight descriptor/flag substitutions, original trust drift, concurrent flag drift
between observation brackets and a healthy control. Also refuse trust drift at
acknowledgement and an already-refused Starting before pipe allocation. These
extend the initial matrix to nineteen cases. Every chain is fresh; failed owners
retain Busy and existing durable history without mutation.
Descriptor substitutions and concurrent flag drift must report Invalid from
pipe inspection, not a coincident Original time refusal. Trust drift must report
the original guard failure. The acknowledgement hook precedes its final complete
original/kernel closing checks, preserving the same four full authority checks.
The actual owner declares pipes before Starting so Rust drops them before the
original fence. Check that exact declaration order and actual EOF after healthy
owner drop; this is closure of private unexported pipes, not all external writers.
No failed scenario is retried into a green result or admitted by extending time.

Preserve baseline runtime and all lower source guards with exact narrow static
restoration and canaries. No COMPAT artifact changes: this is an opaque runtime
type and anonymous kernel objects, not a persisted/wire record. Register fast
API and manual gates. Run affected current/MSRV check/strict Clippy, units/API/
compile-fail, original collector/IPC/lease/fence regressions, formatting,
static/docs/registry and nonpromotable local contracts. Retain evidence and
update the article with bounded results, not a throughput claim.

Authorized server use is only nonroot synthetic test binaries in owned temporary
trees, with the same read-only boot/supervisor/installed hashes/no-active-context
guards as the previous packet. No root mutation, installation, supervisor stop,
product process, output files, Actions dispatch or qualification is admitted.
Production trust/epoch and receiver/bootstrap selection remain unresolved.
The next implementation still needs sole-root sinks, original executable,
manager/process/cgroup handoff, independent watchdog and all-writer revocation
before completion/cells2–4; this slice grants none of those capabilities.
