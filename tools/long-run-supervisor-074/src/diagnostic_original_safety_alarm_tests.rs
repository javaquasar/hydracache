use super::*;

#[test]
fn original_alarm_bounds_exclude_equality_overflow_and_later_starting_renewal() {
    let b = boundaries(NS, 4 * NS, 2 * NS, 4 * NS, 5 * NS).unwrap();
    assert_eq!((b.total, b.cell, b.progress), (301 * NS, 64 * NS, 12 * NS));
    assert_eq!(b.first(), 12 * NS);
    assert_ne!(b.progress, 4 * NS + CONTROLLER_LOSS_SECONDS * NS);
    for now in [12 * NS, 13 * NS] {
        assert!(matches!(
            boundaries(NS, 4 * NS, 2 * NS, 4 * NS, now),
            Err(OriginalSafetyAlarmError::Expired)
        ));
    }
    for args in [
        (0, 4, 2, 4, 5),
        (2, 4, 1, 4, 5),
        (1, 2, 3, 4, 5),
        (1, 4, 2, 3, 5),
        (1, 4, 2, 4, 3),
    ] {
        assert!(matches!(
            boundaries(args.0, args.1, args.2, args.3, args.4),
            Err(OriginalSafetyAlarmError::Binding)
        ));
    }
    assert!(matches!(
        boundaries(u64::MAX - 1, u64::MAX, u64::MAX - 1, u64::MAX, u64::MAX),
        Err(OriginalSafetyAlarmError::Binding)
    ));
    // Total/cell bounds also win independently; no round-up or one-second grace.
    assert_eq!(
        boundaries(NS, 295 * NS, 295 * NS, 295 * NS, 296 * NS)
            .unwrap()
            .first(),
        301 * NS
    );
    assert!(matches!(
        boundaries(NS, 295 * NS, 295 * NS, 295 * NS, 301 * NS),
        Err(OriginalSafetyAlarmError::Expired)
    ));
    // In the current first-cell grammar progress precedes Starting, so the
    // ten-second progress ceiling always precedes the sixty-second cell bound.
}
#[test]
fn original_alarm_seeded_bounds_match_checked_reference_without_time_refresh() {
    let mut seed = 0x0740_a117_u64;
    for _ in 0..4096 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let reserved = 1 + seed % 1_000_000_000;
        let progress = reserved + (seed.rotate_left(17) % (299 * NS));
        let cell = progress + (seed.rotate_left(31) % NS);
        let last = cell;
        let now = cell + seed.rotate_left(41) % (12 * NS);
        let expected = (reserved + TOTAL_SECONDS * NS)
            .min(cell + CELL_SECONDS * NS)
            .min(progress + CONTROLLER_LOSS_SECONDS * NS);
        match boundaries(reserved, cell, progress, last, now) {
            Ok(b) => {
                assert!(now < expected);
                assert_eq!(b.first(), expected);
            }
            Err(OriginalSafetyAlarmError::Expired) => assert!(now >= expected),
            Err(e) => panic!("{e:?}"),
        }
    }
    println!("HC074-ALARM-BOUNDS seed=0x0740a117 cases=4096 exact=true renewed=false");
}
fn set(alarm: &Alarm, deadline: u64, interval: u64, flags: i32) {
    let spec = libc::itimerspec {
        it_value: timespec(deadline),
        it_interval: timespec(interval),
    };
    assert_eq!(
        unsafe {
            libc::timerfd_settime(alarm.file.as_raw_fd(), flags, &spec, std::ptr::null_mut())
        },
        0
    );
}
#[test]
fn original_alarm_kernel_flags_clock_interval_disarm_and_extension_refuse() {
    for kind in 0..6 {
        let mut alarm = Alarm::arm(clock().unwrap() + 30 * NS).unwrap();
        alarm.inspect().unwrap();
        match kind {
            0 => set(&alarm, 0, 0, libc::TFD_TIMER_ABSTIME),
            1 => set(&alarm, alarm.deadline + 60 * NS, 0, libc::TFD_TIMER_ABSTIME),
            2 => set(&alarm, alarm.deadline, NS, libc::TFD_TIMER_ABSTIME),
            3 => assert_eq!(
                unsafe { libc::fcntl(alarm.file.as_raw_fd(), libc::F_SETFD, 0) },
                0
            ),
            4 => alarm.file = tempfile::tempfile().unwrap(),
            5 => {
                let fd = unsafe {
                    libc::timerfd_create(
                        libc::CLOCK_MONOTONIC,
                        libc::TFD_CLOEXEC | libc::TFD_NONBLOCK,
                    )
                };
                assert!(fd >= 0);
                alarm.file = unsafe { File::from_raw_fd(fd) };
                set(&alarm, alarm.deadline, 0, libc::TFD_TIMER_ABSTIME);
            }
            _ => unreachable!(),
        }
        assert!(alarm.inspect().is_err(), "kind={kind}");
    }
}
#[test]
fn original_alarm_kernel_expiry_is_readable_and_validation_does_not_consume_or_rearm() {
    let mut alarm = Alarm::arm(clock().unwrap() + 100_000_000).unwrap();
    alarm.inspect().unwrap();
    let mut p = libc::pollfd {
        fd: alarm.file.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    assert_eq!(unsafe { libc::poll(&mut p, 1, 5000) }, 1);
    assert_eq!(p.revents, libc::POLLIN);
    assert!(matches!(
        alarm.inspect(),
        Err(OriginalSafetyAlarmError::Expired)
    ));
    assert!(matches!(
        alarm.inspect(),
        Err(OriginalSafetyAlarmError::Expired)
    ));
    let mut b = [0; 8];
    alarm.file.read_exact(&mut b).unwrap();
    assert_eq!(u64::from_ne_bytes(b), 1);
    assert!(matches!(
        alarm.inspect(),
        Err(OriginalSafetyAlarmError::Expired)
    ));
    assert_eq!(
        alarm.file.read(&mut b).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    // Do not inspect the numeric descriptor after Drop: parallel tests may
    // immediately reuse it. File ownership closes it; this is not a guardian test.
    drop(alarm);
    assert!(matches!(
        Alarm::arm(clock().unwrap()),
        Err(OriginalSafetyAlarmError::Expired)
    ));
}
