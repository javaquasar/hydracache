# Signer process executable object observation

Baseline is `3fa5e666be0d0fc095b4a536860afa9cc1a38336`. The next local Linux
step joins the original asserted signer bundle to an existing retained
ProcessRead. It proves a point-in-time kernel executable object match, not
original exec invocation, loaded instruction integrity, dynamic runtime or
production bootstrap custody.

## Fixed implementation scope

Add a crate-private executable-only ProcessRead guard. Reuse unchanged
ProcessIoRead executable metadata, read-only descriptor clone and fixed procfs
magic-link helper, exposing those only to sibling process modules. Bracket
original pidfd/generation/boot/group/cgroup validation around acquisition and
rechecks. Retain the expected read-only descriptor plus original `/proc/exe`
O_PATH/CLOEXEC target; compare dev/inode/UID/GID/mode/links/length/mtime/ctime
against fresh targets from the original retained proc directory. No arbitrary
path, PID or replacement observation is accepted by revalidation. Keep legacy
stream-binding predicates and public process constructors unchanged.

Add `AssertedSignerBundle::bind_asserted_process_executable(&mut self,
&ProcessRead) -> AssertedSignerExecutableRead` borrowing both original owners.
Check original bundle before/after executable binding and revalidation. Any
binding/check error latches bundle refusal; a returned guard also refuses
permanently. Healthy drop releases only additional observation descriptors,
never signals a process or refuses the healthy bundle. No public File/FD/path,
process/model export, Clone/Deserialize, replacement arguments, producer/CLI/
server/start/cleanup/signing consumer or durable/wire schema is added.

ProcessRead's existing production scope remains tied to diagnostic worker
cgroups. No controller-job enrollment or arbitrary public PID capture is added.
Tests use its existing cfg(test) owned-helper capture only. Runtime/bootstrap
contexts remain opaque and production issuer/pins unselected.

## Falsification and tests

Capture original bundle and process-IO baseline first, then missing-API
compile refusal. Use owned nonroot temporary bundles, synthetic signatures
and a copied `/bin/sh` local helper with a fixed stdin protocol. Spawn only
this owned helper with a cleared environment; commands either acknowledge,
exec `/bin/cat`, re-exec the same copied binary, or exit on stdin EOF. No
product process, real key/token, systemd/cgroup mutation, signal/SIGKILL,
external job or expensive workload. Handshake deadlines are harness bounds,
not timing correctness assertions or performance thresholds.

Test the actual original executable positive, equal-byte foreign inode/wrong
process, later different-inode same-generation exec, process exit, bundle
replacement/refusal, healthy drop, CLOEXEC/descriptor release, and cross-refusal
with no repair. Seed `0x07403b24` covers 16 fresh control/exec observations and
four concurrent independent originals. Include a positive same-inode re-exec
counterexample: generation and executable object can still match after an
actual re-exec. This limitation must remain explicit, not silently promoted
to authenticated original start. Compile-fail cases prohibit clone,
deserialization, FD export and replacement recheck arguments; extend the
existing registered Linux API target.

Run three focused repeats, original bundle/legacy process-IO/artifact/model/
producer regressions, doctests, affected check/strict Clippy on Rust 1.94 and
MSRV 1.88 Windows/Linux, root contracts/registry/docs/local performance gates.
Require-ship remains expected red. Retain tested source, unchanged/frozen
hashes, raw positive/negative/counterexample logs in a separate evidence packet;
keep preregistration, tested code and evidence/article commits separate.

Independent trusted ancestry/issuer/job custody, loader/runtime and memory
integrity, a reviewed pre-exposure bootstrap, original exec/start evidence,
credential coordination, replay/common fence and all-writer cessation remain
before production/server/pilot/qualification admission.
