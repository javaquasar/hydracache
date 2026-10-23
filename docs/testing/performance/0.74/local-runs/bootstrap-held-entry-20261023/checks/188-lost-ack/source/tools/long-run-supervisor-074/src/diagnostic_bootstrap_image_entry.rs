//! Explicit image-consistency mode; all routes still refuse authentication.
use std::ffi::{OsStr, OsString};
use std::io;
pub(super) const ARG: &str = "--systemd-bootstrap-image-v1";
pub(super) fn requested() -> bool {
    std::env::args_os().nth(1).as_deref() == Some(OsStr::new(ARG))
}
fn inspect(args: &[OsString], env: &[(OsString, OsString)], pid: u32) -> io::Result<()> {
    if args.len() != 1 || args[0] != OsStr::new(ARG) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "bootstrap image argument",
        ));
    }
    // Validate the unchanged assertions only, not an old-mode execution fallback.
    super::ingress::inspect(&[super::ingress::ARG.into()], env, pid)
}
pub(super) fn run() -> io::Result<()> {
    inspect(
        &std::env::args_os().skip(1).take(2).collect::<Vec<_>>(),
        &std::env::vars_os().take(4).collect::<Vec<_>>(),
        std::process::id(),
    )?;
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        let entry = super::reader::read(14)?;
        let roles = super::reader::roles::observe(&entry)?;
        let image = super::reader::current_image::observe(&entry)?;
        super::reader::roles::revalidate(&entry, &roles)?;
        super::reader::current_image::revalidate(&entry, &image)?;
    }
    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        "original bootstrap authentication is closed",
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn env() -> Vec<(OsString, OsString)> {
        vec![
            ("LISTEN_PID".into(), "74".into()),
            ("LISTEN_FDS".into(), "12".into()),
            ("LISTEN_FDNAMES".into(), super::super::ingress::NAMES.into()),
        ]
    }
    #[test]
    fn image_mode_requires_explicit_argument_and_unmodified_activation_assertions() {
        inspect(&[ARG.into()], &env(), 74).unwrap();
        for args in [
            vec![],
            vec![super::super::ingress::ARG.into()],
            vec![super::super::census_entry::ARG.into()],
            vec![ARG.into(), "extra".into()],
        ] {
            assert!(inspect(&args, &env(), 74).is_err());
        }
        for at in 0..3 {
            let mut values = env();
            values[at].1 = "wrong".into();
            assert!(inspect(&[ARG.into()], &values, 74).is_err());
        }
    }
    #[test]
    fn image_mode_cannot_refresh_pid_or_ignore_additional_environment() {
        for pid in [0, 1, 73, 75] {
            assert!(inspect(&[ARG.into()], &env(), pid).is_err());
        }
        let mut values = env();
        values.push(("UNKNOWN".into(), "".into()));
        assert!(inspect(&[ARG.into()], &values, 74).is_err());
    }
}
