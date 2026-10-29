//! Test-only observation DTO transfer; creates no owner, connection or authority.
use std::io;
use std::os::fd::AsFd;
use zbus6::{
    fdo::ConnectionCredentials,
    wire::{Variant, serialized::Context},
};

pub(super) fn transfer(
    old: ::zbus::fdo::ConnectionCredentials,
) -> io::Result<ConnectionCredentials> {
    // Preserve missing versus empty groups and labels through the typed dict.
    // This is metadata only, never a Message conversion or a receive path.
    let mut fields = std::collections::BTreeMap::new();
    if let Some(value) = old.unix_user_id() {
        fields.insert("UnixUserID", Variant::from(value));
    }
    if let Some(value) = old.unix_group_ids() {
        fields.insert("UnixGroupIDs", Variant::from(value.clone()));
    }
    if let Some(value) = old.process_id() {
        fields.insert("ProcessID", Variant::from(value));
    }
    if let Some(value) = old.windows_sid() {
        fields.insert("WindowsSID", Variant::from(value.as_str()));
    }
    if let Some(value) = old.linux_security_label() {
        fields.insert("LinuxSecurityLabel", Variant::from(value.clone()));
    }
    let bytes = zbus6::wire::to_bytes(Context::new(zbus6::wire::LE, 0), &fields)
        .map_err(io::Error::other)?;
    let (mut current, _) = bytes
        .deserialize::<ConnectionCredentials>()
        .map_err(io::Error::other)?;
    drop(bytes);
    drop(fields);
    if let Some(fd) = old.process_fd() {
        // zbus 5 exposes only a borrow. Retain the same kernel resource through
        // a CLOEXEC duplicate; dropping old closes its original numeric slot.
        let fd = fd.as_fd().try_clone_to_owned()?;
        current = current.set_process_fd(fd.into());
    }
    Ok(current)
}

#[test]
fn negotiated_credential_adapter_keeps_optional_fields_and_process_fd_alias() {
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::MetadataExt;
    for groups in [None, Some(Vec::<u32>::new()), Some(vec![12u32, 12, 34])] {
        let mut fields = std::collections::BTreeMap::new();
        if let Some(value) = &groups {
            fields.insert("UnixGroupIDs", ::zbus::zvariant::Value::from(value.clone()));
        }
        let bytes = ::zbus::zvariant::to_bytes(
            ::zbus::zvariant::serialized::Context::new_dbus(::zbus::zvariant::LE, 0),
            &fields,
        )
        .unwrap();
        let (old, _) = bytes
            .deserialize::<::zbus::fdo::ConnectionCredentials>()
            .unwrap();
        let file = tempfile::tempfile().unwrap();
        let metadata = file.metadata().unwrap();
        let number = file.as_raw_fd();
        let fd: std::os::fd::OwnedFd = file.into();
        let old = old
            .set_unix_user_id(74)
            .set_process_id(75)
            .set_windows_sid("S-1-5-18".into())
            .set_linux_security_label(vec![])
            .set_process_fd(fd.into());
        let current = transfer(old).unwrap();
        assert_eq!(current.unix_group_ids(), groups.as_ref());
        assert_eq!(current.unix_user_id(), Some(74));
        assert_eq!(current.process_id(), Some(75));
        assert_eq!(current.windows_sid().map(String::as_str), Some("S-1-5-18"));
        assert_eq!(current.linux_security_label(), Some(&vec![]));
        let alias = current.process_fd().unwrap().as_raw_fd();
        assert_ne!(alias, number);
        assert_eq!(unsafe { libc::fcntl(number, libc::F_GETFD) }, -1);
        assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::EBADF));
        let mut status: libc::stat = unsafe { std::mem::zeroed() };
        assert_eq!(unsafe { libc::fstat(alias, &mut status) }, 0);
        assert_eq!(status.st_dev, metadata.dev());
        assert_eq!(status.st_ino, metadata.ino());
        assert_eq!(
            unsafe { libc::fcntl(alias, libc::F_GETFD) },
            libc::FD_CLOEXEC
        );
        drop(current);
        assert_eq!(unsafe { libc::fcntl(alias, libc::F_GETFD) }, -1);
        assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::EBADF));
    }
    let current = transfer(::zbus::fdo::ConnectionCredentials::default()).unwrap();
    assert_eq!(current, ConnectionCredentials::default());
}
