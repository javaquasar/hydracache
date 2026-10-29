use super::*;
use std::fs::File;
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use zbus::wire::{Fd, Variant as Value};
use zbus6 as zbus;
const MANAGER: &str = ":1.74";
const CLIENT: &str = ":1.75";

#[path = "diagnostic_dispatch_exchange_socket_tests.rs"]
mod socket_tests;

#[path = "diagnostic_dispatch_reply_flags_tests.rs"]
mod reply_flags_tests;

#[path = "diagnostic_dispatch_negotiated_tests.rs"]
mod negotiated_tests;

#[path = "diagnostic_dispatch_reply_state_tests.rs"]
mod reply_state_tests;

#[path = "diagnostic_original_reply_binding_tests.rs"]
mod original_reply_tests;

fn pipe() -> [File; 2] {
    let mut fds = [-1; 2];
    assert_eq!(unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) }, 0);
    assert_eq!(
        unsafe { libc::fcntl(fds[0], libc::F_SETFL, libc::O_NONBLOCK) },
        0
    );
    unsafe { [File::from_raw_fd(fds[0]), File::from_raw_fd(fds[1])] }
}
fn request_kind(writers: [&File; 2], kind: &str) -> Message {
    let mut builder = Message::method_call(
        if kind == "path" { "/foreign" } else { PATH },
        if kind == "member" { "StopUnit" } else { METHOD },
    )
    .destination(if kind == "well-known" {
        "org.freedesktop.systemd1"
    } else {
        MANAGER
    })
    .interface(if kind == "interface" {
        "org.example.Foreign"
    } else {
        INTERFACE
    });
    if kind == "sender" {
        builder = builder.sender(CLIENT);
    }
    if kind == "reply-serial" {
        builder = builder.reply_serial(Some(std::num::NonZeroU32::new(7).unwrap()));
    }
    for (name, flag) in [
        ("no-reply", zbus::message::Flags::NO_REPLY_EXPECTED),
        ("no-auto", zbus::message::Flags::NO_AUTO_START),
        ("interactive", zbus::message::Flags::ALLOW_INTERACTIVE_AUTH),
    ] {
        if kind == name {
            builder = builder.with_flags(flag);
        }
    }
    let mut props = vec![(
        "StandardOutputFileDescriptor",
        Value::from(Fd::from(writers[0])),
    )];
    if kind != "missing-fd" {
        props.push((
            "StandardErrorFileDescriptor",
            Value::from(Fd::from(writers[1])),
        ));
    }
    let third = tempfile::tempfile().unwrap();
    if kind == "extra-fd" {
        props.push(("UnexpectedDescriptor", Value::from(Fd::from(&third))));
    }
    if kind == "body" {
        return builder
            .build(&(Fd::from(writers[0]), Fd::from(writers[1])))
            .unwrap();
    }
    let auxiliary: Vec<(&str, Vec<(&str, Value<'_>)>)> = vec![];
    builder
        .build(&(
            if kind == "oversize" {
                "x".repeat(65_537)
            } else {
                "hydracache-diagnostic-074-fixture-1.service".into()
            },
            "fail",
            props,
            auxiliary,
        ))
        .unwrap()
}
fn exchange() -> DispatchExchange {
    let [_out_r, out_w] = pipe();
    let [_err_r, err_w] = pipe();
    DispatchExchange::prepare(request_kind([&out_w, &err_w], "ok"), MANAGER, CLIENT).unwrap()
}
fn reply(exchange: &DispatchExchange, path: &str) -> Message {
    Message::method_return(&exchange.request.header())
        .sender(MANAGER)
        .destination(CLIENT)
        .build(&OwnedObjectPath::try_from(path).unwrap())
        .unwrap()
}
#[test]
fn dispatch_request_predicates_fail_without_releasing_owned_message() {
    let seed = 0x0740_d518_u64;
    for kind in [
        "path",
        "member",
        "interface",
        "well-known",
        "sender",
        "reply-serial",
        "no-reply",
        "no-auto",
        "interactive",
        "missing-fd",
        "extra-fd",
        "oversize",
        "body",
    ] {
        let [_a, a] = pipe();
        let [_b, b] = pipe();
        let failure =
            DispatchExchange::prepare(request_kind([&a, &b], kind), MANAGER, CLIENT).unwrap_err();
        assert_eq!(
            failure.error,
            ExchangeError::Request,
            "seed={seed:#x} kind={kind}"
        );
        assert!(!failure.request.data().fds().is_empty());
    }
    for error in [false, true] {
        let original = exchange();
        let [_a, a] = pipe();
        let [_b, b] = pipe();
        let builder = if error {
            Message::error(&original.request.header(), "org.example.Refused")
        } else {
            Message::method_return(&original.request.header())
        };
        let wrong_type = builder.build(&(Fd::from(&a), Fd::from(&b))).unwrap();
        assert!(DispatchExchange::prepare(wrong_type, MANAGER, CLIENT).is_err());
    }
}
#[test]
fn dispatch_name_claims_are_distinct_bounded_data_not_manager_authentication() {
    for (manager, client) in [
        ("org.freedesktop.systemd1", CLIENT),
        (MANAGER, "org.example.Client"),
        (MANAGER, MANAGER),
        (":1", CLIENT),
        (":1.2.3", CLIENT),
        (":1.x", CLIENT),
        (":.1", CLIENT),
        (":1.", CLIENT),
        ("", CLIENT),
    ] {
        let [_a, a] = pipe();
        let [_b, b] = pipe();
        assert!(DispatchExchange::prepare(request_kind([&a, &b], "ok"), manager, client).is_err());
    }
    let name = format!(":1.{}", "7".repeat(64));
    assert!(!unique_name(&name));
}
#[test]
fn dispatch_writer_shape_and_aliases_are_checked() {
    let [read, write] = pipe();
    let [_other, writer] = pipe();
    let alias = write.try_clone().unwrap();
    let request = request_kind([&write, &alias], "ok");
    assert_eq!(request.data().fds().len(), 2);
    assert!(DispatchExchange::prepare(request, MANAGER, CLIENT).is_err());
    for writers in [[&write, &write], [&read, &writer]] {
        assert!(DispatchExchange::prepare(request_kind(writers, "ok"), MANAGER, CLIENT).is_err());
    }
    assert_eq!(
        unsafe { libc::fcntl(write.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) },
        0
    );
    assert!(
        DispatchExchange::prepare(request_kind([&write, &writer], "ok"), MANAGER, CLIENT).is_err()
    );
    assert!(writer_shape(-1).is_err());
    let [_a, a] = pipe();
    let [_b, b] = pipe();
    let request = request_kind([&a, &b], "ok");
    assert_eq!(
        unsafe { libc::fcntl(request.data().fds()[0].as_raw_fd(), libc::F_SETFD, 0) },
        0
    );
    assert!(DispatchExchange::prepare(request, MANAGER, CLIENT).is_err());
}
#[test]
fn dispatch_arms_uncertain_before_transport_and_never_rearms() {
    let mut exchange = exchange();
    assert_eq!(exchange.phase, Phase::Prepared);
    exchange.arm().unwrap();
    assert_eq!(exchange.phase, Phase::Uncertain);
    assert_eq!(exchange.arm(), Err(ExchangeError::AlreadyAttempted));
    let good = reply(&exchange, "/org/freedesktop/systemd1/job/1");
    assert_eq!(exchange.observe(&good).unwrap_err(), ExchangeError::Reply);
}
#[test]
fn dispatch_reply_before_attempt_latches_refusal() {
    let mut exchange = exchange();
    let good = reply(&exchange, "/org/freedesktop/systemd1/job/1");
    assert_eq!(exchange.observe(&good).unwrap_err(), ExchangeError::Reply);
    assert_eq!(exchange.phase, Phase::Refused);
    assert_eq!(exchange.arm(), Err(ExchangeError::AlreadyAttempted));
}
#[test]
fn dispatch_delayed_reply_cannot_heal_a_closed_unknown_outcome() {
    let mut held = exchange();
    let good = reply(&held, "/org/freedesktop/systemd1/job/74");
    held.arm().unwrap();
    held.lose_reply();
    assert_eq!(held.phase, Phase::Uncertain);
    assert_eq!(held.observe(&good).unwrap_err(), ExchangeError::Reply);
    assert_eq!(held.arm(), Err(ExchangeError::AlreadyAttempted));
    let mut early = exchange();
    early.lose_reply();
    assert_eq!(early.phase, Phase::Refused);
    assert_eq!(early.arm(), Err(ExchangeError::AlreadyAttempted));
}
#[test]
fn dispatch_job_observation_is_one_shot_data_not_start_or_completion() {
    for id in [1, u32::MAX] {
        let mut exchange = exchange();
        let good = reply(&exchange, &format!("/org/freedesktop/systemd1/job/{id}"));
        exchange.arm().unwrap();
        let observed = exchange.observe(&good).unwrap();
        assert_eq!(
            observed.path.as_str(),
            format!("/org/freedesktop/systemd1/job/{id}")
        );
        assert_eq!(exchange.phase, Phase::JobObserved);
        assert_eq!(exchange.observe(&good).unwrap_err(), ExchangeError::Reply);
        assert_eq!(exchange.arm(), Err(ExchangeError::AlreadyAttempted));
    }
}
#[test]
fn dispatch_job_id_grammar_refuses_noncanonical_or_foreign_paths() {
    for path in [
        "/org/freedesktop/systemd1/job/0",
        "/org/freedesktop/systemd1/job/01",
        "/org/freedesktop/systemd1/job/4294967296",
        "/org/freedesktop/systemd1/job",
        "/org/freedesktop/systemd1/job/1/extra",
        "/org/freedesktop/systemd1/unit/1",
    ] {
        let mut exchange = exchange();
        let bad = reply(&exchange, path);
        exchange.arm().unwrap();
        assert_eq!(
            exchange.observe(&bad).unwrap_err(),
            ExchangeError::Reply,
            "{path}"
        );
        assert_eq!(exchange.phase, Phase::Refused);
    }
}
#[test]
fn dispatch_reply_envelope_body_and_fd_predicates_are_exact() {
    for kind in [
        "sender",
        "no-sender",
        "destination",
        "no-destination",
        "serial",
        "no-serial",
        "path",
        "interface",
        "member",
        "flags",
        "string",
        "extra-body",
        "fd",
        "empty",
        "oversize",
    ] {
        let mut exchange = exchange();
        let mut builder = Message::method_return(&exchange.request.header());
        if kind != "no-sender" {
            builder = builder.sender(if kind == "sender" { ":1.99" } else { MANAGER });
        }
        if kind != "no-destination" {
            builder = builder.destination(if kind == "destination" {
                ":1.98"
            } else {
                CLIENT
            });
        }
        if kind == "serial" {
            builder = builder.reply_serial(Some(
                std::num::NonZeroU32::new(
                    exchange
                        .request
                        .primary_header()
                        .serial_num()
                        .get()
                        .wrapping_add(1)
                        .max(1),
                )
                .unwrap(),
            ));
        }
        if kind == "no-serial" {
            builder = builder.reply_serial(None);
        }
        if kind == "path" {
            builder = builder.path("/foreign");
        }
        if kind == "interface" {
            builder = builder.interface("org.example.Foreign");
        }
        if kind == "member" {
            builder = builder.member("Foreign");
        }
        if kind == "flags" {
            builder = builder.with_flags(zbus::message::Flags::NO_AUTO_START);
        }
        let path = OwnedObjectPath::try_from("/org/freedesktop/systemd1/job/1").unwrap();
        let file = tempfile::tempfile().unwrap();
        let bad = match kind {
            "string" => builder.build(&path.as_str()),
            "extra-body" => builder.build(&(path.as_ref(), 7u32)),
            "fd" => builder.build(&(path.as_ref(), Fd::from(&file))),
            "empty" => builder.build(&()),
            "oversize" => builder.build(&"x".repeat(65_537)),
            _ => builder.build(&path),
        }
        .unwrap();
        exchange.arm().unwrap();
        assert_eq!(
            exchange.observe(&bad).unwrap_err(),
            ExchangeError::Reply,
            "{kind}"
        );
        let good = reply(&exchange, "/org/freedesktop/systemd1/job/1");
        assert_eq!(
            exchange.observe(&good).unwrap_err(),
            ExchangeError::Reply,
            "sticky {kind}"
        );
    }
}
#[test]
fn dispatch_manager_error_is_retained_without_exposing_its_payload_or_retry() {
    let mut exchange = exchange();
    let error = Message::error(
        &exchange.request.header(),
        "org.freedesktop.systemd1.UnitExists",
    )
    .sender(MANAGER)
    .destination(CLIENT)
    .build(&"sensitive-manager-detail")
    .unwrap();
    exchange.arm().unwrap();
    let error = exchange.observe(&error).unwrap_err();
    assert_eq!(error, ExchangeError::ManagerError);
    assert!(!format!("{error:?}").contains("sensitive"));
    assert_eq!(exchange.arm(), Err(ExchangeError::AlreadyAttempted));
}
#[test]
fn dispatch_failure_and_uncertain_drop_retain_pipe_writers_until_owner_destruction() {
    for kind in ["ok", "well-known"] {
        let [mut out, out_w] = pipe();
        let [mut err, err_w] = pipe();
        let request = request_kind([&out_w, &err_w], kind);
        drop(out_w);
        drop(err_w);
        let held = DispatchExchange::prepare(request, MANAGER, CLIENT);
        assert_eq!(
            out.read(&mut [0u8; 1]).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        assert_eq!(
            err.read(&mut [0u8; 1]).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        drop(held);
        assert_eq!(out.read(&mut [0u8; 1]).unwrap(), 0);
        assert_eq!(err.read(&mut [0u8; 1]).unwrap(), 0);
    }
}
#[test]
fn dispatch_job_and_drop_do_not_prove_cessation_of_hidden_message_aliases() {
    let [mut out, out_w] = pipe();
    let [mut err, err_w] = pipe();
    let request = request_kind([&out_w, &err_w], "ok");
    let hidden = request.clone();
    drop(out_w);
    drop(err_w);
    let mut exchange = DispatchExchange::prepare(request, MANAGER, CLIENT).unwrap();
    exchange.arm().unwrap();
    let good = reply(&exchange, "/org/freedesktop/systemd1/job/74");
    exchange.observe(&good).unwrap();
    drop(exchange);
    assert_eq!(
        out.read(&mut [0u8; 1]).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_eq!(
        err.read(&mut [0u8; 1]).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    drop(hidden);
    assert_eq!(out.read(&mut [0u8; 1]).unwrap(), 0);
    assert_eq!(err.read(&mut [0u8; 1]).unwrap(), 0);
}
