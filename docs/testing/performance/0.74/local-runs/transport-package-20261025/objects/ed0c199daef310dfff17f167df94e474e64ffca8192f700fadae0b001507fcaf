//! Only exact new registrations may differ from the prior private owner.
const GATES: &str = r###"
# BEGIN ORIGINAL ADDRESSED MESSAGE LOCAL GATES

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-grouped-addressed-controls"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_manager_message_frame_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_grouped_addressed_frame_controls"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual nonroot original addressed message controls; no systemd dispatch"
tier = "manual"
required_features = []
required_env = ["HC074_START_MATERIAL_BUNDLE"]
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::enrollment_tests::manager_tests::addressed_tests::original_grouped_addressed_frame_controls","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-grouped-addressed-refusals"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_manager_message_frame_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_grouped_addressed_frame_refusals"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual nonroot original addressed message refusals; no systemd dispatch"
tier = "manual"
required_features = []
required_env = ["HC074_START_MATERIAL_BUNDLE"]
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::enrollment_tests::manager_tests::addressed_tests::original_grouped_addressed_frame_refusals","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-grouped-addressed-fixed-refusal"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_manager_message_frame_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_grouped_addressed_nonroot_fixed_refusal"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual nonroot grouped fixed-channel refusal retains the original owner"
tier = "manual"
required_features = []
required_env = ["HC074_START_MATERIAL_BUNDLE"]
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::enrollment_tests::manager_tests::addressed_tests::original_grouped_addressed_nonroot_fixed_refusal","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ORIGINAL ADDRESSED MESSAGE LOCAL GATES
"###;
const COMPAT: &str = r###"
## Private original uniquely addressed message

The private consuming message owner retains the original live manager join,
unchanged legacy message and complete frame. A second unsent message uses only
that connection's retained unique manager/client binding, canonical original
body and serial, and two distinct pinned writer aliases for the original pipes.
The extra aliases close before original custody. The constructor consumes the
original frame directly, retaining the same fixed channel and receiver binding.
Pure preparation occurs inside one complete frame
with channel/context checks on both sides; later checks use that same round.
All original live predicates, origins, alarms and deadlines remain. Failure
retains and poisons the same owner, with no extraction, retry, reconstruction,
send or start capability.

No public/durable format changes. Same-process SCM_RIGHTS fixture delivery is
not negotiated system-bus delivery, production enrollment, active start,
whole-writer cessation, completion or server admission.
"###;
const OWNER: &str = "\n#[cfg(test)]\n#[path = \"diagnostic_original_manager_message_red.rs\"]\nmod message_contract;\n\n#[path = \"diagnostic_original_manager_message.rs\"]\nmod message_owner;\n";
const API: &str =
    "\n#[path = \"diagnostic_original_manager_message_test_api.rs\"]\nmod addressed_tests;\n";
const FIXTURE: &str =
    "\n#[path = \"diagnostic_original_manager_message_frame_tests.rs\"]\nmod addressed_tests;\n";
const TRANSFER: &str =
    "\n#[path = \"diagnostic_original_addressed_transfer_tests.rs\"]\nmod original_addressed;\n";
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let exact = match path {
        "docs/COMPAT.md" => COMPAT,
        "docs/testing/gated-test-registry.toml" => GATES,
        "tools/long-run-supervisor-074/src/diagnostic_original_manager_join.rs" => OWNER,
        "tools/long-run-supervisor-074/src/diagnostic_original_manager_join_tests.rs" => API,
        "tools/long-run-supervisor-074/src/diagnostic_original_manager_frame_tests.rs" => FIXTURE,
        "tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange_socket_tests.rs" => {
            TRANSFER
        }
        _ => return raw.to_owned(),
    };
    assert_eq!(
        raw.matches(exact).count(),
        1,
        "addressed registration drift"
    );
    raw.replacen(exact, "", 1)
}
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(name: &str) -> String {
    let raw = crate::runtime_io::read_to_string(root().join(name))
        .unwrap()
        .replace('\r', "");
    crate::live_walk::normalize(name, &raw)
}
fn compact(raw: &str) -> String {
    raw.split_whitespace().collect()
}
#[test]
fn original_grouped_message_preserves_old_manager_frame_message_exchange_and_custody() {
    for name in [
        "diagnostic_original_manager_join.rs",
        "diagnostic_original_manager_join_tests.rs",
        "diagnostic_original_manager_frame_tests.rs",
        "diagnostic_dispatch_exchange_socket_tests.rs",
        "diagnostic_manager_channel.rs",
        "diagnostic_manager_channel_kernel.rs",
        "diagnostic_original_pipe_transport.rs",
        "diagnostic_original_enrollment_frame.rs",
        "diagnostic_original_enrollment_frame_build.rs",
        "diagnostic_original_custody_join.rs",
        "diagnostic_original_descriptor_plan.rs",
        "diagnostic_dispatch_exchange.rs",
        "diagnostic_original_grouped_preparation.rs",
        "diagnostic_autonomous_watchdog.rs",
    ] {
        let path = format!("tools/long-run-supervisor-074/src/{name}");
        let old = std::process::Command::new("git")
            .args(["show", &format!("d506d4d9:{path}")])
            .current_dir(root())
            .output()
            .unwrap();
        assert!(old.status.success());
        assert_eq!(
            normalize(&path, &read(&path)),
            String::from_utf8(old.stdout).unwrap().replace('\r', ""),
            "{path}"
        );
    }
}
fn closed(raw: &str) -> bool {
    let s = compact(raw);
    let Some(round) = s
        .split("pub(super)fnround(")
        .nth(1)
        .and_then(|part| part.split("pub(crate)structOriginalManagerMessage").next())
    else {
        return false;
    };
    let mut rest = round;
    for step in [
        "ifpayload.refused||!frame.ready||frame.original.is_refused()",
        "frame.observe().map_err(JoinError::from)?;",
        "channel.check(receiver,pin)?;",
        "frame.observe().map_err(JoinError::from)?;",
        "letinput=inputs(frame)?;",
        "payload.check(&input,manager,client)?;",
        "frame.revalidate().map_err(JoinError::from)?;",
        "channel.check(receiver,pin)?;",
        "payload.check(&inputs(frame)?,manager,client)?;",
        "frame.observe().map_err(JoinError::from)?;",
    ] {
        let Some((_, after)) = rest.split_once(step) else {
            return false;
        };
        rest = after;
    }
    [
        "message.body().data().bytes()!=input.legacy.body().data().bytes()",
        "aliases(message)?!=pinned",
        "serial!=input.legacy.primary_header().serial_num()",
        "canonical::descriptor(&streams[0].1,input.writers[0])?;",
        "canonical::descriptor(&streams[1].1,input.writers[1])?;",
        "frame.original.refuse();",
        "payload.refused=true;",
        "ifpayload.pinned.is_some()||payload.serial.is_some()||payload.names.is_some()",
        "letmanager=channel.manager.clone();",
        "letclient=channel.client.clone();",
        "payload:Payload,original:OriginalManagerJoin",
        "held:Box::new(held)",
        "self.original.ready=false;",
        "held.original.receiver=Some(retain_receiver(&mutheld.original.original)?);",
        "Some(FixedManagerChannel::open_fixed().map_err(|_|JoinError::Channel)?);",
    ]
    .iter()
    .all(|part| s.contains(part))
}
#[test]
fn original_grouped_message_round_body_alias_order_and_refusal_have_canaries() {
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_original_manager_message.rs");
    assert!(closed(&raw));
    let raw = compact(&raw);
    assert!(closed(&raw));
    assert_eq!(
        raw.matches("frame.revalidate().map_err(JoinError::from)?;")
            .count(),
        1
    );
    assert_eq!(raw.matches("channel.check(receiver,pin)?;").count(), 2);
    assert!(!raw.contains("validation_round("));
    assert!(
        raw.contains("#[cfg(test)]iffault==Some(\"delivery\"){payload.transfer_for_test(frame);}")
    );
    for part in [
        "frame.revalidate().map_err(JoinError::from)?;",
        "channel.check(receiver,pin)?;",
        "frame.observe().map_err(JoinError::from)?;",
        "message.body().data().bytes()!=input.legacy.body().data().bytes()",
        "aliases(message)?!=pinned",
        "frame.original.refuse();",
        "payload.refused=true;",
        "self.original.ready=false;",
    ] {
        assert!(!closed(&raw.replace(part, "")), "{part}");
    }
    for denied in [
        ".send(",
        "sendmsg(",
        "pubfn",
        "Serialize",
        "Deserialize",
        "derive(Clone",
        "into_message",
        "into_original",
        "into_channel",
    ] {
        assert!(!raw.contains(denied), "{denied}");
    }
    assert!(raw.contains("pub(crate)fnprepare_fixed_manager_message(self,)"));
    assert!(raw.contains("canonical::inspect_message"));
    assert!(read("docs/COMPAT.md").contains(COMPAT));
}
#[test]
fn original_addressed_delivery_adapter_is_test_only_and_cannot_construct_a_fixed_owner() {
    let api =
        read("tools/long-run-supervisor-074/src/diagnostic_original_manager_message_test_api.rs");
    assert!(!api.contains("OriginalManagerJoin {"));
    assert!(!api.contains("OriginalManagerMessage {"));
    let parent = read("tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange.rs");
    assert!(parent.contains("#[cfg(test)]\n#[path = \"diagnostic_dispatch_exchange_tests.rs\"]"));
    let adapter =
        read("tools/long-run-supervisor-074/src/diagnostic_original_addressed_transfer_tests.rs");
    assert!(adapter.contains("transfer(&sender, &receiver, message)"));
    assert!(!adapter.contains("FixedManagerChannel"));
}
