//! The real payload checks correlation without exporting its original Message.
use super::*;
fn held(input: &Inputs<'_>) -> Payload {
    let message = encode(input.legacy, MANAGER, CLIENT).unwrap();
    Payload {
        reply: Some(OriginalReplyState::prepare(&message, MANAGER, CLIENT).unwrap()),
        pinned: Some(aliases(&message).unwrap()),
        serial: Some(message.primary_header().serial_num()),
        names: Some((MANAGER.into(), CLIENT.into())),
        message: Some(message),
        refused: false,
    }
}
#[test]
fn original_reply_payload_repeated_checks_retain_exact_request_and_writers() {
    let (spec, out, err) = fixture();
    let legacy = canonical::message(&spec, &out[1], &err[1]).unwrap();
    let input = Inputs {
        legacy: &legacy,
        spec,
        writers: [&out[1], &err[1]],
    };
    let mut payload = held(&input);
    let pinned = payload.pinned;
    for _ in 0..3 {
        payload.check(&input, MANAGER, CLIENT).unwrap();
        assert_eq!(payload.pinned, pinned);
        assert_eq!(
            aliases(payload.message.as_ref().unwrap()).unwrap(),
            pinned.unwrap()
        );
    }
    drop(payload);
    for fd in pinned.unwrap() {
        assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::EBADF)
        );
    }
    canonical::inspect_message(&legacy, &input.spec, &out[1], &err[1]).unwrap();
}
#[test]
fn original_reply_payload_refuses_missing_foreign_and_inconsistent_correlation() {
    let (spec, out, err) = fixture();
    let legacy = canonical::message(&spec, &out[1], &err[1]).unwrap();
    let input = Inputs {
        legacy: &legacy,
        spec,
        writers: [&out[1], &err[1]],
    };
    for case in [
        "reply-missing",
        "reply-manager",
        "reply-client",
        "reply-serial",
    ] {
        let mut payload = held(&input);
        let pinned = payload.pinned;
        payload.tamper(&input, case);
        assert!(
            matches!(
                payload.check(&input, MANAGER, CLIENT),
                Err(MessageError::Binding)
            ),
            "{case}"
        );
        assert_eq!(payload.pinned, pinned);
        assert_eq!(
            aliases(payload.message.as_ref().unwrap()).unwrap(),
            pinned.unwrap()
        );
        for fd in pinned.unwrap() {
            assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, libc::FD_CLOEXEC);
        }
        payload.refused = true;
        assert!(matches!(
            payload.check(&input, MANAGER, CLIENT),
            Err(MessageError::Refused)
        ));
    }
}
#[test]
fn original_reply_channel_names_require_exact_unpoisoned_retained_binding() {
    let check = super::super::super::require_channel_names;
    check(false, (MANAGER, CLIENT), (MANAGER, CLIENT)).unwrap();
    for (refused, retained, claimed) in [
        (true, (MANAGER, CLIENT), (MANAGER, CLIENT)),
        (false, (MANAGER, CLIENT), (":1.99", CLIENT)),
        (false, (MANAGER, CLIENT), (MANAGER, ":1.99")),
        (false, (MANAGER, MANAGER), (MANAGER, MANAGER)),
        (
            false,
            ("org.freedesktop.systemd1", CLIENT),
            ("org.freedesktop.systemd1", CLIENT),
        ),
        (false, (MANAGER, "malformed"), (MANAGER, "malformed")),
    ] {
        assert!(check(refused, retained, claimed).is_err());
    }
}
