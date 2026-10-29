//! Preparation-only wrapper: no transport or original-owner construction.
use super::*;

#[test]
fn original_reply_binding_borrows_without_owning_or_duplicating_writers() {
    let [mut out, out_w] = pipe();
    let [mut err, err_w] = pipe();
    let request = request_kind([&out_w, &err_w], "ok");
    let before: Vec<_> = request
        .data()
        .fds()
        .iter()
        .map(AsRawFd::as_raw_fd)
        .collect();
    let state = OriginalReplyState::prepare(&request, MANAGER, CLIENT).unwrap();
    for _ in 0..3 {
        state.check(&request, MANAGER, CLIENT).unwrap();
    }
    assert_eq!(
        before,
        request
            .data()
            .fds()
            .iter()
            .map(AsRawFd::as_raw_fd)
            .collect::<Vec<_>>()
    );
    drop(out_w);
    drop(err_w);
    drop(request);
    assert_eq!(out.read(&mut [0; 1]).unwrap(), 0);
    assert_eq!(err.read(&mut [0; 1]).unwrap(), 0);
    drop(state);
}

#[test]
fn original_reply_binding_requires_exact_serial_names_and_prepared_closed_state() {
    let held = exchange();
    for mutation in [
        "manager",
        "client",
        "serial",
        "uncertain",
        "observed",
        "refused",
        "open",
    ] {
        let mut wrapper = OriginalReplyState::prepare(&held.request, MANAGER, CLIENT).unwrap();
        match mutation {
            "manager" => wrapper.state.manager = ":1.99".into(),
            "client" => wrapper.state.client = ":1.99".into(),
            "serial" => {
                wrapper.state.serial = NonZeroU32::new(wrapper.state.serial.get() + 1).unwrap()
            }
            "uncertain" => wrapper.state.arm().unwrap(),
            "observed" => wrapper.state.phase = Phase::JobObserved,
            "refused" => wrapper.state.phase = Phase::Refused,
            "open" => wrapper.state.reply_open = true,
            _ => unreachable!(),
        }
        assert!(
            wrapper.check(&held.request, MANAGER, CLIENT).is_err(),
            "{mutation}"
        );
    }
    let wrapper = OriginalReplyState::prepare(&held.request, MANAGER, CLIENT).unwrap();
    assert!(wrapper.check(&held.request, ":1.99", CLIENT).is_err());
    assert!(wrapper.check(&held.request, MANAGER, ":1.99").is_err());
    assert!(wrapper.check(&held.request, MANAGER, MANAGER).is_err());
}

#[test]
fn original_reply_binding_rechecks_request_and_live_descriptor_predicates() {
    let held = exchange();
    let wrapper = OriginalReplyState::prepare(&held.request, MANAGER, CLIENT).unwrap();
    for kind in [
        "well-known",
        "path",
        "member",
        "interface",
        "no-reply",
        "missing-fd",
        "body",
        "sender",
        "extra-fd",
    ] {
        let [_out, out_w] = pipe();
        let [_err, err_w] = pipe();
        let request = request_kind([&out_w, &err_w], kind);
        assert!(
            OriginalReplyState::prepare(&request, MANAGER, CLIENT).is_err(),
            "{kind}"
        );
        assert!(wrapper.check(&request, MANAGER, CLIENT).is_err(), "{kind}");
    }
    let fd = held.request.data().fds()[0].as_raw_fd();
    assert_eq!(unsafe { libc::fcntl(fd, libc::F_SETFD, 0) }, 0);
    assert!(wrapper.check(&held.request, MANAGER, CLIENT).is_err());
}
