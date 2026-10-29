//! Root-only temporary test origin. Never a production trust or root admission.
use std::os::unix::fs::{MetadataExt, PermissionsExt};
thread_local! {
    static SCOPES: std::cell::RefCell<Vec<serde_json::Value>> = const {
        std::cell::RefCell::new(Vec::new())
    };
}
pub(crate) fn identity(path: &std::path::Path) -> serde_json::Value {
    let info = std::fs::symlink_metadata(path).unwrap();
    serde_json::json!({"path":path,"device":info.dev(),"inode":info.ino()})
}
pub(crate) fn scopes() -> Vec<serde_json::Value> {
    SCOPES.with(|scopes| scopes.borrow().clone())
}
pub(crate) fn require(path: &std::path::Path) {
    assert_eq!(
        std::env::var("HC074_OWNED_ROOT_FIXTURE").as_deref(),
        Ok("local-only-v1")
    );
    assert_eq!(
        (unsafe { libc::geteuid() }, unsafe { libc::getegid() }),
        (0, 0)
    );
    assert_eq!(path.parent(), Some(std::path::Path::new("/tmp")));
    assert!(path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("hc074-owned-root-"));
    assert_eq!(std::fs::canonicalize(path).unwrap(), path);
    let metadata = std::fs::symlink_metadata(path).unwrap();
    assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
    assert_eq!(
        (metadata.uid(), metadata.gid(), metadata.mode() & 0o7777),
        (0, 0, 0o700)
    );
}
pub(crate) fn new() -> tempfile::TempDir {
    let root = tempfile::Builder::new()
        .prefix("hc074-owned-root-")
        .tempdir_in("/tmp")
        .unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    require(root.path());
    SCOPES.with(|scopes| scopes.borrow_mut().push(identity(root.path())));
    root
}
