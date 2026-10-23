#![allow(dead_code)] // Exact additive views for preceding frozen modes.
pub const MODE: &str = "\n#[path = \"../diagnostic_bootstrap_held_mode.rs\"]\nmod held_mode;\n";
pub const DISPATCH: &str = "    if held_mode::requested() {\n        if let Err(error) = held_mode::run() {\n            eprintln!(\"Held bootstrap stopped: {error}; no worker release or executable handoff.\");\n        }\n        std::process::exit(2);\n    }\n";
pub const CHILD: &str =
    "\n#[path = \"diagnostic_bootstrap_held_child.rs\"]\npub(crate) mod held;\n";
pub const CONTROLS: &str = "\n#[path = \"bootstrap_held_entry_controls.rs\"]\nmod held_controls;\n";
pub const BLOCK: &str = r#"
<!-- BEGIN HELD CHILD ENTRY V1 -->
## Held child readiness entry v1

--systemd-bootstrap-held-readiness-v1 now selects a separate readiness-only
entry. This supersedes only the earlier candidate's unactivated-argument
statement. Old child modes, HCBT0741/HCBI0741 and HCHR0741/HCHA0741 stay unchanged.
FD0..16 keep all old role predicates in their original order. A separate bounded
closed census accepts FD0..17 and its own enumeration FD, with 21 entries and at
most 22 reads. FD17 must be a distinct nonblocking Unix seqpacket endpoint with
PASSCRED and PASSPIDFD. Only this new role changes inherited FD flags from zero
to CLOEXEC before ownership; no old role is repaired or remapped by the reader.

Activation accepts exactly canonical current LISTEN_PID, LISTEN_FDS=15, the fixed
fifteen names, nonzero lowercase 32-hex INVOCATION_ID and current SYSTEMD_EXEC_PID.
The parent sets SetLoginEnvironment=false and a finite UnsetEnvironment list for
login/path/locale defaults. Unknown variables are refused, not silently removed
by the child. Unsupported manager properties refuse without fallback.

The same observed entry and actual PID derive expectations. Only the nonce is
learned from the kernel-authenticated challenge; full role frames run before
receive, after receive and after the sole possible ACK. Original creator and
guardian pidfds, absolute alarm and readiness channel bound waiting before and
after ACK. Any event, malformed packet, context drift, disconnect or timeout
terminates readiness without retry, worker release, exec, completion or fence
release. All original live checks and admission origins/windows remain intact.

Readiness authenticates the message peer and consistency with the observed
assertions, not production-policy/root enrollment. A matching owned synthetic
fixture may ACK and hold, but cannot execute a worker. Positive original-manager
delivery, complete loaded-unit hardening and all-writer cessation require separate
integration gates. This entry does not admit installation or server execution.
<!-- END HELD CHILD ENTRY V1 -->
"#;
fn remove(raw: &str, addition: &str) -> String {
    assert_eq!(raw.matches(addition).count(), 1);
    raw.replacen(addition, "", 1)
}
pub fn source_view(name: &str, raw: &str) -> String {
    match name {
        "bin/diagnostic_held_bootstrap.rs" => remove(&remove(raw, MODE), DISPATCH),
        "diagnostic_bootstrap_installed_child.rs" => remove(raw, CHILD),
        "bootstrap_installed_controls.rs" => remove(raw, CONTROLS),
        _ => raw.into(),
    }
}
pub fn compat_view(raw: &str) -> String {
    remove(raw, BLOCK)
}
