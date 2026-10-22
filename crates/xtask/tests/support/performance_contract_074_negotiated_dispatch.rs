//! Exact additive fixture registration; production owners and deadlines stay frozen.
#[path = "performance_contract_074_reply_state.rs"]
mod reply_state;
const BASE: &str = "a36ac6e8";
const PATH: &str = "tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange_tests.rs";
const REG: &str =
    "\n#[path = \"diagnostic_dispatch_negotiated_tests.rs\"]\nmod negotiated_tests;\n";
const GATE: &str = r###"
# BEGIN NEGOTIATED DISPATCH GATE

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.negotiated-dispatch"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_dispatch_negotiated_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "dispatch_negotiated_private_bus_fault_matrix"
cfg = 'target_os = "linux"'
env = ""
reason = "manual nonroot private dbus-daemon negotiation; no system bus or execution"
tier = "manual"
required_features = []
required_env = []
required_tools = ["cargo", "dbus-daemon"]
timeout_seconds = 120
owner_release = "0.74.0"
ship_mandatory = false
artifacts = []

[gate.ci]
workflow = ".github/workflows/ci.yml"
job = "gated-proof-registry"
step = "Run registered gated proofs"

[gate.command]
program = "cargo"
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::validation_frame::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::custody::descriptor_plan::barred_custody::barred_collector::enrollment_frame::dispatch_exchange::tests::negotiated_tests::dispatch_negotiated_private_bus_fault_matrix","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END NEGOTIATED DISPATCH GATE
"###;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = reply_state::normalize(path, raw);
    let raw = current.as_str();
    let exact = match path {
        PATH => REG,
        "docs/testing/gated-test-registry.toml" => GATE,
        _ => return raw.to_owned(),
    };
    assert_eq!(
        raw.matches(exact).count(),
        1,
        "negotiated fixture registration drift"
    );
    raw.replacen(exact, "", 1)
}
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    crate::runtime_io::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn old(path: &str) -> String {
    let output = std::process::Command::new("git")
        .args(["show", &format!("{BASE}:{path}")])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().replace('\r', "")
}
#[test]
fn negotiated_dispatch_preserves_all_old_runtime_and_test_bodies() {
    let output = std::process::Command::new("git")
        .args([
            "ls-tree",
            "-r",
            "--name-only",
            BASE,
            "--",
            "tools/long-run-supervisor-074/src",
        ])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    let paths: Vec<_> = text.lines().filter(|p| p.ends_with(".rs")).collect();
    assert_eq!(paths.len(), 356);
    for path in paths {
        assert_eq!(normalize(path, &read(path)), old(path), "{path}");
    }
    let path = "docs/testing/gated-test-registry.toml";
    assert_eq!(normalize(path, &read(path)), old(path));
    let raw = read(PATH);
    assert_ne!(
        normalize(PATH, &raw.replacen("fn pipe()", "fn changed()", 1)),
        old(PATH)
    );
    assert!(std::panic::catch_unwind(|| normalize(PATH, &raw.replacen(REG, "", 1))).is_err());
}
#[test]
fn negotiated_dispatch_is_private_bus_only_and_does_not_bypass_exchange() {
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_dispatch_negotiated_tests.rs");
    for token in [
        "Command::new(\"/usr/bin/dbus-daemon\")",
        ".arg(\"--nofork\")",
        ".arg(\"--config-file\")",
        "AuthMechanism::External",
        "socket.peer_cred()",
        "NEGOTIATE_UNIX_FD",
        "AGREE_UNIX_FD",
        "DispatchExchange::prepare(",
        "exchange.arm()",
        "exchange.lose_reply()",
        "exchange.observe(&response)",
        "ExchangeError::AlreadyAttempted",
        "writer_shape(fd.as_raw_fd())",
        "writer_shape(original.as_raw_fd())",
        "checked_add(1)",
        "zbus::Error::Unsupported",
        "drop(exchange);",
        "write_original(&received, &mut readers);",
        "self.child.kill()",
        "self.child.wait()",
    ] {
        assert!(raw.contains(token), "{token}");
    }
    assert!(
        raw.find("exchange.arm().unwrap()").unwrap()
            < raw.find("client.send(&exchange.request)").unwrap()
    );
    for denied in [
        "Connection::system",
        "Connection::session",
        "Builder::system",
        "Builder::session",
        "FixedManagerChannel",
        "OriginalManagerJoin",
        "systemctl",
        "sudo",
        "SIGKILL",
        "/run/dbus",
        "Command::new(\"sh\")",
        "set_var(",
        "set_unique_name",
    ] {
        assert!(!raw.contains(denied), "{denied}");
    }
    let atomic =
        read("tools/long-run-supervisor-074/src/diagnostic_dispatch_negotiated_recv_tests.rs");
    assert!(atomic.contains("libc::MSG_DONTWAIT | libc::MSG_CMSG_CLOEXEC"));
    assert!(!atomic.contains("F_SETFD"));
    assert!(
        atomic.find("fds.push(OwnedFd::from_raw_fd").unwrap()
            < atomic.find("if count == 0").unwrap()
    );
    assert!(raw.contains("case != \"bare-receiver\""));
    assert!(raw.contains("Err(ExchangeError::Request)"));
}
