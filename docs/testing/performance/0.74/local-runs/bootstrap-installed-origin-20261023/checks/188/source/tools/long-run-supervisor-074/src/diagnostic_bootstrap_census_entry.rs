//! A separate refusal-only mode; never a retry through entry-only ingress.
use std::ffi::{OsStr, OsString};
use std::io;
pub(super) const ARG: &str = "--systemd-bootstrap-census-v1";
pub(super) fn requested() -> bool {
    std::env::args_os().nth(1).as_deref() == Some(OsStr::new(ARG))
}
fn inspect(args: &[OsString], env: &[(OsString, OsString)], pid: u32) -> io::Result<()> {
    if args.len() != 1 || args[0] != OsStr::new(ARG) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "bootstrap census argument",
        ));
    }
    // Reuse ONLY the exact activation-assertion validator, never the old reader
    // execution route. The new mode has already been separately selected above.
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
        let census = super::reader::roles::observe(&entry)?;
        super::reader::roles::revalidate(&entry, &census)?;
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
    fn census_ingress_only_explicit_new_mode_with_unchanged_metadata() {
        inspect(&[ARG.into()], &env(), 74).unwrap();
        for args in [
            vec![],
            vec![super::super::ingress::ARG.into()],
            vec![ARG.into(), "extra".into()],
        ] {
            assert!(inspect(&args, &env(), 74).is_err());
        }
    }
    #[test]
    fn census_ingress_cannot_repair_unknown_pid_or_environment() {
        for pid in [0, 1, 73, 75] {
            assert!(inspect(&[ARG.into()], &env(), pid).is_err());
        }
        for at in 0..3 {
            let mut values = env();
            values[at].1 = "wrong".into();
            assert!(inspect(&[ARG.into()], &values, 74).is_err());
        }
        let mut values = env();
        values.push(("UNKNOWN".into(), "".into()));
        assert!(inspect(&[ARG.into()], &values, 74).is_err());
    }
}
