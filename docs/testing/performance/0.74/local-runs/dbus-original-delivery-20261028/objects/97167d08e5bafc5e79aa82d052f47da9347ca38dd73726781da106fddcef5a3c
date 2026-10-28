use super::*;

const GOOD: &str = "Uid:\t1000 1000 1000 1000\nGid:\t1001 1001 1001 1001\nThreads:\t1\nCapInh:\t0000000000000000\nCapPrm:\t0000000000000000\nCapEff:\t0000000000000000\nCapAmb:\t0000000000000000\nNoNewPrivs:\t0\nSeccomp:\t0\nSeccomp_filters:\t0\n";

#[test]
fn child_entry_status_bounds_duplicates_credentials_and_capabilities() {
    assert!(decode_status(GOOD, 1000, 1001).is_ok());
    for (uid, gid) in [(0, 1001), (1000, 0), (1002, 1001), (1000, 1002)] {
        assert!(decode_status(GOOD, uid, gid).is_err());
    }
    for (old, new) in [
        ("Threads:\t1", "Threads:\t2"),
        ("Threads:\t1", "Threads:\t+1"),
        ("1000 1000 1000 1000", "1000 1000 1000 1002"),
        ("1001 1001 1001 1001", "1001 1001 1001"),
        ("NoNewPrivs:\t0", "NoNewPrivs:\t2"),
        ("Seccomp:\t0", "Seccomp:\t1"),
        ("Seccomp_filters:\t0", "Seccomp_filters:\t1"),
    ] {
        // Rust accepts a leading '+' integer; reject it in the bounded numeric grammar.
        assert!(
            decode_status(&GOOD.replace(old, new), 1000, 1001).is_err(),
            "{new}"
        );
    }
    for name in [
        "Uid:",
        "Gid:",
        "Threads:",
        "CapInh:",
        "CapPrm:",
        "CapEff:",
        "CapAmb:",
        "NoNewPrivs:",
        "Seccomp:",
        "Seccomp_filters:",
    ] {
        let line = GOOD.lines().find(|line| line.starts_with(name)).unwrap();
        assert!(decode_status(&format!("{GOOD}{line}\n"), 1000, 1001).is_err());
        assert!(decode_status(&GOOD.replace(&format!("{line}\n"), ""), 1000, 1001).is_err());
    }
    for name in ["CapInh:", "CapPrm:", "CapEff:", "CapAmb:"] {
        assert!(decode_status(
            &GOOD.replace(
                &format!("{name}\t0000000000000000"),
                &format!("{name}\t0000000000000001")
            ),
            1000,
            1001
        )
        .is_err());
    }
    assert!(decode_status(&"x".repeat(65537), 1000, 1001).is_err());
    assert!(decode_status("", 1000, 1001).is_err());
}

#[test]
fn child_entry_status_filter_count_has_bounded_post_install_slot() {
    for count in [1, 64, 65] {
        let raw = GOOD
            .replace("Seccomp:\t0", "Seccomp:\t2")
            .replace("Seccomp_filters:\t0", &format!("Seccomp_filters:\t{count}"));
        assert_eq!(decode_status(&raw, 1000, 1001).unwrap().filters, count);
    }
    let raw = GOOD
        .replace("Seccomp:\t0", "Seccomp:\t2")
        .replace("Seccomp_filters:\t0", "Seccomp_filters:\t66");
    assert!(decode_status(&raw, 1000, 1001).is_err());
}

fn record(name: &str, kind: u8) -> Vec<u8> {
    let len = (20 + name.len() + 7) & !7;
    let mut raw = vec![0; len];
    raw[16..18].copy_from_slice(&(len as u16).to_ne_bytes());
    raw[18] = kind;
    raw[19..19 + name.len()].copy_from_slice(name.as_bytes());
    raw
}

#[test]
fn child_entry_directory_decoder_bounds_canonical_names_and_types() {
    let mut names = BTreeSet::new();
    let mut dots = BTreeSet::new();
    for (name, kind) in [
        (".", libc::DT_DIR),
        ("..", libc::DT_DIR),
        ("0", libc::DT_LNK),
        ("1", libc::DT_LNK),
        ("2", libc::DT_LNK),
        ("3", libc::DT_LNK),
    ] {
        records(&record(name, kind), &mut names, &mut dots).unwrap();
        assert!(records(&record(name, kind), &mut names, &mut dots).is_err());
    }
    assert_eq!(names, BTreeSet::from([0, 1, 2, 3]));
    for raw in [
        vec![0; 513],
        vec![0; 19],
        vec![0; 24],
        record("00", libc::DT_LNK),
        record("+1", libc::DT_LNK),
        record("x", libc::DT_LNK),
        record("", libc::DT_LNK),
        record("4294967296", libc::DT_LNK),
        record("0", libc::DT_DIR),
        record(".", libc::DT_LNK),
    ] {
        assert!(records(&raw, &mut BTreeSet::new(), &mut BTreeSet::new()).is_err());
    }
    for length in [0, 19, 23, 25, 512, 65535] {
        let mut raw = record("0", libc::DT_LNK);
        raw[16..18].copy_from_slice(&(length as u16).to_ne_bytes());
        assert!(records(&raw, &mut BTreeSet::new(), &mut BTreeSet::new()).is_err());
    }
    let mut raw = record("0", libc::DT_LNK);
    raw[19..].fill(b'0');
    assert!(records(&raw, &mut BTreeSet::new(), &mut BTreeSet::new()).is_err());
    let raw = (0..9)
        .flat_map(|fd| record(&fd.to_string(), libc::DT_LNK))
        .collect::<Vec<_>>();
    assert!(records(&raw, &mut BTreeSet::new(), &mut BTreeSet::new()).is_err());
}
