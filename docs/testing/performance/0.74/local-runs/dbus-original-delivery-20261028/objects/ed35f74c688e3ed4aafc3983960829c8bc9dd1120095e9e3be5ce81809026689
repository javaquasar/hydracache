use super::*;
use std::os::unix::fs::PermissionsExt;

fn facts() -> Facts {
    fn s(device: u64, inode: u64) -> Stamp {
        Stamp {
            device,
            inode,
            uid: 0,
            gid: 0,
            mode: libc::S_IFDIR | 0o755,
        }
    }
    Facts {
        anchors: [s(1, 1), s(2, 2), s(2, 3), s(3, 4)],
        system: s(3, 5),
        named_root: s(3, 6),
        retained_parent: s(3, 5),
        retained_root: s(3, 6),
        name_matches: true,
    }
}
#[test]
fn fixed_slice_name_is_exact_seeded_and_directly_below_system_slice() {
    let mut seed = 0x0745_11ce_u64;
    for _ in 0..128 {
        let mut attempt = [0; 32];
        for byte in &mut attempt {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            *byte = seed as u8;
        }
        let actual = name(attempt);
        let raw = actual.to_bytes();
        assert_eq!(raw.len(), 82);
        assert!(raw.starts_with(b"system-hc074") && raw.ends_with(b".slice"));
        assert_eq!(raw.iter().filter(|b| **b == b'-').count(), 1);
        let hex: String = attempt.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(actual.to_str().unwrap(), format!("system-hc074{hex}.slice"));
    }
    println!("HC074-FIXED-SLICE seed=0x074511ce identities=128 send=false");
}
#[test]
fn fixed_slice_ancestry_requires_same_original_objects_and_not_just_name() {
    let good = facts();
    binding(&good, good.retained_root).unwrap();
    for case in 0..8 {
        let mut bad = good;
        match case {
            0 => bad.name_matches = false,
            1 => bad.retained_parent.inode += 1,
            2 => bad.named_root.inode += 1,
            3 => bad.retained_root.inode += 1,
            4 => bad.system.device += 1,
            5 => bad.anchors[3].device += 1,
            6 => {
                bad.system = good.retained_root;
                bad.retained_parent = bad.system;
            }
            _ => {
                bad.system = good.anchors[3];
                bad.retained_parent = bad.system;
            }
        }
        assert!(binding(&bad, good.retained_root).is_err(), "{case}");
    }
}
#[test]
fn fixed_slice_retained_snapshot_detects_every_anchor_identity_security_change() {
    let good = facts();
    unchanged(&good, &good).unwrap();
    for object in 0..8 {
        for field in 0..5 {
            let mut bad = good;
            let stamp = match object {
                0..=3 => &mut bad.anchors[object],
                4 => &mut bad.system,
                5 => &mut bad.named_root,
                6 => &mut bad.retained_parent,
                _ => &mut bad.retained_root,
            };
            match field {
                0 => stamp.device += 1,
                1 => stamp.inode += 1,
                2 => stamp.uid += 1,
                3 => stamp.gid += 1,
                _ => stamp.mode ^= 0o020,
            }
            assert!(unchanged(&bad, &good).is_err(), "{object} {field}");
        }
    }
}
#[test]
fn fixed_slice_no_symlink_unsafe_component_or_regular_filesystem_substitute() {
    let tmp = tempfile::tempdir().unwrap();
    let parent = File::open(tmp.path()).unwrap();
    assert!(stamp(&parent).is_err());
    let regular = tempfile::tempfile().unwrap();
    assert!(directory_stamp(&regular).is_err());
    std::fs::create_dir(tmp.path().join("owned")).unwrap();
    std::os::unix::fs::symlink(tmp.path().join("owned"), tmp.path().join("alias")).unwrap();
    assert!(child(parent.as_fd(), c"alias", true, false).is_err());
    for bad in [
        c"",
        c".",
        c"..",
        c"../system.slice",
        c"/system.slice",
        c"a/b",
    ] {
        assert!(child(parent.as_fd(), bad, true, false).is_err());
    }
    std::fs::set_permissions(
        tmp.path().join("owned"),
        std::fs::Permissions::from_mode(0o777),
    )
    .unwrap();
    let writable = child(parent.as_fd(), c"owned", true, false).unwrap();
    assert!(directory_stamp(&writable).is_err());
}
#[test]
#[ignore = "manual read-only local cgroup2 anchor check; no root creation or admission"]
fn privileged_fixed_slice_readonly_anchors() {
    let retained = anchors().unwrap();
    assert_eq!(
        identities(&retained).unwrap(),
        identities(&anchors().unwrap()).unwrap()
    );
    let system = child(retained[3].as_fd(), c"system.slice", true, false).unwrap();
    let original = stamp(&system).unwrap();
    assert_eq!(original.device, stamp(&retained[3]).unwrap().device);
    assert_ne!(original.inode, stamp(&retained[3]).unwrap().inode);
    let named = child(retained[3].as_fd(), c"system.slice", true, false).unwrap();
    assert_eq!(stamp(&named).unwrap(), original);
    for file in retained.iter().chain([&system, &named]) {
        let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
        assert!(flags >= 0);
        assert_eq!(flags & libc::O_ACCMODE, libc::O_RDONLY);
        assert_ne!(
            unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
            0
        );
    }
    println!("HC074-FIXED-SLICE-ANCHORS device={} inode={} readonly=true fixed_root_admitted=false original_send=false server=false product=false",original.device,original.inode);
}

#[path = "../tests/support/fixed_slice_namespace.rs"]
mod namespace;
