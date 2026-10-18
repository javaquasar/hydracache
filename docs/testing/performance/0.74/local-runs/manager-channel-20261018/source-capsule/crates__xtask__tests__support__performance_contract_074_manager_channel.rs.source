//! A fixed read-only producer is not original launch authority.
const COMPAT: &str = r###"
## Private fixed manager channel observations

This private owner uses standard D-Bus version 1 EXTERNAL authentication and
only Hello plus GetNameOwner, GetConnectionUnixUser and GetConnectionUnixProcessID
on the fixed root-selected system bus. It adds no public/durable codec. Actual
SO_PEERCRED and atomic SO_PEERPIDFD, retained procfs/nsfs identity and PID/user
namespace equality precede trust in namespace-relative PID 1/UID 0 replies.
Its METHOD_RETURN flags are exactly 0 or NO_REPLY_EXPECTED; that flag has no
reply semantics under D-Bus. Other known flags, wrong types and correlations
still refuse. The older exchange/request predicates remain unchanged.
The daemon's actual UID need not be zero; root-selected socket ancestry and the
retained kernel peer bind its provenance. Missing kernel access fails closed.

Client name, manager name, server GUID, boot, socket ancestry and original pidfds
remain owned by the live connection. A changed/closed/refused channel cannot
heal. The absolute BOOTTIME two-second observation cap is not an admission lease
and cannot renew any original window. There is no connection/name/FD export,
snapshot constructor, fixture promotion, message sender or execution authority.
Original frame/message joining and FD negotiation/delivery proof remain separate.
"###;
const GATES: &str = r###"
# BEGIN FIXED MANAGER CHANNEL LOCAL GATES

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.manager-channel-nonroot-refusal"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_manager_channel_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "manager_channel_nonroot_fixed_context_refusal"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual local nonroot fixed manager namespace-access refusal; no mutation"
tier = "manual"
required_features = []
required_env = ["HC074_LOCAL_MANAGER_READONLY"]
required_tools = ["cargo"]
timeout_seconds = 1200
owner_release = "0.74.0"
ship_mandatory = false
artifacts = []

[gate.ci]
workflow = ".github/workflows/ci.yml"
job = "gated-proof-registry"
step = "Run registered gated proofs"

[gate.command]
program = "cargo"
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::validation_frame::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::custody::descriptor_plan::barred_custody::barred_collector::enrollment_frame::manager_channel::tests::manager_channel_nonroot_fixed_context_refusal","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.manager-channel-root-readonly"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_manager_channel_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "manager_channel_root_fixed_context"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual local root read-only fixed manager context; no mutation"
tier = "manual"
required_features = []
required_env = ["HC074_LOCAL_MANAGER_READONLY"]
required_tools = ["cargo"]
timeout_seconds = 1200
owner_release = "0.74.0"
ship_mandatory = false
artifacts = []

[gate.ci]
workflow = ".github/workflows/ci.yml"
job = "gated-proof-registry"
step = "Run registered gated proofs"

[gate.command]
program = "cargo"
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::validation_frame::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::custody::descriptor_plan::barred_custody::barred_collector::enrollment_frame::manager_channel::tests::manager_channel_root_fixed_context","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END FIXED MANAGER CHANNEL LOCAL GATES
"###;
const FRAME_REG: &str = "\n#[path = \"diagnostic_manager_channel.rs\"]\nmod manager_channel;\n";
const GUARD_REG: &str =
    "#[path = \"performance_contract_074_manager_channel.rs\"]\nmod manager_channel;\n";
const GUARD_JOIN: &str = "    let normalized = manager_channel::normalize(path, raw);\n    let raw = normalized.as_str();\n";
const LOCK_PACKAGE: &str = r###"name = "hydracache-long-run-supervisor-074"
version = "0.0.0"
dependencies = [
 "base64",
 "ed25519-dalek",
 "fs2",
 "libc",
 "ring",
 "serde",
 "serde_json",
 "sha2 0.11.0",
 "tar",
 "tempfile",
 "thiserror 2.0.18",
 "toml",
 "windows-sys 0.61.2",
 "zbus",
 "zstd",
]
"###;
fn strip(raw: &str, allowed: &str) -> String {
    assert_eq!(
        raw.matches(allowed).count(),
        1,
        "manager-channel exception drift"
    );
    raw.replacen(allowed, "", 1)
}
fn restore_dependency_reference(path: &str, raw: &str) -> String {
    let root = path == "crates/xtask/tests/performance_contract_074.rs";
    let indent = if root { "    " } else { "        " };
    let begin = format!("{indent}// BEGIN FIXED MANAGER DEPENDENCY REFERENCE\n");
    let end = format!("{indent}// END FIXED MANAGER DEPENDENCY REFERENCE\n");
    assert_eq!(raw.matches(&begin).count(), 1);
    assert_eq!(raw.matches(&end).count(), 1);
    let start = raw.find(&begin).unwrap();
    let finish = raw.find(&end).unwrap() + end.len();
    let expected = if root {
        "let raw_lock=std::fs::read_to_string(root().join(\"Cargo.lock\")).unwrap();
         let current:toml::Value=toml::from_str(&original_start_consent::normalize(\"Cargo.lock\",&raw_lock)).unwrap();"
    } else {
        "let current=read(p);
         let current=if matches!(p,\"Cargo.lock\"|\"tools/long-run-supervisor-074/Cargo.toml\") {
           super::original_start_consent::normalize(p,&current)
         } else {current};
         assert_eq!(current,old(p),\"out of scope: {p}\");"
    };
    assert_eq!(
        compact(&raw[start + begin.len()..finish - end.len()]),
        compact(expected),
        "unapproved legacy dependency reference"
    );
    let old = if root {
        "    let current: toml::Value =\n        toml::from_str(&std::fs::read_to_string(root().join(\"Cargo.lock\")).unwrap()).unwrap();\n"
    } else {
        "        assert_eq!(read(p), old(p), \"out of scope: {p}\");\n"
    };
    format!("{}{}{}", &raw[..start], old, &raw[finish..])
}
pub(super) fn normalize(path: &str, raw: &str) -> String {
    match path {
        "docs/COMPAT.md" => strip(raw, COMPAT),
        "docs/testing/gated-test-registry.toml" => strip(raw, GATES),
        "tools/long-run-supervisor-074/src/diagnostic_original_enrollment_frame.rs" => {
            strip(raw, FRAME_REG)
        }
        "tools/long-run-supervisor-074/Cargo.toml" => strip(raw, "tokio.workspace = true\n"),
        "Cargo.lock" => {
            let changed = LOCK_PACKAGE.replace(" \"toml\",\n", " \"tokio\",\n \"toml\",\n");
            assert_eq!(raw.matches(&changed).count(), 1);
            raw.replacen(&changed, LOCK_PACKAGE, 1)
        }
        "crates/xtask/tests/support/performance_contract_074_dispatch_exchange.rs" => {
            strip(&strip(raw, GUARD_REG), GUARD_JOIN)
        }
        "crates/xtask/tests/performance_contract_074.rs"
        | "crates/xtask/tests/support/performance_contract_074_output_join.rs"
        | "crates/xtask/tests/support/performance_contract_074_starting_publication.rs" => {
            restore_dependency_reference(path, raw)
        }
        _ => raw.to_owned(),
    }
}
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
#[test]
fn manager_channel_preserves_old_authority_versions_workloads_and_sender_boundaries() {
    let mut paths = vec![
        "Cargo.toml".to_owned(),
        "Cargo.lock".to_owned(),
        "tools/long-run-supervisor-074/Cargo.toml".to_owned(),
        "docs/COMPAT.md".to_owned(),
        "docs/testing/gated-test-registry.toml".to_owned(),
        "docs/testing/performance/0.74/qualification-manifest.toml".to_owned(),
        "crates/xtask/tests/support/performance_contract_074_dispatch_exchange.rs".to_owned(),
        "crates/xtask/tests/performance_contract_074.rs".to_owned(),
        "crates/xtask/tests/support/performance_contract_074_output_join.rs".to_owned(),
        "crates/xtask/tests/support/performance_contract_074_starting_publication.rs".to_owned(),
    ];
    for name in [
        "diagnostic_manager.rs",
        "systemd_spawn.rs",
        "systemd_unit.rs",
        "diagnostic_dispatch_exchange.rs",
        "diagnostic_dispatch_exchange_tests.rs",
        "diagnostic_dispatch_exchange_socket_tests.rs",
        "diagnostic_original_enrollment_frame.rs",
        "diagnostic_original_enrollment_frame_build.rs",
        "diagnostic_original_grouped_preparation.rs",
        "diagnostic_original_custody_join.rs",
        "diagnostic_original_barred_custody.rs",
        "diagnostic_original_barred_collector_kernel.rs",
        "diagnostic_autonomous_watchdog.rs",
    ] {
        paths.push(format!("tools/long-run-supervisor-074/src/{name}"));
    }
    for path in paths {
        let out = std::process::Command::new("git")
            .args(["show", &format!("ff4f3b7e:{path}")])
            .current_dir(root())
            .output()
            .unwrap();
        assert!(out.status.success());
        assert_eq!(
            normalize(&path, &read(&path)),
            String::from_utf8(out.stdout).unwrap().replace('\r', ""),
            "{path}"
        );
    }
}
fn compact(raw: &str) -> String {
    raw.split_whitespace().collect()
}
fn closed(raw: &str) -> bool {
    let raw = compact(raw);
    [
        "auth_mechanism(zbus::AuthMechanism::External)",
        "max_queued(16)",
        "method_timeout(Duration::from_millis(250))",
        "owner(&connection,&client).await?!=manager",
        "uid!=0||pid!=1",
        "!unique(&name)||name==client",
        "kernel.inspect()?;",
        "paths.inspect()?;",
        "boot()?!=initial_boot",
        "self.connection.is_closed()",
        "self.refused=true;",
        "clock()?>=self.deadline",
        "biased;",
        "libc::SO_PEERPIDFD",
        "live(&peer)?;live(&init)?;",
        "stamp(&remote)?!=stamp(&local)?",
        "filesystem(&file,libc::NSFS_MAGICas_)?;",
        "libc::NS_GET_NSTYPE",
        "s.mode&0o022==0",
        "libc::TFD_TIMER_ABSTIME",
        "h.sender().map(|v|v.as_str())!=Some(BUS)",
        "!reply.data().fds().is_empty()",
        "reply.primary_header().flags().bits()&!(zbus::message::Flags::NoReplyExpectedasu8)!=0",
    ]
    .iter()
    .all(|v| raw.contains(v))
}
#[test]
fn manager_channel_live_auth_namespace_deadline_and_sticky_guards_have_canaries() {
    let source = read("tools/long-run-supervisor-074/src/diagnostic_manager_channel.rs");
    let kernel = read("tools/long-run-supervisor-074/src/diagnostic_manager_channel_kernel.rs");
    let raw = compact(&(source.clone() + &kernel));
    assert!(closed(&raw));
    for needed in [
        "auth_mechanism(zbus::AuthMechanism::External)",
        "owner(&connection,&client).await?!=manager",
        "uid!=0||pid!=1",
        "!unique(&name)||name==client",
        "self.refused=true;",
        "libc::SO_PEERPIDFD",
        "clock()?>=self.deadline",
        "stamp(&remote)?!=stamp(&local)?",
        "s.mode&0o022==0",
        "!reply.data().fds().is_empty()",
        "reply.primary_header().flags().bits()&!(zbus::message::Flags::NoReplyExpectedasu8)!=0",
    ] {
        assert!(!closed(&raw.replace(needed, "")), "{needed}");
    }
    assert!(
        source.find("readiness=alarm.readable()").unwrap() < source.find("result=future").unwrap()
    );
    let kernel = compact(&kernel);
    let peer = &kernel[kernel.find("fnpeer_pidfd(").unwrap()..];
    assert!(
        peer.find("File::from_raw_fd(value)").unwrap()
            < peer.find("iflenasusize!=size_of::<i32>()").unwrap()
    );
}
#[test]
fn manager_channel_has_no_execution_exports_fixture_promotion_or_caller_route() {
    for name in [
        "diagnostic_manager_channel.rs",
        "diagnostic_manager_channel_kernel.rs",
    ] {
        let raw = read(&format!("tools/long-run-supervisor-074/src/{name}"));
        for forbidden in [
            "pub fn",
            "Serialize",
            "DeserializeOwned",
            "impl Clone for FixedManagerChannel",
            "into_connection",
            "as_connection",
            "StartTransientUnit",
            "StopUnit",
            "Connection::system",
            "Builder::address",
            ".send(",
            "Command::",
            "std::env::",
        ] {
            assert!(!raw.contains(forbidden), "{name}: {forbidden}");
        }
    }
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_manager_channel.rs");
    assert!(compact(&raw).contains("fnopen_fixed()->Result<Self>"));
    assert_eq!(raw.matches(".call_method(").count(), 1);
    assert_eq!(raw.matches("GetNameOwner").count(), 1);
    assert_eq!(raw.matches("GetConnectionUnixUser").count(), 1);
    assert_eq!(raw.matches("GetConnectionUnixProcessID").count(), 1);
    assert!(read("docs/COMPAT.md").contains(COMPAT));
}
#[test]
fn manager_channel_manual_root_and_nonroot_observations_are_not_product_admission() {
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_manager_channel_tests.rs");
    for needed in [
        "manager_channel_nonroot_fixed_context_refusal",
        "manager_channel_root_fixed_context",
        "ChannelError::Namespace",
        "ChannelError::Refused",
        "HC074_LOCAL_MANAGER_READONLY",
        "original_join=false start=false",
    ] {
        assert!(raw.contains(needed), "{needed}");
    }
    assert!(read("docs/testing/gated-test-registry.toml").contains(GATES));
    assert_eq!(GATES.matches("[[gate]]").count(), 2);
}
