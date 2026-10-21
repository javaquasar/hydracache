# Pure baseline for trust component C strings

Freeze the existing `CString::new(name)` operation in worker trust input's
private `child` helper before changing it. It is shared by worker, controller,
held-runtime and OIDC source readers. Inspect the six production component
names and retain generic compatibility rather than treating literals as the
only accepted input. No runtime body or call graph changes in this baseline.

Measure Rust allocation requests through the existing opt-in thread-local
counter, with 256 constructions per named case. Include fixed components,
empty and short names, lengths around 31/32 and 63/64, 255/256, a 4096-byte
generic name, Unicode and slash/dot-containing inputs. Separately observe
leading/interior/trailing embedded-NUL refusals and retain error bytes/position;
do not mistake a conversion refusal for a filesystem call or successful open.

Freeze the complete `child` body in a pure model, substituting only the openat
operand, last-error operand and File ownership construction. Preserve conversion
before the open, exact flags/parent/terminated bytes, one callback, negative-fd
handling, last-error schedule and no retry. Seed the schedule/error differential
and retain explicit NUL priority, generic input and independent ownership tests.
Source canaries must reconstruct the complete production body and entire old
allocator-test file; older guards reverse exactly one new private test hook.

Run only pure controls/source canaries and affected all-target check/strict
all-feature lint on host/GNU/MSRV, formatting and local structural gates. Keep
first logs and refusals; a compile/lint repair needs a separate cohort. Do not
invoke ignored/manual tests, original-owner/live profiles, clock/service changes,
SSH/server/Actions, installation, supervisor or product workloads. Preserve
300/60/10-second windows and both first Causal(Time) failures. Verify all seven
prior raw seals without changing source-pinned historical reviewers.

This is an allocation/error baseline, not a stack implementation or timing
comparison. A later candidate must keep arbitrary-name compatibility, embedded
NUL priority and exact syscall semantics with its own isolated tests/evidence.
Pure success opens no material, all-writer, fault/reconnect or server gate.
