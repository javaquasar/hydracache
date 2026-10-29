# Local blocking-caller/runtime shadow

This is local preparation of the approved zbus migration. No root dependency,
supervisor body, validation frame, authority/pin, clock, service or server changes.
No system/session bus or real manager is contacted. The final implementation is
`bridge_v2.rs`; `bridge.rs` is the retained first prototype, not the final bridge.

Clone the exact prior F3 backport/compatible lock. For both controls enable p2p
only to use upstream's synthetic in-process Channel and constrain the existing
UUID 1.23.4. Registry identities remain equal to F3. The fixture lock adds only
the probe's UUID constraint and UUID's getrandom edge for p2p. These are test
prerequisites, not product feature/dependency adoption.

C uses tokio+proxy+p2p. Its first single progress contract fails outside block_on,
after the task has been released; reentry joins it with 74 before the assertion.
The failure is retained, not a timeout of the outer runner or an abandoned task.
M changes only tokio to tokio-multithread: all 13 controls pass. This matches the
old zbus 5 blocking API's multi-threaded runtime type and process-lifetime static
runtime. There is no new per-call runtime or unbounded worker queue.

The bridge holds/clones the original async connection. Proxy retains a connection
clone and drops its async proxy inside the runtime, as the old blocking wrapper
did. Address and ProxyBuilder setters convert immediately before build, retaining
old diagnostic error order/cache selection. Calls return the original Message or
MethodError unchanged. The bridge exposes only the two callers' used subset; it
is not a complete replacement for upstream's removed blocking module.

The original diagnostic_manager.rs body is independently pinned to SHA-256
7950deead36bcc6514f1c2f73be3cbce126a6e7a3bff3d1a55d11eac0be674db.
Select whole unique_owner/proxy/ReplyError/sender_matches/classify_error/
decode_reply/checked_call bodies. Change one blocking Builder path and three
wire alias spellings only. The inert inverse projection must equal those selected
original bodies. Do not claim this is the full manager/worker/boot/epoch census.

B adds a constructor priority regression to passing M. Its first selected test
fails after closing both synthetic endpoints: Proxy::new validates a bad name
before entering a nested runtime. R selects bridge_v2.rs, whose async constructor
is entered first, preserving the old source-derived panic priority. All 14 tests
pass on 1.88 and 1.94. First strict Clippy commands refuse three test setup
async-yields-async diagnostics on both compilers; keep them. S changes only
test overlay spellings to runtime_tests_v2.rs/caller_tests_v2.rs, obtaining an
explicit runtime Handle then spawning outside the async return. Tasks, checks,
GUID/body/timeout and bridge implementation remain the same. Final S passes
14 tests, check and strict Clippy on both compilers, plus scoped deny policy.

Use `python -B tools/dbus-runtime-bridge-074/run.py PHASE`. Historical fixed phases:
prepare-C, resolve-C-194, build/list/progress-C-194; prepare-M and build/list/test-M-194;
prepare-B and build/list/regress-B-194; prepare-R and build/list/test/check/clippy-R
on 194 and 188; prepare-S and build/list/test/check/clippy-S on both compilers,
then policy-S-194. Outputs are exclusive under target/dbus-runtime-20261028.
Preparation requires the retained prior MSRV fixture/receipts, not a new registry
resolution. Native builds bind compiler artifacts, SHA and exact catalogs (13 or
14). Keep the original 1200-second local envelope, 4 GiB floor and five-second
capacity polling/Job retirement. Test-only progress/read/join bounds use the
existing 250-ms method duration; no 300/60/10, 780/4800 or live limit changes.

The Channel bypasses OS authentication deliberately: every name/credential is
synthetic. Tests prove selected wire/error semantics, not real manager trust,
Unix FD transfer, kernel credentials, original delivery, watchdog/cgroup writer
retirement, reconnect or production cleanup. System connection/address build
and get_property bodies compile but are not exercised on a real bus. Property
types beyond the used String/u32 and the complete systemd_unit caller remain
unqualified. No product integration or performance claim follows.
