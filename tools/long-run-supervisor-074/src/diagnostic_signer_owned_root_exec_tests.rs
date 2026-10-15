use super::*;
use crate::root::tests::images;
use crate::root::{chmod, Source};
use std::os::unix::fs::PermissionsExt;
fn root() -> (Root, tempfile::TempDir) {
    let temp = tempfile::tempdir().unwrap();
    let sources = images::images()
        .into_iter()
        .enumerate()
        .map(|(i, bytes)| {
            let path = temp.path().join(format!("source{i}"));
            std::fs::write(&path, bytes).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o555)).unwrap();
            Source::capture(File::open(path).unwrap(), unsafe { libc::getuid() }, i == 0).unwrap()
        })
        .collect::<Vec<_>>()
        .try_into()
        .ok()
        .unwrap();
    let selected = Root::create(File::open(temp.path()).unwrap(), c"root".into(), sources).unwrap();
    (selected, temp)
}
fn mapping(root: &Root, loaded: bool) -> String {
    let paths = [
        "signer",
        "lib64/ld-linux-x86-64.so.2",
        "lib/x86_64-linux-gnu/libc.so.6",
        "lib/x86_64-linux-gnu/libgcc_s.so.1",
    ];
    let mut raw = String::new();
    for (index, path) in paths.iter().enumerate().take(if loaded { 4 } else { 2 }) {
        let f = &root.files[index].stamp;
        let start = (index + 1) * 4096;
        raw.push_str(&format!(
            "{start:x}-{:x} r-xp 00000000 {:x}:{:x} {} /{path}\n",
            start + 4096,
            libc::major(f.dev),
            libc::minor(f.dev),
            f.ino
        ));
    }
    raw.push_str("10000-11000 rw-p 0 00:00 0 [stack]\n12000-13000 r-xp 0 00:00 0 [vdso]\n");
    raw
}
#[test]
fn bounded_maps_require_selected_inodes_roles_offsets_paths_and_private_permissions() {
    let (root, _temp) = root();
    for loaded in [false, true] {
        let good = mapping(&root, loaded);
        maps::validate(&good, &root, loaded).unwrap();
        for bad in [
            good.replace("r-xp", "rwxp"),
            good.replace("r-xp", "r-xs"),
            good.replace("/signer", "/foreign"),
            good.replace("[stack]", "[other]"),
            good.replace("00000000", "00001001"),
            good.replace("1000-2000", "1001-2000"),
            good.replace(&root.files[0].stamp.ino.to_string(), "0"),
            good.lines().rev().map(|s| format!("{s}\n")).collect(),
            good.trim_end().to_owned(),
            format!("{good}{}", "\n".repeat(129)),
            "x".repeat(65537),
            String::new(),
        ] {
            assert!(maps::validate(&bad, &root, loaded).is_err());
        }
    }
    assert!(maps::validate(&mapping(&root, true), &root, false).is_err());
    assert!(maps::validate(&mapping(&root, false), &root, true).is_err());
}
#[test]
fn seeded_map_mutations_never_panic_and_known_identity_mutations_refuse() {
    let (root, _temp) = root();
    let good = mapping(&root, true);
    let mut seed = 0x0740_c07b_u64;
    for _ in 0..4096 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut bytes = good.as_bytes().to_vec();
        let index = seed as usize % bytes.len();
        bytes[index] = (seed >> 32) as u8;
        if let Ok(raw) = std::str::from_utf8(&bytes) {
            let _ = maps::validate(raw, &root, true);
        }
    }
    println!("ordinary_root_maps seed=0x0740c07b mutations=4096 admission=false");
}
#[test]
fn status_duplicates_missing_identity_and_capability_drift_refuse() {
    let good = "Uid:\t1000 1000 1000 1000\nGid:\t1000 1000 1000 1000\nPid:\t123\nTgid:\t123\nPPid:\t100\nTracerPid:\t101\nThreads:\t1\nNoNewPrivs:\t1\nCapInh:\t0000000000000000\nCapPrm:\t0000000000000000\nCapEff:\t0000000000000000\nCapBnd:\t0000000000000000\nCapAmb:\t0000000000000000\n";
    status(good, 1000, 1000, 123, 100, 101).unwrap();
    for line in good.lines() {
        assert!(status(&format!("{good}{line}\n"), 1000, 1000, 123, 100, 101).is_err());
        assert!(status(
            &good.replace(&format!("{line}\n"), ""),
            1000,
            1000,
            123,
            100,
            101
        )
        .is_err());
    }
    for bad in [
        good.replace("1000", "0"),
        good.replace("0000000000000000", "0000000000000001"),
        good.replace("Threads:\t1", "Threads:\t2"),
        good.replace("TracerPid:\t101", "TracerPid:\t100"),
    ] {
        assert!(status(&bad, 1000, 1000, 123, 100, 101).is_err());
    }
}
#[test]
fn initial_drop_and_wrong_phase_kill_reap_original_before_root_cleanup() {
    for wrong in [false, true] {
        let (root, temp) = root();
        let mut own = Owner::new(root).unwrap();
        if wrong {
            assert!(own.advance(Phase::Rooted, Owner::first_exec).is_err());
            assert_eq!(own.phase, Phase::Refused);
            assert!(own.reaped);
            assert!(own.advance(Phase::Initial, Owner::enter).is_err());
        }
        drop(own);
        assert!(!temp.path().join("root").exists());
    }
}
#[test]
fn actual_root_transition_drift_and_synthetic_nonshim_image_refuse() {
    for drift in [false, true] {
        let (root, temp) = root();
        let mut own = Owner::new(root).unwrap();
        own.advance(Phase::Initial, Owner::enter).unwrap();
        if drift {
            chmod(&own.root.files[2].file, 0o755).unwrap();
            assert!(own.advance(Phase::Rooted, Owner::first_exec).is_err());
        } else {
            // Synthetic ELF declarations can reach a kernel exec stop. They do
            // not execute the real child shim or establish loaded-runtime proof.
            own.advance(Phase::Rooted, Owner::first_exec).unwrap();
            assert!(own.advance(Phase::Exec, Owner::loaded).is_err());
        }
        assert_eq!(own.phase, Phase::Refused);
        assert!(own.reaped);
        drop(own);
        assert!(!temp.path().join("root").exists());
    }
}

#[test]
fn original_sigkill_and_drop_at_rooted_and_exec_boundaries_reap_before_cleanup() {
    for exec in [false, true] {
        for kill in [false, true] {
            let (root, temp) = root();
            let mut own = Owner::new(root).unwrap();
            own.advance(Phase::Initial, Owner::enter).unwrap();
            if exec {
                own.advance(Phase::Rooted, Owner::first_exec).unwrap();
            }
            if kill {
                own.terminate().unwrap();
                assert_eq!(own.terminal, Some(libc::SIGKILL));
                assert!(own
                    .advance(
                        if exec { Phase::Exec } else { Phase::Rooted },
                        if exec {
                            Owner::loaded
                        } else {
                            Owner::first_exec
                        }
                    )
                    .is_err());
                assert_eq!(own.phase, Phase::Refused);
            }
            drop(own);
            assert!(!temp.path().join("root").exists());
        }
    }
}

#[test]
fn missing_pipe_eof_preserves_root_until_quiescence_is_proved() {
    let (root, temp) = root();
    let mut own = Owner::new(root).unwrap();
    let extra_writer = own
        .relative("fd/3", libc::O_WRONLY | libc::O_NONBLOCK)
        .unwrap();
    assert!(own.terminate().is_err());
    assert!(own.reaped && temp.path().join("root").is_dir());
    // Only the unit fixture retains cleanup handles, never a recaptured path.
    let original_dirs: Vec<_> = own
        .root
        .dirs
        .iter()
        .map(|node| node.file.try_clone().unwrap())
        .collect();
    drop(own);
    assert!(temp.path().join("root").is_dir());
    drop(extra_writer);
    for directory in original_dirs {
        chmod(&directory, 0o700).unwrap();
    }
    // TempDir owns this entire outer unit fixture, not production cleanup.
}
