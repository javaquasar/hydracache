use super::*;
fn address(serial: NonZeroU32) -> Address<'static> {
    Address {
        manager: ":1.2",
        client: ":1.3",
        serial,
        deadline: child_exchange::clock().unwrap() + 5_000_000_000,
    }
}
fn fixture() -> (Vec<std::fs::File>, HeldRequest) {
    let files = (0..18)
        .map(|_| tempfile::tempfile().unwrap())
        .collect::<Vec<_>>();
    let fds = std::array::from_fn(|i| files[i].as_raw_fd());
    let request = build(
        "hydracache-diagnostic-074-fixture-1.service",
        &[("NoNewPrivileges", UnitProperty::Boolean(true))],
        address(NonZeroU32::new(74).unwrap()),
        fds,
        &fds,
    )
    .unwrap();
    (files, request)
}
fn reply(r: &HeldRequest, manager: &str, client: &str, path: &str) -> Message {
    Message::method_return(&r.message.header())
        .unwrap()
        .sender(manager)
        .unwrap()
        .destination(client)
        .unwrap()
        .build(&OwnedObjectPath::try_from(path).unwrap())
        .unwrap()
}
#[test]
fn held_delivery_wire_exact_eighteen_aliases_names_and_fixed_command() {
    let (_files, r) = fixture();
    r.inspect().unwrap();
    let b = r.message.body();
    let (unit, mode, props, aux): Body<'_> = b.deserialize().unwrap();
    assert_eq!(unit, "hydracache-diagnostic-074-fixture-1.service");
    assert_eq!(mode, "fail");
    assert!(aux.is_empty());
    assert_eq!(props.len(), 8);
    assert_eq!(props[0].0, "NoNewPrivileges");
    assert_eq!(props[5].0, "ExtraFileDescriptors");
    assert_eq!(props[5].1.value_signature().to_string(), "a(hs)");
    assert_eq!(props[6].0, "SetLoginEnvironment");
    assert_eq!(bool::try_from(&props[6].1).unwrap(), false);
    assert_eq!(props[7].0, "UnsetEnvironment");
    assert_eq!(props[7].1.value_signature().to_string(), "as");
    assert!(r.body.windows(ARG.len()).any(|v| v == ARG.as_bytes()));
    assert!(r.body.windows(IMAGE.len()).any(|v| v == IMAGE.as_bytes()));
    assert!(!r.names_match("org.freedesktop.systemd1", ":1.3"));
}
#[test]
fn held_delivery_wire_one_shot_job_is_not_child_ack() {
    let (_files, mut r) = fixture();
    let response = reply(&r, ":1.2", ":1.3", "/org/freedesktop/systemd1/job/74");
    r.arm().unwrap();
    assert_eq!(
        r.observe(&response).unwrap().as_str(),
        "/org/freedesktop/systemd1/job/74"
    );
    r.inspect().unwrap();
    assert_eq!(r.phase, Phase::JobObserved);
    assert!(r.arm().is_err());
    assert!(r.observe(&response).is_err());
}
#[test]
fn held_delivery_wire_lost_ack_and_every_malformed_reply_never_rearm() {
    for (m, c, p) in [
        (":1.4", ":1.3", "/org/freedesktop/systemd1/job/74"),
        (":1.2", ":1.4", "/org/freedesktop/systemd1/job/74"),
        (":1.2", ":1.3", "/org/freedesktop/systemd1/job/0"),
        (":1.2", ":1.3", "/org/freedesktop/systemd1/job/074"),
        (":1.2", ":1.3", "/org/freedesktop/systemd1/job/4294967296"),
        (":1.2", ":1.3", "/foreign/74"),
    ] {
        let (_f, mut r) = fixture();
        let response = reply(&r, m, c, p);
        r.arm().unwrap();
        assert!(r.observe(&response).is_err());
        assert!(r.arm().is_err());
        assert_eq!(r.message.data().fds().len(), 18);
    }
    let (_f, mut lost) = fixture();
    lost.arm().unwrap();
    lost.refuse();
    assert!(lost.arm().is_err());
    assert_eq!(lost.message.data().fds().len(), 18);
}
#[test]
fn held_delivery_wire_mutated_alias_or_original_flags_refuse() {
    let (f, r) = fixture();
    assert_eq!(
        unsafe { libc::fcntl(f[7].as_raw_fd(), libc::F_SETFD, 0) },
        0
    );
    assert!(r.inspect().is_err());
}
#[test]
fn held_delivery_wire_legacy_properties_duplicate_names_and_aliases_refuse() {
    let (files, r) = fixture();
    let fds = std::array::from_fn(|i| files[i].as_raw_fd());
    for key in [
        "ExecStart",
        "SetLoginEnvironment",
        "UnsetEnvironment",
        "ExtraFileDescriptors",
        "StandardInput",
        "StandardOutputFileToAppend",
        "StandardErrorFileDescriptor",
    ] {
        assert!(build(
            "hydracache-diagnostic-074-fixture-1.service",
            &[(key, UnitProperty::Text("foreign".into()))],
            address(r.serial),
            fds,
            &fds
        )
        .is_err());
    }
    let mut aliases = fds;
    aliases[17] = aliases[0];
    assert!(build(
        "hydracache-diagnostic-074-fixture-1.service",
        &[],
        address(r.serial),
        aliases,
        &fds
    )
    .is_err());
    assert!(build(
        "hydracache-diagnostic-074-fixture-1.service",
        &[
            ("User", UnitProperty::Text("x".into())),
            ("User", UnitProperty::Text("x".into()))
        ],
        address(r.serial),
        fds,
        &fds
    )
    .is_err());
}

#[test]
fn held_delivery_wire_absolute_bound_cannot_be_renewed_after_arm_or_reply() {
    let (_f, mut r) = fixture();
    r.deadline = 1;
    assert!(r.arm().is_err());
    assert_eq!(r.phase, Phase::Refused);
    let (_f, mut r) = fixture();
    let response = reply(&r, ":1.2", ":1.3", "/org/freedesktop/systemd1/job/74");
    r.arm().unwrap();
    r.deadline = 1;
    assert!(r.observe(&response).is_err());
    assert!(r.arm().is_err());
}
