//! Exact additive prearmed mechanics; original predicates remain byte-exact.
const BASE: &str = "a44618c2";
const MODULE: &str = r####"
#[path = "diagnostic_prearmed_watchdog.rs"]
mod prearmed;
"####;
const COMPAT: &str = r####"
## Private prearmed guardian enrollment

hc074-prearmed-guardian-v1 uses one ephemeral AF_UNIX SOCK_SEQPACKET pair.
Packets are exactly 16 bytes: HCPG0741, kind byte (1 Pending, 2 Enroll, 3 Armed,
4 Refused, 5 Revoking), then seven zero bytes. Enroll requires one atomic
CLOEXEC pidfd and exactly one kernel SCM_CREDENTIALS record for the creating
PID/UID/GID. Notices require no rights and the guardian's actual credentials.
Only the echoed MSG_CMSG_CLOEXEC flag is allowed. Unknown versions, reserved
bytes, kinds, shapes, extra/truncated rights or credentials refuse. The parent
also retains its creating thread; SCM_CREDENTIALS itself does not prove a TID.

The guardian accepts one live direct child under local procfs inspection,
retains its pidfd before Armed, and treats a second message as revocation.
Revoking means signal attempted, not cessation. No ACK, terminal pidfd or EOF
unlocks the fence. The reader window is the original socketpair lifetime;
there is no reconnect, adoption, downgrade, migration or production reader.
This is not the historical original owner, trusted kernel host context,
whole-cgroup admission, signed launch or all-writer completion proof.

The private hc074-prearmed-owner-test-v1 carrier sends the fixed fixture JSON
schema with creating-owner credentials and one guardian pidfd, optionally a
target pidfd/read pipe and hidden write alias (at most four rights). A one-byte
D without rights marks a completed harness command. Only the owning nonroot
subreaper understands it during its temporary-directory lifetime; it grants no
production identity or authority. The old v0 barrier and signed v2 requirements
remain byte-identical.
"####;
const REGISTRY: &str = r####"
# BEGIN PREARMED WATCHDOG GATE

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.prearmed-watchdog-owner-loss"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_prearmed_watchdog_fixture.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "prearmed_owner_victim"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual nonroot prearmed guardian and target faults; no original admission"
tier = "manual"
required_features = []
required_env = ["HC074_PREARMED_TEST_BINARY"]
required_tools = ["python3"]
timeout_seconds = 180
owner_release = "0.74.0"
ship_mandatory = false
artifacts = []

[gate.ci]
workflow = ".github/workflows/ci.yml"
job = "gated-proof-registry"
step = "Run registered gated proofs"

[gate.command]
program = "python3"
args = ["tools/long-run-supervisor-074/tests/prearmed_watchdog.py","--binary-env","HC074_PREARMED_TEST_BINARY"]
cwd = "."
platform = "linux"

[gate.command.env]

# END PREARMED WATCHDOG GATE
"####;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let addition = match path {
        "tools/long-run-supervisor-074/src/diagnostic_held_barrier.rs" => MODULE,
        "docs/COMPAT.md" => COMPAT,
        "docs/testing/gated-test-registry.toml" => REGISTRY,
        _ => return raw.to_owned(),
    };
    assert_eq!(
        raw.matches(addition).count(),
        1,
        "prearmed structural drift: {path}"
    );
    raw.replacen(addition, "", 1)
}
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path))
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
#[test]
fn prearmed_preserves_all_previous_sources_protocols_and_guards() {
    let paths = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    let paths: Vec<_> = paths.lines().filter(|p| p.ends_with(".rs")).collect();
    assert_eq!(paths.len(), 366);
    for path in paths.into_iter().chain([
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "tools/long-run-supervisor-074/tests/original_custody_owner_loss.py",
    ]) {
        assert_eq!(
            normalize(path, &read(path)),
            git(&["show", &format!("{BASE}:{path}")]),
            "{path}"
        );
    }
}
#[test]
fn prearmed_exact_adapter_rejects_registration_and_old_predicate_mutations() {
    for (path, addition) in [
        (
            "tools/long-run-supervisor-074/src/diagnostic_held_barrier.rs",
            MODULE,
        ),
        ("docs/COMPAT.md", COMPAT),
        ("docs/testing/gated-test-registry.toml", REGISTRY),
    ] {
        let raw = read(path);
        assert!(
            std::panic::catch_unwind(|| normalize(path, &raw.replacen(addition, "", 1))).is_err()
        );
        assert!(std::panic::catch_unwind(|| normalize(path, &format!("{raw}{addition}"))).is_err());
        let changed = raw.replacen(addition, &format!("{addition}/* mutation */"), 1);
        assert_ne!(
            normalize(path, &changed),
            git(&["show", &format!("{BASE}:{path}")])
        );
    }
    let path = "tools/long-run-supervisor-074/src/diagnostic_original_manager_message.rs";
    let raw = read(path);
    for token in [
        "frame.revalidate().map_err(JoinError::from)?;",
        "channel.check(receiver, pin)?;",
    ] {
        assert!(raw.contains(token));
        assert_ne!(
            normalize(
                path,
                &raw.replacen(token, &format!("{token} /* mutation */"), 1)
            ),
            git(&["show", &format!("{BASE}:{path}")])
        );
    }
}
#[test]
fn prearmed_mechanics_do_not_promote_local_owner_to_original_authority() {
    let code = read("tools/long-run-supervisor-074/src/diagnostic_prearmed_watchdog.rs");
    for forbidden in [
        "pub ",
        "LOCK_UN",
        "StartTransientUnit",
        "OriginalManagerMessage",
        "tokio::",
        "Command::",
    ] {
        assert!(!code.contains(forbidden), "{forbidden}");
    }
    for required in [
        "libc::CLONE_PIDFD",
        "libc::SO_PASSCRED",
        "libc::SCM_CREDENTIALS",
        "libc::MSG_CMSG_CLOEXEC",
        "target = Some(fd)",
        "Origin::current() != self.origin",
    ] {
        assert!(code.contains(required), "{required}");
    }
    assert!(code.find("target = Some(fd)").unwrap() < code.find("send(peer, ARMED").unwrap());
    assert!(code.find("self.consumed = true;").unwrap() < code.find("!send(self.peer").unwrap());
    let contract: toml::Value = toml::from_str(&read(
        "docs/testing/performance/0.74/prearmed-watchdog-contract.toml",
    ))
    .unwrap();
    for key in [
        "pending_before_target",
        "target_retained_before_ack",
        "enrollment_at_most_once",
        "no_fence_unlock_path",
    ] {
        assert_eq!(contract[key].as_bool(), Some(true), "{key}");
    }
    for key in [
        "historical_original_owner_authenticated",
        "trusted_original_kernel_context",
        "whole_cgroup_revocation",
        "all_external_writers_revoked",
        "guardian_death_fence_guarantee",
        "production_constructor_enrolled",
        "server_operation_allowed",
        "qualification_allowed",
    ] {
        assert_eq!(contract[key].as_bool(), Some(false), "{key}");
    }
}
