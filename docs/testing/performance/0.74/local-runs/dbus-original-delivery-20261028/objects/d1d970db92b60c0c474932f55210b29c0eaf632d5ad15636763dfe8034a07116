//! Borrowed state is reply data, never request custody or start authority.
use super::*;
#[test]
fn dispatch_reply_state_requires_the_borrowed_preparation_api() {
    let held = exchange();
    let _state = ReplyState::prepare(&held.request, MANAGER, CLIENT).unwrap();
}

#[test]
fn dispatch_reply_state_does_not_keep_any_request_writer_alive() {
    let [mut out, out_w] = pipe();
    let [mut err, err_w] = pipe();
    let request = request_kind([&out_w, &err_w], "ok");
    let response = Message::method_return(&request.header())
        .unwrap()
        .sender(MANAGER)
        .unwrap()
        .destination(CLIENT)
        .unwrap()
        .build(&OwnedObjectPath::try_from("/org/freedesktop/systemd1/job/74").unwrap())
        .unwrap();
    let mut state = ReplyState::prepare(&request, MANAGER, CLIENT).unwrap();
    drop(out_w);
    drop(err_w);
    drop(request);
    assert_eq!(out.read(&mut [0; 1]).unwrap(), 0);
    assert_eq!(err.read(&mut [0; 1]).unwrap(), 0);
    // This can still classify data: it is NOT request custody or start authority.
    state.arm().unwrap();
    assert_eq!(
        state.observe(&response).unwrap().path.as_str(),
        "/org/freedesktop/systemd1/job/74"
    );
    assert_eq!(state.arm(), Err(ExchangeError::AlreadyAttempted));
}

#[test]
fn dispatch_reply_state_refuses_borrowed_invalid_requests_without_taking_them() {
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
        let [mut out, out_w] = pipe();
        let [mut err, err_w] = pipe();
        let request = request_kind([&out_w, &err_w], kind);
        assert!(
            matches!(
                ReplyState::prepare(&request, MANAGER, CLIENT),
                Err(ExchangeError::Request)
            ),
            "{kind}"
        );
        drop(out_w);
        drop(err_w);
        assert_eq!(
            out.read(&mut [0; 1]).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        // A missing stderr property never created that request alias.
        if kind == "missing-fd" {
            assert_eq!(err.read(&mut [0; 1]).unwrap(), 0);
        } else {
            assert_eq!(
                err.read(&mut [0; 1]).unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
        }
        drop(request);
        assert_eq!(out.read(&mut [0; 1]).unwrap(), 0);
        assert_eq!(err.read(&mut [0; 1]).unwrap(), 0);
    }
}

#[test]
fn dispatch_reply_state_owning_delegate_keeps_the_exact_request_descriptors() {
    use std::os::fd::AsRawFd;
    let [mut out, out_w] = pipe();
    let [mut err, err_w] = pipe();
    let request = request_kind([&out_w, &err_w], "ok");
    let fds: Vec<_> = request
        .data()
        .fds()
        .iter()
        .map(AsRawFd::as_raw_fd)
        .collect();
    let mut held = DispatchExchange::prepare(request, MANAGER, CLIENT).unwrap();
    drop(out_w);
    drop(err_w);
    assert_eq!(
        held.request
            .data()
            .fds()
            .iter()
            .map(AsRawFd::as_raw_fd)
            .collect::<Vec<_>>(),
        fds
    );
    held.arm().unwrap();
    let response = reply(&held, "/org/freedesktop/systemd1/job/74");
    held.observe(&response).unwrap();
    for reader in [&mut out, &mut err] {
        assert_eq!(
            reader.read(&mut [0; 1]).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
    drop(held);
    assert_eq!(out.read(&mut [0; 1]).unwrap(), 0);
    assert_eq!(err.read(&mut [0; 1]).unwrap(), 0);
}

struct Model {
    phase: u8,
    open: bool,
}
impl Model {
    fn step(&mut self, action: u64) -> u8 {
        if action == 0 {
            if self.phase == 0 {
                self.phase = 1;
                self.open = true;
                return 0;
            }
            self.phase = 3;
            return 3;
        }
        if action == 1 {
            self.open = false;
            if self.phase == 0 {
                self.phase = 3;
            }
            return 1;
        }
        let pending = self.phase == 1 && self.open;
        self.open = false;
        self.phase = 3;
        if !pending {
            return 4;
        }
        if matches!(action, 2 | 6) {
            self.phase = 2;
            return 2;
        }
        if action == 5 {
            return 5;
        }
        4
    }
}
fn response(held: &DispatchExchange, action: u64) -> Message {
    let good = reply(held, "/org/freedesktop/systemd1/job/74");
    let mut builder = if action == 5 {
        Message::error(
            &held.request.header(),
            "org.freedesktop.systemd1.UnitExists",
        )
        .unwrap()
        .sender(MANAGER)
        .unwrap()
        .destination(CLIENT)
        .unwrap()
    } else {
        zbus::message::Builder::from(good.header())
    };
    match action {
        3 => builder = builder.sender(":1.999").unwrap(),
        4 => {
            builder =
                builder.reply_serial(NonZeroU32::new(held.serial.get().wrapping_add(1).max(1)))
        }
        6 => {
            // The high-level builder rejects this bit on replies; construct the
            // interoperable wire form through Header, as in the old flag corpus.
            let mut header = good.header();
            let flags = header.primary().flags() | zbus::message::Flags::NoReplyExpected;
            header.primary_mut().set_flags(flags);
            builder = zbus::message::Builder::from(header);
        }
        7 => {
            builder = builder
                .with_flags(zbus::message::Flags::NoAutoStart)
                .unwrap()
        }
        _ => (),
    }
    if action == 5 || action == 8 {
        return builder.build(&"private-detail").unwrap();
    }
    let path = if action == 9 {
        "/org/freedesktop/systemd1/job/0"
    } else {
        "/org/freedesktop/systemd1/job/74"
    };
    builder
        .build(&OwnedObjectPath::try_from(path).unwrap())
        .unwrap()
}
fn error_code(error: ExchangeError) -> u8 {
    match error {
        ExchangeError::AlreadyAttempted => 3,
        ExchangeError::Reply => 4,
        ExchangeError::ManagerError => 5,
        ExchangeError::Request => panic!("request error after preparation"),
    }
}
fn classify(result: Result<JobObservation, ExchangeError>) -> u8 {
    match result {
        Ok(job) => {
            assert_eq!(job.path.as_str(), "/org/freedesktop/systemd1/job/74");
            2
        }
        Err(error) => error_code(error),
    }
}
#[test]
fn dispatch_reply_state_matches_seeded_independent_model_and_owning_delegate() {
    let seed = 0x0740_e518_u64;
    let mut random = seed;
    let prefixes: &[&[u64]] = &[
        &[],
        &[0],
        &[0, 2],
        &[0, 1],
        &[1],
        &[2],
        &[0, 5],
        &[0, 6],
        &[0, 7],
        &[0, 4],
    ];
    let mut coverage = [0usize; 6];
    for case in 0..512 {
        let mut held = exchange();
        let mut state = ReplyState::prepare(&held.request, MANAGER, CLIENT).unwrap();
        let mut model = Model {
            phase: 0,
            open: false,
        };
        let mut actions = prefixes[case % prefixes.len()].to_vec();
        for _ in 0..32 {
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            actions.push(random % 10);
        }
        for action in actions {
            let expected = model.step(action);
            coverage[expected as usize] += 1;
            let (own, borrowed) = match action {
                0 => (
                    held.arm().map(|()| 0).unwrap_or_else(error_code),
                    state.arm().map(|()| 0).unwrap_or_else(error_code),
                ),
                1 => {
                    held.lose_reply();
                    state.lose_reply();
                    (1, 1)
                }
                _ => {
                    let message = response(&held, action);
                    (
                        classify(held.observe(&message)),
                        classify(state.observe(&message)),
                    )
                }
            };
            assert_eq!(
                (own, borrowed),
                (expected, expected),
                "seed={seed:x} case={case} action={action}"
            );
            let phase = match model.phase {
                0 => Phase::Prepared,
                1 => Phase::Uncertain,
                2 => Phase::JobObserved,
                _ => Phase::Refused,
            };
            assert_eq!(held.phase, phase);
            assert_eq!(state.phase, phase);
            assert_eq!(held.reply_open, model.open);
            assert_eq!(state.reply_open, model.open);
            assert_eq!(held.serial, state.serial);
            assert_eq!(state.manager, MANAGER);
            assert_eq!(state.client, CLIENT);
        }
    }
    assert!(
        coverage.iter().all(|count| *count > 0),
        "seed={seed:x} coverage={coverage:?}"
    );
    println!("HC074-REPLY-STATE seed={seed:08x} sequences=512 random_steps=16384 checks={} coverage={coverage:?}", coverage.iter().sum::<usize>());
}
