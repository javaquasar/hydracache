//! The interoperability exception cannot relax original requests or authority.
#[path = "performance_contract_074_original_manager_join.rs"]
mod original_manager_join;
#[path = "performance_contract_074_original_manager_message.rs"]
mod original_manager_message;
const OLD: &str = "            || !reply.primary_header().flags().is_empty()\n";
const NEW: &str = "            || reply.primary_header().flags().bits()\n                & !(zbus::message::Flags::NoReplyExpected as u8)\n                != 0\n            || (reply.message_type() != Type::MethodReturn\n                && !reply.primary_header().flags().is_empty())\n";
const REG: &str =
    "\n#[path = \"diagnostic_dispatch_reply_flags_tests.rs\"]\nmod reply_flags_tests;\n";
const COMPAT: &str = r###"
## Private dispatch method return interoperability

The one-shot private dispatch exchange now accepts METHOD_RETURN flags 0 or
NO_REPLY_EXPECTED only. D-Bus assigns that bit no reply semantics. All other
known or unknown bits refuse; request flags and ERROR classification stay exact.
Original serial/sender/destination, size/body/FD checks and refusal-before-parse
remain unchanged. A permitted reply remains job-path data, not start, cessation,
completion, retry or release authority. No public/durable format or sender is added.
This supersedes only the old exchange's empty METHOD_RETURN flag predicate;
the fixed read-only manager producer and every execution/admission limit stay exact.
"###;

pub(super) fn normalize(path: &str, raw: &str) -> String {
    let grouped = original_manager_message::normalize(path, raw);
    let normalized = original_manager_join::normalize(path, &grouped);
    let raw = normalized.as_str();
    let (changed, restored) = match path {
        "tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange.rs" => (NEW, OLD),
        "tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange_tests.rs" => (REG, ""),
        "docs/COMPAT.md" => (COMPAT, ""),
        _ => return raw.to_owned(),
    };
    assert_eq!(
        raw.matches(changed).count(),
        1,
        "reply flag exception drift: {path}"
    );
    raw.replacen(changed, restored, 1)
}
fn compact(raw: &str) -> String {
    raw.split_whitespace().collect()
}
fn flags_closed(raw: &str) -> bool {
    let raw = compact(raw);
    raw.contains(&compact(NEW))
        && raw.contains("!message.primary_header().flags().is_empty()")
        && raw.contains("letattempted=self.phase==Phase::Uncertain&&self.reply_open;")
        && raw.contains("self.phase=Phase::Refused;")
}
#[test]
fn dispatch_reply_flag_exception_is_exactly_one_original_predicate() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange.rs",
        "tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange_tests.rs",
    ] {
        let raw = super::read(path);
        let old = std::process::Command::new("git")
            .args(["show", &format!("ff665c3b:{path}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(old.status.success());
        assert_eq!(
            normalize(path, &raw),
            String::from_utf8(old.stdout).unwrap().replace('\r', "")
        );
    }
    assert!(super::read("docs/COMPAT.md").contains(COMPAT));
}
#[test]
fn dispatch_reply_flag_mask_error_request_and_sticky_guards_have_canaries() {
    let raw = super::read("tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange.rs");
    assert!(flags_closed(&raw));
    let raw = compact(&raw);
    for needed in [
        "reply.primary_header().flags().bits()&!(zbus::message::Flags::NoReplyExpectedasu8)!=0",
        "reply.message_type()!=Type::MethodReturn&&!reply.primary_header().flags().is_empty()",
        "!message.primary_header().flags().is_empty()",
        "letattempted=self.phase==Phase::Uncertain&&self.reply_open;",
    ] {
        assert!(!flags_closed(&raw.replace(needed, "")), "{needed}");
    }
    let corpus =
        super::read("tools/long-run-supervisor-074/src/diagnostic_dispatch_reply_flags_tests.rs");
    for needed in [
        "for bits in 0..8",
        "for bits in 1..8",
        "for bits in [0, 1]",
        "held.lose_reply()",
        "ExchangeError::ManagerError",
        "failure.request.data().fds().len()",
    ] {
        assert!(corpus.contains(needed), "{needed}");
    }
}
