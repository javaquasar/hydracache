//! Remove only the exact new protocol and manual gate, not any old bytes.
pub(crate) const COMPAT: &str = "docs/COMPAT.md";
pub(crate) const BLOCK: &str = r##"
<!-- BEGIN LOCAL HELD LAUNCHER V1 -->
## Local inherited descriptor held launcher v1

HCHL0741 bootstrap is exactly 256 little-endian bytes in a read-only memfd with
exact SEAL, GROW, SHRINK and WRITE seals. Offsets: deadline u64 at 8; owner
PID/UID/GID u32 at 16/20/24; guardian PIDs u32 at 28/32; token32 at 40;
worker/config SHA256 bytes at 72/104; worker/config device/inode/length u64
triples at 136/160; three stdio device/inode pairs at 184/200/216; control pair
at 232. Bytes 36..40 and 248..256 are zero. Token is nonzero; process IDs are
distinct positive non-self IDs. Worker/config bounds are 128 MiB/64 KiB.

HCLR0741 Ready=1 and Release=2 are exactly 80 bytes: magic8, kind at 8, zero
bytes 9..16, token32 at 16 and bootstrap SHA256 bytes at 48. Unknown versions,
kinds, reserved bits, lengths, changed bindings, ancillary rights/truncation,
wrong owner credentials and queued extra packets refuse. Require one exact
SCM_CREDENTIALS matching the fixed AF_UNIX SEQPACKET peer and bootstrap owner.
Ready is not watchdog Armed, execution ACK or a completion assertion. EOF,
failed send and lost confirmation are uncertain and never permit a resend.

The new local diagnostic_held_launcher target uses only inherited descriptors
0..10: stdio, control3, worker4, config5, owner6, guardians7/8, alarm9 and
bootstrap10. Only stdio/config survive its one execveat with AT_EMPTY_PATH;
there is no pathname/command fallback. Worker argv is fixed --run with
/proc/self/fd/5, expected digest and frozen worker source. Environment is empty.
Helper refuses root/set-ID, capabilities or extra threads and sets no_new_privs.
Config is sealed; worker must be sealed or root-owned executable without group/
world write permission. Root is trusted for the latter, not independently fenced.
The inherited absolute CLOCK_BOOTTIME one-shot alarm is revalidated against the
unchanged supplied deadline; it is never consumed or rearmed by the helper.

HCLF0741 is a fixture-only eight-byte carrier with exactly four SCM_RIGHTS:
held pidfd, two guardian pidfds and stdout reader. Its one SCM_CREDENTIALS must
identify the externally launched fixture owner. An owned subreaper harness uses
it for real owner SIGKILL and exact orphan cleanup; it is not production IPC.

These formats have no migration, fallback or upgrade from worker, signed launch,
placement or guardian protocols. Existing formats/live checks/deadlines remain.
They add a local target to the previously reserved recipe, not a production unit,
installed path, enrolled authority or original-owner/full-frame join. Guardian
liveness is not arming/all-writer cessation. No Running/completion/unlock or server
admission follows from helper success or this local nonproduct fixture.
<!-- END LOCAL HELD LAUNCHER V1 -->
"##;
pub(crate) const REGISTRY: &str = "docs/testing/gated-test-registry.toml";
pub(crate) const GATE: &str = r##"
# BEGIN LOCAL HELD LAUNCHER GATES
[[gate]]
id = "cfg.hydracache-long-run-supervisor-074.held-launcher"
kind = "cfg_test_target"
source = "tools/long-run-supervisor-074/tests/held_launcher.rs"
package = "hydracache-long-run-supervisor-074"
target = "held_launcher"
test = ""
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "local held launcher synthetic ELF fixture target; process matrix remains manual"
tier = "fast"
required_features = []
required_env = []
required_tools = ["cargo"]
timeout_seconds = 180
owner_release = "0.74.0"
ship_mandatory = false
artifacts = []
[gate.ci]
workflow = ".github/workflows/ci.yml"
job = "rust"
step = "Release 0.74 Linux supervisor contracts"
[gate.command]
program = "cargo"
args = ["test","--locked","-p","hydracache-long-run-supervisor-074","--test","held_launcher"]
cwd = "."
platform = "linux"
[gate.command.env]

[[gate]]
id = "ignored.hydracache-long-run-supervisor-074.held-launcher.owned-launcher-process-fault-matrix"
kind = "ignored_test"
source = "tools/long-run-supervisor-074/tests/held_launcher.rs"
package = "hydracache-long-run-supervisor-074"
target = "held_launcher"
test = "owned_launcher_process_fault_matrix"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "owned nonroot synthetic ELF process/FD fault matrix; no systemd or product"
tier = "manual"
required_features = []
required_env = []
required_tools = ["cargo","/bin/sleep"]
timeout_seconds = 180
owner_release = "0.74.0"
ship_mandatory = false
artifacts = []
[gate.ci]
workflow = ".github/workflows/ci.yml"
job = "gated-proof-registry"
step = "Run registered gated proofs"
[gate.command]
program = "cargo"
args = ["test","--locked","-p","hydracache-long-run-supervisor-074","--test","held_launcher","owned_launcher_process_fault_matrix","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"
[gate.command.env]
# END LOCAL HELD LAUNCHER GATES
"##;
pub(crate) fn normalize(path: &str, raw: &str) -> String {
    let exact = match path {
        COMPAT => BLOCK,
        REGISTRY => GATE,
        _ => return raw.into(),
    };
    assert_eq!(
        raw.matches(exact).count(),
        1,
        "exact held launcher addition: {path}"
    );
    raw.replacen(exact, "", 1)
}
