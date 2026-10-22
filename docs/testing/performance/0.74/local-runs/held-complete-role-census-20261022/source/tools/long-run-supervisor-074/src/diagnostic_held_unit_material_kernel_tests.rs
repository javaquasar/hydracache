use super::*;

#[test]
fn held_complete_census_kernel_bridge_joins_three_original_roles_to_material_inventory() {
    use crate::diagnostic_artifacts::linux::held_role_census::tests::Fixture;
    let fixture = Fixture::new();
    let (mut b, _initial, files) = role_fixture();
    b.worker = (id(&fixture.worker).unwrap(), crate::sha256_hex(b"worker"));
    b.guardians = fixture.claims();
    let resources = Resources::create(&b, b"config\n").unwrap();
    let fds = files.each_ref().map(AsRawFd::as_raw_fd);
    let census = fixture.ready();
    let inventory = resources.inspect(&b, fds).unwrap();
    let partial = bind_roles(&b, &inventory).unwrap();
    let a = complete_roles(&partial, &inventory, census).unwrap();
    let census = fixture.ready();
    let inventory = resources.inspect(&b, fds).unwrap();
    let b = complete_roles(&partial, &inventory, census).unwrap();
    assert!(a == b);
}

fn role_fixture() -> (Bindings, Resources, [File; 14]) {
    let mut b = bindings();
    let mut files = original();
    for i in 0..2 {
        let mut fds = [-1; 2];
        assert_eq!(unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) }, 0);
        files[2 + i] = owned(fds[0]).unwrap();
        files[6 + i] = owned(fds[1]).unwrap();
        b.streams[i] = pair_id(&files[6 + i]).unwrap();
    }
    let resources = Resources::create(&b, b"config\n").unwrap();
    (b, resources, files)
}

#[test]
fn held_material_roles_bind_exact_eight_objects_without_descriptor_copies_or_offset_changes() {
    let (b, resources, files) = role_fixture();
    let fds = files.each_ref().map(AsRawFd::as_raw_fd);
    let inventory = resources.inspect(&b, fds).unwrap();
    let before = resources.inspect(&b, fds).unwrap();
    let roles = bind_roles(&b, &inventory).unwrap();
    let source = [14, 6, 7, 16, 17, 19, 1, 18];
    assert_eq!(roles.0.map(|v| v.0), [0, 1, 2, 3, 5, 6, 9, 10]);
    assert_eq!(roles.0.map(|v| v.1), source.map(|i| inventory[i].fd));
    for i in [0, 2, 3, 4, 5, 8, 9, 10, 11, 12, 13, 15] {
        assert!(roles.0.iter().all(|v| v.1 != inventory[i].fd));
    }
    assert_eq!(roles, bind_roles(&b, &inventory).unwrap());
    assert_eq!(resources.inspect(&b, fds).unwrap(), before);
}

#[test]
fn held_material_roles_refuse_each_negative_duplicate_and_non_cloexec_census_entry() {
    let (b, resources, files) = role_fixture();
    let fds = files.each_ref().map(AsRawFd::as_raw_fd);
    for i in 0..20 {
        for fault in 0..3 {
            let mut inventory = resources.inspect(&b, fds).unwrap();
            match fault {
                0 => inventory[i].fd = -1,
                1 => inventory[i].fd = inventory[(i + 1) % 20].fd,
                _ => inventory[i].descriptor = 0,
            }
            assert!(bind_roles(&b, &inventory).is_err(), "{i}/{fault}");
        }
    }
    let inventory = resources.inspect(&b, fds).unwrap();
    for count in 0..20 {
        assert!(bind_roles(&b, &inventory[..count]).is_err());
    }
    let mut extra = resources.inspect(&b, fds).unwrap();
    extra.push(stamp(fds[0]).unwrap());
    assert!(bind_roles(&b, &extra).is_err());
}

#[test]
fn held_material_roles_refuse_wrong_stream_resource_shapes_and_exchanged_objects() {
    let (b, resources, files) = role_fixture();
    let fds = files.each_ref().map(AsRawFd::as_raw_fd);
    for fault in 0..12 {
        let mut inventory = resources.inspect(&b, fds).unwrap();
        match fault {
            0 => inventory[6].stat[1] += 1,
            1 => inventory[7].stat[0] += 1,
            2 => inventory[6].status = libc::O_RDONLY,
            3 => inventory[7].stat[4] = libc::S_IFREG as u64,
            4 => inventory.swap(6, 7),
            5 => inventory[14].stat[4] = libc::S_IFREG as u64,
            6 => inventory[16].stat[4] = libc::S_IFIFO as u64,
            7 => inventory[17].status |= libc::O_PATH,
            8 => inventory[18].stat[4] = libc::S_IFSOCK as u64,
            9 => inventory[17].stat[6] += 1,
            10 => inventory[18].stat[6] += 1,
            _ => inventory[16].status = libc::O_RDONLY,
        }
        assert!(bind_roles(&b, &inventory).is_err(), "{fault}");
    }
}

#[test]
fn held_material_roles_fixed_seed_census_preserves_exact_source_and_slot_order() {
    let (b, resources, files) = role_fixture();
    let fds = files.each_ref().map(AsRawFd::as_raw_fd);
    let mut seed = 0x0740_fd82u32;
    for _ in 0..256 {
        let mut inventory = resources.inspect(&b, fds).unwrap();
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let base = 1000 + (seed % 10000) as i32;
        for (i, s) in inventory.iter_mut().enumerate() {
            s.fd = base + i as i32;
        }
        let roles = bind_roles(&b, &inventory).unwrap();
        assert_eq!(
            roles.0,
            MATERIAL_ROLES.map(|(slot, at)| (slot, base + at as i32))
        );
    }
}

fn bindings() -> Bindings {
    Bindings {
        owner: current_owner(),
        deadline: now().unwrap() + 10_000_000_000,
        guardians: [(1_000_001, [8, 9]), (1_000_002, [8, 10])],
        worker: ([2, 3, 64], "b".repeat(64)),
        config: ([4, 5, 7], crate::sha256_hex(b"config\n")),
        streams: [[6, 7], [6, 8]],
    }
}
fn original() -> [File; 14] {
    std::array::from_fn(|_| tempfile::tempfile().unwrap())
}
fn word(raw: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(raw[at..at + 8].try_into().unwrap())
}

#[test]
fn held_material_kernel_creates_exact_readonly_sealed_protocol_and_twenty_role_inventory() {
    let b = bindings();
    let resources = Resources::create(&b, b"config\n").unwrap();
    let files = original();
    let fds = files.each_ref().map(AsRawFd::as_raw_fd);
    let first = resources.inspect(&b, fds).unwrap();
    assert_eq!(first.len(), 20);
    assert_eq!(resources.inspect(&b, fds).unwrap(), first);
    let raw = read(&resources.bootstrap, BOOT_BYTES).unwrap();
    assert_eq!(&raw[..8], b"HCHL0741");
    assert_eq!(word(&raw, 8), b.deadline);
    for (at, v) in [
        (16, b.owner[0]),
        (20, b.owner[4]),
        (24, b.owner[5]),
        (28, 1_000_001),
        (32, 1_000_002),
    ] {
        assert_eq!(
            u32::from_le_bytes(raw[at..at + 4].try_into().unwrap()),
            v as u32
        );
    }
    assert_eq!(&raw[36..40], &[0; 4]);
    assert_eq!(&raw[248..], &[0; 8]);
    assert_eq!(&raw[40..72], &resources.token);
    assert_ne!(resources.token, [0; 32]);
    assert_eq!(&raw[72..104], &hash(&b.worker.1).unwrap());
    assert_eq!(&raw[104..136], &hash(&b.config.1).unwrap());
    assert_eq!(
        [word(&raw, 136), word(&raw, 144), word(&raw, 152)],
        b.worker.0
    );
    assert_eq!(
        [word(&raw, 160), word(&raw, 168), word(&raw, 176)],
        id(&resources.config).unwrap()
    );
    for (at, expected) in [
        (184, pair_id(&resources.stdin).unwrap()),
        (200, b.streams[0]),
        (216, b.streams[1]),
        (232, pair_id(&resources.release[1]).unwrap()),
    ] {
        assert_eq!([word(&raw, at), word(&raw, at + 8)], expected);
    }
    for f in [&resources.config, &resources.bootstrap] {
        assert_eq!(
            unsafe { libc::pwrite(f.as_raw_fd(), b"x".as_ptr().cast(), 1, 0) },
            -1
        );
        match OpenOptions::new()
            .write(true)
            .open(format!("/proc/thread-self/fd/{}", f.as_raw_fd()))
        {
            Ok(mut alias) => assert!(alias.write_all(b"x").is_err()),
            Err(error) => assert_eq!(error.kind(), io::ErrorKind::PermissionDenied),
        }
    }
    assert_eq!(read(&resources.config, 65536).unwrap(), b"config\n");
}

#[test]
fn held_material_kernel_refuses_wrong_bytes_lengths_owner_and_expired_original_deadline() {
    for at in 0..10 {
        let mut b = bindings();
        match at {
            0 => b.owner[0] += 1,
            1 => b.owner[1] += 1,
            2 => b.owner[4] += 1,
            3 => b.deadline = now().unwrap().saturating_sub(1),
            4 => b.guardians[0].0 = 1,
            5 => b.guardians[1] = b.guardians[0],
            6 => b.guardians[0].0 = b.owner[0] as i32,
            7 => b.config.0[2] += 1,
            8 => b.config.1 = "0".repeat(64),
            _ => b.worker.1 = "bad".into(),
        }
        assert!(Resources::create(&b, b"config\n").is_err(), "{at}");
    }
    assert!(Resources::create(&bindings(), b"other\n").is_err());
    assert!(sealed(&[]).is_err());
    assert!(sealed(&vec![0; 65537]).is_err());
}

#[test]
fn held_material_kernel_detects_changed_flags_offsets_endpoint_and_bootstrap_without_repair() {
    for at in 0..8 {
        let b = bindings();
        let mut r = Resources::create(&b, b"config\n").unwrap();
        match at {
            0 => {
                assert_eq!(
                    unsafe { libc::fcntl(r.stdin.as_raw_fd(), libc::F_SETFD, 0) },
                    0
                );
            }
            1 => {
                assert_eq!(
                    unsafe { libc::fcntl(r.release[0].as_raw_fd(), libc::F_SETFL, 0) },
                    0
                );
            }
            2 => {
                assert_eq!(
                    unsafe { libc::lseek(r.config.as_raw_fd(), 1, libc::SEEK_SET) },
                    1
                );
            }
            3 => {
                let [replacement, _] = release().unwrap();
                r.release[1] = replacement;
            }
            4 => r.bootstrap = sealed(&[0; 256]).unwrap(),
            5 => r.config = sealed(b"tamper\n").unwrap(),
            6 => r.creator = tempfile::tempfile().unwrap(),
            _ => {
                assert_eq!(
                    unsafe { libc::fcntl(r.creator.as_raw_fd(), libc::F_SETFD, 0) },
                    0
                );
            }
        }
        assert!(r.validate(&b).is_err(), "{at}");
    }
}

#[test]
fn held_material_kernel_rejects_data_and_zero_packets_or_disconnection_on_release_channel() {
    for count in [0usize, 1] {
        let b = bindings();
        let r = Resources::create(&b, b"config\n").unwrap();
        assert_eq!(
            unsafe {
                libc::send(
                    r.release[0].as_raw_fd(),
                    b"x".as_ptr().cast(),
                    count,
                    libc::MSG_NOSIGNAL,
                )
            },
            count as isize
        );
        assert!(r.validate(&b).is_err()); // PEEK must not consume or heal it.
        assert!(r.validate(&b).is_err());
    }
    let b = bindings();
    let r = Resources::create(&b, b"config\n").unwrap();
    assert_eq!(
        unsafe { libc::shutdown(r.release[0].as_raw_fd(), libc::SHUT_RDWR) },
        0
    );
    assert!(r.validate(&b).is_err());
}

#[test]
fn held_material_kernel_seeded_bootstrap_substitutions_never_rebind_material() {
    let mut seed = 0x0740_fd61_u64;
    eprintln!("held material bootstrap seed=0x0740fd61 substitutions=256");
    let b = bindings();
    let r = Resources::create(&b, b"config\n").unwrap();
    let expected = read(&r.bootstrap, 256).unwrap();
    for at in 0..256 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut changed = expected.clone();
        changed[at] ^= 1 + (seed % 255) as u8;
        let candidate = sealed(&changed).unwrap();
        assert_ne!(
            read(&candidate, 256).unwrap(),
            bootstrap(
                &b,
                &r.token,
                pair_id(&r.stdin).unwrap(),
                id(&r.config).unwrap(),
                pair_id(&r.release[1]).unwrap()
            )
            .unwrap()
        );
    }
    assert!(bootstrap(&b, &[0; 32], [1, 2], [3, 4, 5], [6, 7]).is_err());
}

#[test]
fn held_material_kernel_inventory_refuses_duplicate_negative_new_alias_and_original_drift() {
    let b = bindings();
    let r = Resources::create(&b, b"config\n").unwrap();
    let files = original();
    let fds = files.each_ref().map(AsRawFd::as_raw_fd);
    let baseline = r.inspect(&b, fds).unwrap();
    for at in 0..14 {
        for value in [-1, r.stdin.as_raw_fd(), fds[(at + 1) % 14]] {
            let mut changed = fds;
            changed[at] = value;
            assert!(r.inspect(&b, changed).is_err(), "{at}/{value}");
        }
    }
    assert_eq!(unsafe { libc::lseek(fds[0], 1, libc::SEEK_SET) }, 1);
    assert_ne!(r.inspect(&b, fds).unwrap(), baseline);
}

#[test]
fn held_material_kernel_drop_closes_all_six_resources_and_partial_creation_is_raii() {
    let b = bindings();
    let r = Resources::create(&b, b"config\n").unwrap();
    let fds = r.fds();
    drop(r);
    // Do not open anything between drop and queries: no descriptor reuse race.
    for fd in fds {
        assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, -1);
    }
    // One deliberately late construction refusal, after stdin/channel/config/
    // creator allocation. The test uses no other FD-opening threads.
    let count = || std::fs::read_dir("/proc/thread-self/fd").unwrap().count();
    let before = count();
    let mut bad = bindings();
    bad.worker.1 = "invalid".into();
    assert!(Resources::create(&bad, b"config\n").is_err());
    assert_eq!(count(), before);
}
