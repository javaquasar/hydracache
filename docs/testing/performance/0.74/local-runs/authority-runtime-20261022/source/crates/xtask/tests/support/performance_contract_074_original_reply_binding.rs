//! Exact original-owner additions only; old predicates and deadlines stay frozen.
#[path = "performance_contract_074_two_phase_launch.rs"]
mod two_phase_launch;
const BASE: &str = "ba017a7d";
const EDITS: &[(&str, &str, &str)] = &[
    (
        "docs/testing/gated-test-registry.toml",
        r####"
# BEGIN ORIGINAL REPLY BINDING GATE

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-reply-binding-controls"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_reply_binding_frame_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_reply_binding_frame_controls_and_refusals"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual nonroot original reply binding frame controls; no systemd dispatch"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::enrollment_tests::manager_tests::addressed_tests::reply_binding::original_reply_binding_frame_controls_and_refusals","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ORIGINAL REPLY BINDING GATE
"####,
        r####""####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange.rs",
        r####"// Preparation-only metadata held beside the original Message. No arm/observe,
// transport, extraction or writer custody capability crosses this boundary.
pub(super) struct OriginalReplyState {
    state: ReplyState,
}
impl OriginalReplyState {
    pub(super) fn prepare(request: &Message, manager: &str, client: &str) -> Result<Self, ()> {
        let state = ReplyState::prepare(request, manager, client).map_err(|_| ())?;
        Ok(Self { state })
    }
    pub(super) fn check(&self, request: &Message, manager: &str, client: &str) -> Result<(), ()> {
        if self.state.phase != Phase::Prepared
            || self.state.reply_open
            || self.state.manager != manager
            || self.state.client != client
            || self.state.serial != request.primary_header().serial_num()
        {
            return Err(());
        }
        request_matches(request, manager, client).map_err(|_| ())
    }
}

"####,
        r####""####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange_tests.rs",
        r####"#[path = "diagnostic_original_reply_binding_tests.rs"]
mod original_reply_tests;

"####,
        r####""####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_original_manager_join.rs",
        r####"    fn require_names(&self, manager: &str, client: &str) -> std::result::Result<(), JoinError>;
"####,
        r####""####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_original_manager_join.rs",
        r####"    fn require_names(&self, manager: &str, client: &str) -> std::result::Result<(), JoinError> {
        require_channel_names(
            self.refused,
            (&self.manager, &self.client),
            (manager, client),
        )
    }
"####,
        r####""####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_original_manager_join.rs",
        r####"fn require_channel_names(
    refused: bool,
    retained: (&str, &str),
    claimed: (&str, &str),
) -> std::result::Result<(), JoinError> {
    if refused
        || retained != claimed
        || !super::unique(retained.0)
        || !super::unique(retained.1)
        || retained.0 == retained.1
    {
        return Err(JoinError::Channel);
    }
    Ok(())
}
"####,
        r####""####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_original_manager_join_tests.rs",
        r####"    fn require_names(&self, manager: &str, client: &str) -> Result<(), JoinError> {
        if (self.case == "names-before" && self.calls == 1)
            || (self.case == "names-after" && self.calls == 2)
        {
            return Err(JoinError::Channel);
        }
        require_channel_names(false, (":1.74", ":1.75"), (manager, client))
    }
"####,
        r####""####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_original_manager_message.rs",
        r####"use super::super::super::dispatch_exchange::OriginalReplyState;
"####,
        r####""####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_original_manager_message.rs",
        r####"    reply: Option<OriginalReplyState>,
"####,
        r####""####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_original_manager_message.rs",
        r####"        let message = self.message.as_ref().ok_or(MessageError::Binding)?;
        self.reply
            .as_ref()
            .ok_or(MessageError::Binding)?
            .check(message, manager, client)
            .map_err(|_| MessageError::Binding)?;
        inspect(
            input,
            message,
"####,
        r####"        inspect(
            input,
            self.message.as_ref().ok_or(MessageError::Binding)?,
"####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_original_manager_message.rs",
        r####"        channel.require_names(manager, client)?;
        frame.observe().map_err(JoinError::from)?;
        let input = inputs(frame)?;
        if payload.message.is_none() {
            if payload.pinned.is_some()
                || payload.serial.is_some()
                || payload.names.is_some()
                || payload.reply.is_some()
            {
                return Err(MessageError::Binding);
            }
            let message = encode(input.legacy, manager, client)?;
            payload.reply = Some(
                OriginalReplyState::prepare(&message, manager, client)
                    .map_err(|_| MessageError::Binding)?,
            );
            payload.pinned = Some(aliases(&message)?);
            payload.serial = Some(message.primary_header().serial_num());
            payload.names = Some((manager.into(), client.into()));
            payload.message = Some(message);
        }
        #[cfg(test)]
        if let Some(fault) = fault.filter(|fault| {
            *fault != "delivery" && !fault.starts_with("inside-") && !fault.starts_with("closing-")
        }) {
"####,
        r####"        frame.observe().map_err(JoinError::from)?;
        let input = inputs(frame)?;
        if payload.message.is_none() {
            if payload.pinned.is_some() || payload.serial.is_some() || payload.names.is_some() {
                return Err(MessageError::Binding);
            }
            let message = encode(input.legacy, manager, client)?;
            payload.pinned = Some(aliases(&message)?);
            payload.serial = Some(message.primary_header().serial_num());
            payload.names = Some((manager.into(), client.into()));
            payload.message = Some(message);
        }
        #[cfg(test)]
        if let Some(fault) =
            fault.filter(|fault| *fault != "delivery" && !fault.starts_with("inside-"))
        {
"####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_original_manager_message.rs",
        r####"        channel.require_names(manager, client)?;
        #[cfg(test)]
        if let Some(case) = fault.and_then(|case| case.strip_prefix("closing-")) {
            payload.tamper(&inputs(frame)?, case);
        }
"####,
        r####""####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_original_manager_message_frame_tests.rs",
        r####"#[path = "diagnostic_original_reply_binding_frame_tests.rs"]
mod reply_binding;
"####,
        r####""####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_original_manager_message_test_api.rs",
        r####"            || case.starts_with("reply-")
            || case.starts_with("closing-reply-")
"####,
        r####""####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_original_manager_message_units.rs",
        r####"            "reply-missing" => self.reply = None,
            "reply-client" => {
                self.reply = Some(
                    OriginalReplyState::prepare(self.message.as_ref().unwrap(), MANAGER, ":1.99")
                        .unwrap(),
                );
            }
            "reply-manager" | "reply-serial" => {
                let message = altered(input, kind.strip_prefix("reply-").unwrap());
                self.reply = Some(
                    OriginalReplyState::prepare(
                        &message,
                        if kind == "reply-manager" {
                            ":1.99"
                        } else {
                            MANAGER
                        },
                        CLIENT,
                    )
                    .unwrap(),
                );
            }
"####,
        r####""####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_original_manager_message_units.rs",
        r####"
#[path = "diagnostic_original_reply_payload_tests.rs"]
mod reply_binding;
"####,
        r####""####,
    ),
];
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let mut clean = two_phase_launch::normalize(path, raw);
    for &(changed, new, old) in EDITS {
        if path == changed {
            assert_eq!(
                clean.matches(new).count(),
                1,
                "original reply structural drift: {path}"
            );
            clean = clean.replacen(new, old, 1);
        }
    }
    clean
}
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    crate::runtime_io::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn git(args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root())
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().replace('\r', "")
}
fn old(path: &str) -> String {
    git(&["show", &format!("{BASE}:{path}")])
}
#[test]
fn original_reply_binding_preserves_every_prior_runtime_and_test_body() {
    let paths = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    let paths: Vec<_> = paths.lines().filter(|p| p.ends_with(".rs")).collect();
    assert_eq!(paths.len(), 359);
    for path in paths {
        assert_eq!(normalize(path, &read(path)), old(path), "{path}");
    }
    let path = "docs/testing/gated-test-registry.toml";
    assert_eq!(normalize(path, &read(path)), old(path));
}
#[test]
fn original_reply_binding_exact_adapter_rejects_mutated_guards_or_registration() {
    for &(path, new, _) in EDITS {
        let raw = read(path);
        let missing = raw.replacen(new, "", 1);
        assert!(
            std::panic::catch_unwind(|| normalize(path, &missing)).is_err(),
            "{path}"
        );
        let changed = raw.replacen(new, &format!("{new}/* unexpected addition */"), 1);
        assert_ne!(normalize(path, &changed), old(path), "{path}");
    }
    let path = "tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange.rs";
    for token in [
        "self.state.phase != Phase::Prepared",
        "self.state.reply_open",
        "self.state.manager != manager",
        "self.state.client != client",
        "self.state.serial != request.primary_header().serial_num()",
        "request_matches(request, manager, client).map_err(|_| ())",
    ] {
        let changed = read(path).replacen(token, &format!("{token} /* mutation */"), 1);
        assert!(
            std::panic::catch_unwind(|| normalize(path, &changed)).is_err(),
            "{token}"
        );
    }
    let path = "tools/long-run-supervisor-074/src/diagnostic_original_manager_message.rs";
    for token in [
        "channel.require_names(manager, client)?;",
        "payload.reply = Some(",
        "self.reply",
        "|| payload.reply.is_some()",
    ] {
        let changed = read(path).replacen(token, &format!("{token} /* mutation */"), 1);
        assert!(
            std::panic::catch_unwind(|| normalize(path, &changed)).is_err(),
            "{token}"
        );
    }
}
#[test]
fn original_reply_binding_does_not_export_transport_or_add_a_full_frame() {
    let path = "tools/long-run-supervisor-074/src/diagnostic_original_manager_message.rs";
    let raw = read(path);
    assert_eq!(
        raw.matches("channel.require_names(manager, client)?;")
            .count(),
        2
    );
    assert_eq!(
        raw.matches("frame.revalidate().map_err(JoinError::from)?;")
            .count(),
        1
    );
    assert_eq!(raw.matches("channel.check(receiver, pin)?;").count(), 2);
    assert!(raw.contains("reply: Option<OriginalReplyState>"));
    assert!(!raw.contains("Message::clone"));
    let wrapper = read("tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange.rs");
    let wrapper = wrapper
        .split("// Preparation-only metadata held beside")
        .nth(1)
        .unwrap()
        .split("#[cfg(test)]")
        .next()
        .unwrap();
    for denied in [
        "fn arm",
        "fn observe",
        "fn lose_reply",
        "pub(crate)",
        "Clone",
        ".send(",
        "Connection",
        "File",
        "OwnedFd",
        "try_clone",
    ] {
        assert!(!wrapper.contains(denied), "{denied}");
    }
    assert!(wrapper.contains("state: ReplyState"));
}
