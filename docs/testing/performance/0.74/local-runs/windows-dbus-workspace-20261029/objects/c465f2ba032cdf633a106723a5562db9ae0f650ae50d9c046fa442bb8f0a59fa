//! Isolated local cgroup2 roles only; no unit, target process or product.
use super::*;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;

struct Fixture {
    temporary: tempfile::TempDir,
    parent: File,
    name: CString,
}
impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::Builder::new()
            .prefix("hc074-transport-fixture-")
            .tempdir()
            .unwrap();
        let name = CString::new(temporary.path().file_name().unwrap().as_encoded_bytes()).unwrap();
        let parent = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .open("/sys/fs/cgroup")
            .unwrap();
        stamp(&parent).unwrap();
        let path = Path::new("/sys/fs/cgroup").join(name.to_str().unwrap());
        assert_eq!(path.parent(), Some(Path::new("/sys/fs/cgroup")));
        std::fs::create_dir(&path).unwrap();
        Self {
            temporary,
            parent,
            name,
        }
    }
    fn root(&self) -> PinnedRoot {
        PinnedRoot::pin_empty(self.parent.as_fd(), &self.name).unwrap()
    }
    fn claims(&self, root: &PinnedRoot) -> RootBindingClaims {
        RootBindingClaims {
            root_device: root.identity.device,
            root_inode: root.identity.inode,
            ..claims()
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let name = self.temporary.path().file_name().unwrap();
        assert_eq!(name.as_encoded_bytes(), self.name.to_bytes());
        assert!(self
            .name
            .to_bytes()
            .starts_with(b"hc074-transport-fixture-"));
        std::fs::remove_dir(Path::new("/sys/fs/cgroup").join(name)).unwrap();
    }
}
#[test]
#[ignore = "explicit isolated local-root cgroup2 fixture only"]
fn cgroup_root_transport_role_matrix() {
    assert_eq!(
        std::env::var("HC074_ROOT_TRANSPORT_FIXTURE").as_deref(),
        Ok("local-only-v1")
    );
    assert_eq!(unsafe { libc::geteuid() }, 0);
    eprintln!("seed=0x074c2020 scope=local-root-descriptor-transfer armed=false");
    let fixture = Fixture::new();
    let mut root = fixture.root();
    let expected = fixture.claims(&root);
    {
        let (mut sender, mut receiver) = pair(&mut root, expected).unwrap();
        sender.send_once().unwrap();
        let received = receiver.receive_once().unwrap();
        received.inspect_empty().unwrap();
        assert_eq!(
            received.identities,
            [
                sender.root.identity,
                sender.root.kill.identity,
                sender.root.events.identity
            ]
        );
        assert!(sender.send_once().is_err());
        assert!(receiver.receive_once().is_err());
        // No ACK exists; the consumed sender is not reopened by a timeout.
        assert!(sender.consumed && sender.send_once().is_err());
    }
    eprintln!("case=exact-three-roles-one-shot-no-ack pass");
    for order in [[1, 0, 2], [0, 2, 1], [2, 1, 0], [0, 1, 1], [0, 0, 2]] {
        let (sender, mut receiver) = pair(&mut root, expected).unwrap();
        let roles = [
            sender.root.directory.as_raw_fd(),
            sender.root.kill.file.as_raw_fd(),
            sender.root.events.file.as_raw_fd(),
        ];
        send(
            &sender.socket,
            &PacketClaims {
                kind: Kind::Enroll,
                binding: expected,
            }
            .encode()
            .unwrap(),
            &order.map(|i| roles[i]),
        )
        .unwrap();
        assert!(receiver.receive_once().is_err());
        assert!(receiver.receive_once().is_err());
        eprintln!("case=role-substitution-{order:?} pass");
    }
    let other_fixture = Fixture::new();
    let other = other_fixture.root();
    for index in 0..3 {
        let (sender, mut receiver) = pair(&mut root, expected).unwrap();
        let mut roles = [
            sender.root.directory.as_raw_fd(),
            sender.root.kill.file.as_raw_fd(),
            sender.root.events.file.as_raw_fd(),
        ];
        roles[index] = [
            other.directory.as_raw_fd(),
            other.kill.file.as_raw_fd(),
            other.events.file.as_raw_fd(),
        ][index];
        send(
            &sender.socket,
            &PacketClaims {
                kind: Kind::Enroll,
                binding: expected,
            }
            .encode()
            .unwrap(),
            &roles,
        )
        .unwrap();
        assert!(receiver.receive_once().is_err());
        eprintln!("case=foreign-root-role-{index} pass");
    }
    {
        let (mut sender, receiver) = pair(&mut root, expected).unwrap();
        drop(receiver);
        assert!(sender.send_once().is_err());
        assert!(sender.consumed && sender.send_once().is_err());
    }
    eprintln!("case=peer-loss-send-failure-no-retry pass");
    {
        let (mut sender, mut receiver) = pair(&mut root, expected).unwrap();
        sender.send_once().unwrap();
        let path = Path::new("/sys/fs/cgroup").join(fixture.name.to_str().unwrap());
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode | 0o020)).unwrap();
        let result = receiver.receive_once();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        assert!(result.is_err());
        assert!(receiver.receive_once().is_err());
    }
    eprintln!("case=post-send-permission-drift-sticky pass");
    assert!(pair(
        &mut root,
        RootBindingClaims {
            root_inode: expected.root_inode + 1,
            ..expected
        }
    )
    .is_err());
    eprintln!("case=pre-send-root-binding-refused pass");
    eprintln!("cases=12 root_transport_matrix=pass original_admission=false completion=false");
}
