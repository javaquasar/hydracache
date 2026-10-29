use super::*;
use std::fs::OpenOptions;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

#[test]
fn cgroup_root_regular_files_and_unsafe_components_cannot_be_kernel_roots() {
    let temporary = tempfile::tempdir().unwrap();
    let parent = File::open(temporary.path()).unwrap();
    assert!(PinnedRoot::pin_empty(parent.as_fd(), c"root").is_err());
    std::os::unix::fs::symlink(temporary.path(), temporary.path().join("alias")).unwrap();
    assert!(child(parent.as_fd(), c"alias", true, false).is_err());
    for name in [c"", c".", c"..", c"../root", c"/root", c"a/b"] {
        assert!(child(parent.as_fd(), name, true, false).is_err());
    }
    let long = CString::new(vec![b'x'; 129]).unwrap();
    assert!(child(parent.as_fd(), &long, true, false).is_err());
}
#[test]
fn cgroup_root_events_are_exact_bounded_and_never_a_completion_receipt() {
    for (p, f) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        assert_eq!(
            events(format!("populated {p}\nfrozen {f}\n").as_bytes()).unwrap(),
            Observation {
                populated: p == 1,
                frozen: f == 1
            }
        );
    }
    for bad in [
        b"".as_slice(),
        b"populated 0\nfrozen 0",
        b"populated 2\nfrozen 0\n",
        b"populated 0\npopulated 0\nfrozen 0\n",
        b"populated 0\nfrozen 0\nother 0\n",
        b"populated 0\nfrozen 0\n\n",
        b"populated 0\n",
        b"frozen 0\n",
    ] {
        assert!(events(bad).is_err());
    }
    let regular = tempfile::tempfile().unwrap();
    regular.write_at(&[b'x'; 4097], 0).unwrap();
    assert!(bounded(&regular).is_err());
}

pub(super) fn checkpoint(stage: u8) {
    let selected = std::env::var("HC074_ROOT_CUT")
        .ok()
        .and_then(|v| v.parse::<u8>().ok());
    if selected != Some(stage) {
        return;
    }
    let socket = std::os::unix::net::UnixDatagram::unbound().unwrap();
    socket
        .connect(std::env::var("HC074_ROOT_NOTICE").unwrap())
        .unwrap();
    assert_eq!(socket.send(&[stage]).unwrap(), 1);
    // The exact owning parent SIGKILLs this process, not an injected Rust error.
    let mut byte = [0];
    std::io::Read::read_exact(&mut std::io::stdin(), &mut byte).unwrap();
    panic!("checkpoint must never be released");
}

#[path = "diagnostic_cgroup_root_fault_tests.rs"]
mod fault;
