//! Exact structural refactor only; no change to a request or reply predicate.
#[path = "performance_contract_074_original_reply_binding.rs"]
mod original_reply_binding;
const BASE: &str = "926a3b6f";
const PATH: &str = "tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange.rs";
const TESTS: &str = "tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange_tests.rs";
const REG: &str =
    "\n#[path = \"diagnostic_dispatch_reply_state_tests.rs\"]\nmod reply_state_tests;\n";
const EDITS: &[(&str, &str)] = &[
    (
        r###"struct DispatchExchange {
    request: Message,
    state: ReplyState,
}
// Correlation data only: no Message, file descriptor or original-owner capability.
struct ReplyState {
    manager: String,
    client: String,
    serial: NonZeroU32,
    phase: Phase,
    reply_open: bool,
}
"###,
        r###"struct DispatchExchange {
    request: Message,
    manager: String,
    client: String,
    serial: NonZeroU32,
    phase: Phase,
    reply_open: bool,
}
"###,
    ),
    (
        r###"impl std::ops::Deref for DispatchExchange {
    type Target = ReplyState;
    fn deref(&self) -> &ReplyState {
        &self.state
    }
}
impl std::ops::DerefMut for DispatchExchange {
    fn deref_mut(&mut self) -> &mut ReplyState {
        &mut self.state
    }
}
impl ReplyState {
    fn prepare(request: &Message, manager: &str, client: &str) -> Result<Self, ExchangeError> {
        request_matches(request, manager, client)?;
        let serial = request.primary_header().serial_num();
        Ok(Self {
            manager: manager.into(),
            client: client.into(),
            serial,
            phase: Phase::Prepared,
            reply_open: false,
        })
    }
"###,
        r###"impl DispatchExchange {
    fn prepare(request: Message, manager: &str, client: &str) -> Result<Self, PreparationFailure> {
        if let Err(error) = request_matches(&request, manager, client) {
            return Err(PreparationFailure { error, request });
        }
        let serial = request.primary_header().serial_num();
        Ok(Self {
            request,
            manager: manager.into(),
            client: client.into(),
            serial,
            phase: Phase::Prepared,
            reply_open: false,
        })
    }
"###,
    ),
    (
        r###"
impl DispatchExchange {
    fn prepare(request: Message, manager: &str, client: &str) -> Result<Self, PreparationFailure> {
        let state = match ReplyState::prepare(&request, manager, client) {
            Ok(state) => state,
            Err(error) => return Err(PreparationFailure { error, request }),
        };
        Ok(Self { request, state })
    }
    fn arm(&mut self) -> Result<(), ExchangeError> {
        self.state.arm()
    }
    fn lose_reply(&mut self) {
        self.state.lose_reply();
    }
    fn observe(&mut self, reply: &Message) -> Result<JobObservation, ExchangeError> {
        self.state.observe(reply)
    }
}
"###,
        r###""###,
    ),
];
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = original_reply_binding::normalize(path, raw);
    let raw = current.as_str();
    if path == TESTS {
        assert_eq!(
            raw.matches(REG).count(),
            1,
            "reply state test registration drift"
        );
        return raw.replacen(REG, "", 1);
    }
    if path != PATH {
        return raw.to_owned();
    }
    let mut clean = raw.to_owned();
    for &(new, old) in EDITS {
        assert_eq!(
            clean.matches(new).count(),
            1,
            "reply state structural drift"
        );
        clean = clean.replacen(new, old, 1);
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
fn reply_state_refactor_preserves_every_old_runtime_and_test_body() {
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
    assert_eq!(paths.len(), 358);
    for path in paths {
        assert_eq!(normalize(path, &read(path)), old(path), "{path}");
    }
}
#[test]
fn reply_state_transition_and_classifier_block_is_the_original_verbatim() {
    let raw = read(PATH);
    let original = old(PATH);
    let marker = "    // Call before any possible transport side effect.";
    let before =
        &original[original.find(marker).unwrap()..original.rfind("\n}\n\n#[cfg(test)]").unwrap()];
    let after =
        &raw[raw.find(marker).unwrap()..raw.find("\n}\n\nimpl DispatchExchange {").unwrap()];
    assert_eq!(after, before);
    assert_eq!(raw.matches("fn observe(").count(), 2);
    assert!(EDITS[2].0.contains("self.state.observe(reply)"));
    assert!(EDITS[2].0.contains("self.state.arm()"));
    assert!(EDITS[2].0.contains("self.state.lose_reply();"));
    let fields = EDITS[0].0.split("struct ReplyState {").nth(1).unwrap();
    for denied in ["Message", "File", "Fd", "Connection"] {
        assert!(!fields.contains(denied));
    }
}
#[test]
fn reply_state_adapter_cannot_hide_changed_predicates_or_delegation() {
    let raw = read(PATH);
    for (new, _) in EDITS {
        let changed = raw.replacen(new, &format!("{new}/* structural mutation */"), 1);
        assert_ne!(normalize(PATH, &changed), old(PATH));
        let missing = raw.replacen(new, "", 1);
        assert!(std::panic::catch_unwind(|| normalize(PATH, &missing)).is_err());
    }
    for token in [
        "request_matches(request, manager, client)?;",
        "self.state.observe(reply)",
        "self.state.arm()",
        "self.state.lose_reply();",
    ] {
        let changed = raw.replacen(token, &format!("{token} /* mutation */"), 1);
        assert!(std::panic::catch_unwind(|| normalize(PATH, &changed)).is_err());
    }
    for token in [
        "h.reply_serial() != Some(self.serial)",
        "self.phase = Phase::Refused;",
        "h.destination().map(|v| v.as_str()) != Some(manager)",
    ] {
        let changed = raw.replacen(token, &format!("{token} /* predicate mutation */"), 1);
        assert_ne!(normalize(PATH, &changed), old(PATH));
    }
    let raw = read(TESTS);
    assert!(std::panic::catch_unwind(|| normalize(TESTS, &raw.replacen(REG, "", 1))).is_err());
}
