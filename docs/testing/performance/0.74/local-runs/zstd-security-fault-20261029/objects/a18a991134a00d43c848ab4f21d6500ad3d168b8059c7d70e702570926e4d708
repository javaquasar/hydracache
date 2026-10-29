use super::*;
use crate::root::launch::tests::root;

fn record(name: &str, kind: u8) -> Vec<u8> {
    let len = (20 + name.len() + 7) & !7;
    let mut bytes = vec![0; len];
    bytes[16..18].copy_from_slice(&(len as u16).to_ne_bytes());
    bytes[18] = kind;
    bytes[19..19 + name.len()].copy_from_slice(name.as_bytes());
    bytes
}
#[test]
fn retained_directory_decoder_refuses_noncanonical_duplicate_and_malformed_records() {
    let good: Vec<_> = [
        (".", libc::DT_DIR),
        ("..", libc::DT_DIR),
        ("0", libc::DT_LNK),
        ("3", libc::DT_LNK),
    ]
    .into_iter()
    .flat_map(|(n, k)| record(n, k))
    .collect();
    let mut names = BTreeSet::new();
    records(&good, &mut names).unwrap();
    assert_eq!(names.len(), 4);
    assert!(records(&good, &mut names).is_err());
    for bad in [
        record("03", libc::DT_LNK),
        record("+1", libc::DT_LNK),
        record("3", libc::DT_DIR),
        record("..", libc::DT_LNK),
        record("", libc::DT_LNK),
        vec![0; 513],
        good[..good.len() - 1].to_vec(),
    ] {
        assert!(records(&bad, &mut BTreeSet::new()).is_err());
    }
    for length in [0u16, 19, 23, 25, 65535] {
        let mut bad = record("3", libc::DT_LNK);
        bad[16..18].copy_from_slice(&length.to_ne_bytes());
        assert!(records(&bad, &mut BTreeSet::new()).is_err());
    }
}
#[test]
fn kernel_filter_state_requires_single_bounded_consistent_pair() {
    for raw in [
        "Seccomp: 0\nSeccomp_filters: 0\n",
        "Seccomp: 2\nSeccomp_filters: 1\n",
        "Seccomp: 2\nSeccomp_filters: 65\n",
    ] {
        filter_state(raw).unwrap();
    }
    for raw in [
        "",
        "Seccomp: 1\nSeccomp_filters: 0\n",
        "Seccomp: 2\nSeccomp_filters: 0\n",
        "Seccomp: 0\nSeccomp_filters: 1\n",
        "Seccomp: 2\nSeccomp_filters: 66\n",
        "Seccomp: 2\nSeccomp_filters: -1\n",
        "Seccomp: 2\nSeccomp_filters: 1\nSeccomp: 2\n",
        "Seccomp: 2\n",
        "Seccomp: 2\nSeccomp_filters: 1",
    ] {
        assert!(filter_state(raw).is_err(), "{raw:?}");
    }
    assert!(filter_state(&"x".repeat(65537)).is_err());
}
#[test]
fn syscall_decoder_rejects_short_foreign_abi_unknown_operation_and_exit_shape() {
    let mut info: libc::ptrace_syscall_info = unsafe { std::mem::zeroed() };
    info.arch = 0xc000_003e;
    info.op = 1;
    assert!(matches!(
        decode(&info, 80).unwrap(),
        Event::Entry { nr: 0, .. }
    ));
    for size in 0..80 {
        assert!(decode(&info, size).is_err());
    }
    assert!(decode(&info, std::mem::size_of_val(&info) + 1).is_err());
    for arch in [0, 0x4000_0003, u32::MAX] {
        info.arch = arch;
        assert!(decode(&info, 80).is_err());
    }
    info.arch = 0xc000_003e;
    for operation in [0, 3, 255] {
        info.op = operation;
        assert!(decode(&info, 80).is_err());
    }
    info.op = 2;
    assert!(matches!(
        decode(&info, 33).unwrap(),
        Event::Exit {
            value: 0,
            error: false
        }
    ));
    assert!(decode(&info, 32).is_err());
    info.u.exit.is_error = 2;
    assert!(decode(&info, 33).is_err());
    info.op = 1;
    info.flags = 1;
    assert!(decode(&info, 80).is_err());
}
fn stopped() -> (Owner, tempfile::TempDir) {
    let (root, temp) = root();
    (Owner::new(root).unwrap(), temp)
}
fn poke(owner: &Owner, address: usize, raw: &[u8]) {
    assert_eq!(address % 8, 0);
    for (i, word) in raw.chunks_exact(8).enumerate() {
        let data = usize::from_ne_bytes(word.try_into().unwrap());
        assert_eq!(
            unsafe { libc::ptrace(libc::PTRACE_POKEDATA, owner.pid, address + i * 8, data) },
            0
        );
    }
}
#[test]
fn original_stopped_child_memory_header_policy_and_seeded_mutations_are_bounded() {
    let (mut owner, _temp) = stopped();
    let mut regs: libc::user_regs_struct = unsafe { std::mem::zeroed() };
    assert_eq!(
        unsafe { libc::ptrace(libc::PTRACE_GETREGS, owner.pid, 0usize, &mut regs) },
        0
    );
    // Test-only original pre-exec stack, NOT a synthetic loaded-runtime proof.
    let maps = owner.text("maps").unwrap();
    let (low, high) = maps
        .lines()
        .filter_map(|line| {
            let p: Vec<_> = line.split_whitespace().collect();
            let (l, h) = p.first()?.split_once('-')?;
            let l = usize::from_str_radix(l, 16).ok()?;
            let h = usize::from_str_radix(h, 16).ok()?;
            (p.get(1) == Some(&"rw-p") && l <= regs.rsp as usize && (regs.rsp as usize) < h)
                .then_some((l, h))
        })
        .next()
        .unwrap();
    owner.stack = Some((low, high));
    let base = (regs.rsp as usize - 2048) & !7;
    let address = base + 512;
    let expected = hydracache_long_run_supervisor_074::diagnostic_signer_isolation::policy_bytes();
    poke(&owner, base, &expected);
    let mut header = [0u8; 16];
    header[..2].copy_from_slice(&64u16.to_ne_bytes());
    header[8..].copy_from_slice(&base.to_ne_bytes());
    poke(&owner, address, &header);
    policy(&owner, address).unwrap();
    let mut seed = 0x0740_c07c_u64;
    for _ in 0..256 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let index = seed as usize % 512;
        let mut bad = expected;
        bad[index] ^= 1 << (seed % 8);
        let begin = index & !7;
        poke(&owner, base + begin, &bad[begin..begin + 8]);
        assert!(policy(&owner, address).is_err());
        poke(&owner, base + begin, &expected[begin..begin + 8]);
    }
    for len in [0u16, 63, 65, u16::MAX] {
        header[..2].copy_from_slice(&len.to_ne_bytes());
        poke(&owner, address, &header);
        assert!(policy(&owner, address).is_err());
    }
    header[..2].copy_from_slice(&64u16.to_ne_bytes());
    header[8..].copy_from_slice(&usize::MAX.to_ne_bytes());
    poke(&owner, address, &header);
    assert!(policy(&owner, address).is_err());
    for (address, size) in [
        (0, 16),
        (low - 1, 16),
        (high - 1, 16),
        (base, 0),
        (base, 513),
        (usize::MAX, 16),
    ] {
        assert!(memory(&owner, address, size).is_err());
    }
    poke(&owner, base, &[255; 8]);
    assert_eq!(memory(&owner, base, 8).unwrap(), [255; 8]);
    owner.terminate().unwrap();
    println!("root_policy_memory seed=0x0740c07c mutations=256 original_child=true ptrace_words=true authority=false");
}
#[test]
fn fixed_install_order_duplicate_entry_missing_exit_and_failed_syscall_refuse() {
    let (owner, _temp) = stopped();
    let mut step = 1;
    let mut expected = None;
    let event = || Event::Entry {
        nr: libc::SYS_prctl as u64,
        args: [38, 1, 0, 0, 0, 0],
    };
    accept(&owner, &mut step, &mut expected, event()).unwrap();
    assert!(accept(&owner, &mut step, &mut expected, event()).is_err());
    accept(
        &owner,
        &mut step,
        &mut expected,
        Event::Exit {
            value: 0,
            error: false,
        },
    )
    .unwrap();
    assert_eq!(step, 2);
    assert!(accept(
        &owner,
        &mut step,
        &mut expected,
        Event::Exit {
            value: 1,
            error: false
        }
    )
    .is_err());
    for (value, error) in [(-1, true), (1, false)] {
        let mut at = 1;
        let mut pending = None;
        accept(&owner, &mut at, &mut pending, event()).unwrap();
        assert!(accept(&owner, &mut at, &mut pending, Event::Exit { value, error }).is_err());
    }
    for stage in 0..10 {
        assert!(entry(&owner, stage, libc::SYS_openat as u64, [0; 6]).is_err());
    }
    for bit in [32, 63] {
        assert!(entry(
            &owner,
            1,
            libc::SYS_prctl as u64,
            [38, 1 | (1 << bit), 0, 0, 0, 0]
        )
        .is_err());
    }
    assert!(entry(&owner, 4, libc::SYS_prctl as u64, [4, 1, 0, 0, 0, 0]).is_err());
}
#[test]
fn retained_proc_readers_require_original_procfs_cloexec_and_no_foreign_replacement() {
    let (owner, temp) = stopped();
    let mut readers = Readers::capture(&owner).unwrap();
    assert!(readers.names().unwrap().contains("3"));
    let fd = readers.descriptors.file.as_raw_fd();
    assert_eq!(unsafe { libc::fcntl(fd, libc::F_SETFD, 0) }, 0);
    assert!(readers.names().is_err());
    assert_eq!(
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) },
        0
    );
    readers.names().unwrap();
    readers.descriptors.file = File::open(temp.path()).unwrap();
    assert!(readers.names().is_err());
    assert!(ProcRead::capture(File::open(temp.path()).unwrap(), true).is_err());
}
