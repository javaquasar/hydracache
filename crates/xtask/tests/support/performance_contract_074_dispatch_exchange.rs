//! Message mechanics cannot become original execution authority.
const COMPAT: &str = r###"
## Private one shot dispatch exchange mechanics

The private dispatch exchange consumes existing D-Bus version 1 messages and
introduces no new public or durable schema. Client/manager unique-name claims
are data, not authenticated connection or PID 1 evidence. Preparation rejects
well-known routing, malformed envelope/body signature and missing, aliased or
nonblocking/non-CLOEXEC pipe writers. It does not check signed unit settings or
original ancestry; those remain obligations of the future original join.

Arming closes retry before transport entry. A reply must match the original
serial, unique manager sender and client destination, with no returned FDs or
unexpected headers/flags. A canonical nonzero u32 job path is observation data
only; missing/malformed/error/duplicate replies never authorize retry, release,
start or completion. Error bodies are not diagnostics. Message ownership survives
preparation failure and uncertain outcomes. No message/FD export API is provided.
Socket-pair SCM_RIGHTS fixtures carry byte-identical zbus-built messages and
verify real local credentials/FDs; they are not system-bus authentication or
original execution. Unknown shapes fail closed. No ordinary sender is connected.
"###;
const REG: &str = "\n#[path = \"diagnostic_dispatch_exchange.rs\"]\nmod dispatch_exchange;\n";
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let exception = match path {
        "docs/COMPAT.md" => COMPAT,
        "tools/long-run-supervisor-074/src/diagnostic_original_enrollment_frame.rs" => REG,
        _ => return raw.to_owned(),
    };
    assert_eq!(
        raw.matches(exception).count(),
        1,
        "dispatch exception drift"
    );
    raw.replacen(exception, "", 1)
}
#[test]
fn dispatch_exchange_keeps_original_owners_manager_and_kernels_frozen() {
    for name in [
        "diagnostic_original_enrollment_frame.rs",
        "diagnostic_original_enrollment_frame_build.rs",
        "diagnostic_original_enrollment_frame_test_api.rs",
        "diagnostic_original_enrollment_frame_tests.rs",
        "diagnostic_original_enrollment_fault_tests.rs",
        "diagnostic_original_custody_join.rs",
        "diagnostic_original_grouped_preparation.rs",
        "diagnostic_original_pipe_transport.rs",
        "diagnostic_original_barred_custody.rs",
        "diagnostic_original_barred_collector_kernel.rs",
        "diagnostic_manager.rs",
        "systemd_unit.rs",
        "systemd_spawn.rs",
        "diagnostic_autonomous_watchdog.rs",
    ] {
        let path = format!("tools/long-run-supervisor-074/src/{name}");
        let out = std::process::Command::new("git")
            .args(["show", &format!("6af23de5:{path}")])
            .current_dir(super::super::root())
            .output()
            .unwrap();
        assert!(out.status.success());
        assert_eq!(
            normalize(&path, &super::read(&path)),
            String::from_utf8(out.stdout).unwrap().replace('\r', ""),
            "{path}"
        );
    }
}
fn compact(raw: &str) -> String {
    raw.split_whitespace().collect()
}
fn closed(raw: &str) -> bool {
    let s = compact(raw);
    [
        "h.destination().map(|v|v.as_str())!=Some(manager)",
        "h.reply_serial()!=Some(self.serial)",
        "h.sender().map(|v|v.as_str())!=Some(self.manager.as_str())",
        "h.destination().map(|v|v.as_str())!=Some(self.client.as_str())",
        "self.phase!=Phase::Prepared",
        "self.phase=Phase::Uncertain;",
        "letattempted=self.phase==Phase::Uncertain&&self.reply_open;",
        "self.reply_open=false;",
        "self.reply_open=true;",
        "self.phase=Phase::Refused;",
        "!reply.data().fds().is_empty()",
        "h.unix_fds().unwrap_or(0)!=0",
        "h.path().is_some()",
        "h.interface().is_some()",
        "h.member().is_some()",
        "reply.message_type()!=Type::MethodReturn",
        "h.error_name().is_some()",
        "id==0||id.to_string()!=suffix",
        "writer_shape(fds[0].as_raw_fd())?",
        "writer_shape(fds[1].as_raw_fd())?",
        "phase:Phase::Prepared",
    ]
    .iter()
    .all(|needed| s.contains(needed))
}
#[test]
fn dispatch_exchange_retry_and_reply_predicates_have_mutation_canaries() {
    let raw = super::read("tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange.rs");
    assert!(closed(&raw));
    let raw = compact(&raw);
    for needed in [
        "h.reply_serial()!=Some(self.serial)",
        "h.sender().map(|v|v.as_str())!=Some(self.manager.as_str())",
        "h.destination().map(|v|v.as_str())!=Some(self.client.as_str())",
        "self.phase=Phase::Uncertain;",
        "self.phase=Phase::Refused;",
        "letattempted=self.phase==Phase::Uncertain&&self.reply_open;",
        "self.reply_open=false;",
        "!reply.data().fds().is_empty()",
        "id==0||id.to_string()!=suffix",
    ] {
        assert!(!closed(&raw.replace(needed, "")), "{needed}");
    }
    let observation = raw.split("fnobserve(").nth(1).unwrap();
    assert!(
        observation.find("self.phase=Phase::Refused;").unwrap()
            < observation.find("reply.header()").unwrap()
    );
    assert!(!raw.contains("self.phase=Phase::Prepared"));
}
#[test]
fn dispatch_exchange_has_no_sender_or_authority_conversion_and_retains_failures() {
    let raw = super::read("tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange.rs");
    for denied in [
        "pub fn",
        "pub(crate)",
        ".send(",
        "sendmsg(",
        "Connection",
        "Proxy",
        "Deserialize",
        "Serialize",
        "derive(Clone",
        "into_message",
        "as_message",
        "timerfd",
        "clock_gettime",
    ] {
        assert!(!raw.contains(denied), "{denied}");
    }
    assert!(compact(&raw).contains("PreparationFailure{error,request}"));
    assert!(raw.contains("struct JobObservation"));
    assert!(super::read("docs/COMPAT.md").contains(COMPAT));
}
#[test]
fn dispatch_socket_fixtures_are_byte_exact_owned_rights_not_a_system_bus() {
    let raw = super::read(
        "tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange_socket_tests.rs",
    );
    for needed in [
        "UnixDatagram::pair()",
        "libc::SCM_CREDENTIALS",
        "libc::SCM_RIGHTS",
        "libc::MSG_CMSG_CLOEXEC",
        "writer_shape(fd.as_raw_fd())",
        "bytes, data.bytes()",
        "Message::from_bytes(transferred)",
        "Phase::Uncertain",
        "ExchangeError::AlreadyAttempted",
    ] {
        assert!(raw.contains(needed), "{needed}");
    }
    assert!(raw.find("rights.push(").unwrap() < raw.find("assert!(n >= 0)").unwrap());
    assert!(
        raw.find("assert_eq!(bytes, data.bytes())").unwrap()
            < raw.find("Message::from_bytes(transferred)").unwrap()
    );
    for denied in [
        "Connection::system",
        "Command::",
        "SYS_clone3",
        "SIGKILL",
        "systemctl",
    ] {
        assert!(!raw.contains(denied), "{denied}");
    }
}
