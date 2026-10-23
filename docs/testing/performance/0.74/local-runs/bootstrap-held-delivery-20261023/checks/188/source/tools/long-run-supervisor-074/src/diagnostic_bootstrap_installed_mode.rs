//! Explicit refusal-only mode. Activation metadata is not sender authority.
use std::{
    ffi::{OsStr, OsString},
    io,
};
pub(super) const ARG: &str = "--systemd-bootstrap-installed-v1";
pub(super) const NAMES: &str = "release:worker:config:creator:root-a:root-b:alarm:bootstrap:host:launcher:bootstrap-image:entry:installed-image:installed-entry";
pub(super) fn requested() -> bool {
    std::env::args_os().nth(1).as_deref() == Some(OsStr::new(ARG))
}
fn inspect(args: &[OsString], env: &[(OsString, OsString)], pid: u32) -> io::Result<()> {
    let refused = || {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            "installed bootstrap activation assertions",
        )
    };
    if args.len() != 1 || args[0] != OsStr::new(ARG) || pid <= 1 || env.len() != 3 {
        return Err(refused());
    }
    let pid = pid.to_string();
    let values = [pid.as_str(), "14", NAMES];
    let keys = ["LISTEN_PID", "LISTEN_FDS", "LISTEN_FDNAMES"];
    let mut seen = 0;
    for (key, value) in env {
        let at = keys
            .iter()
            .position(|k| key == OsStr::new(k))
            .ok_or_else(refused)?;
        if seen & (1 << at) != 0 || value != OsStr::new(values[at]) {
            return Err(refused());
        }
        seen |= 1 << at;
    }
    if seen != 7 {
        return Err(refused());
    }
    Ok(())
}
pub(super) fn run() -> io::Result<()> {
    inspect(
        &std::env::args_os().skip(1).take(2).collect::<Vec<_>>(),
        &std::env::vars_os().take(4).collect::<Vec<_>>(),
        std::process::id(),
    )?;
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        let observed = super::reader::roles::installed::observe()?;
        super::reader::roles::installed::revalidate(&observed)?;
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "original bootstrap authentication is closed",
        ))
    }
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Linux x86-64 required",
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn env() -> Vec<(OsString, OsString)> {
        ["LISTEN_PID", "LISTEN_FDS", "LISTEN_FDNAMES"]
            .into_iter()
            .zip(["74", "14", NAMES])
            .map(|(k, v)| (k.into(), v.into()))
            .collect()
    }
    #[test]
    fn bootstrap_installed_mode_exact_activation_and_old_name_prefix() {
        assert_eq!(
            NAMES
                .strip_suffix(":installed-image:installed-entry")
                .unwrap(),
            super::super::ingress::NAMES
        );
        for order in [[0, 1, 2], [2, 1, 0], [1, 0, 2]] {
            inspect(&[ARG.into()], &order.map(|i| env()[i].clone()), 74).unwrap();
        }
        for args in [
            vec![],
            vec!["--systemd-bootstrap-installed-v2".into()],
            vec![super::super::ingress::ARG.into()],
            vec![ARG.into(), "extra".into()],
        ] {
            assert!(inspect(&args, &env(), 74).is_err());
        }
    }
    #[test]
    fn bootstrap_installed_mode_refuses_pid_count_names_duplicate_and_foreign_environment() {
        for pid in [0, 1, 73, 75] {
            assert!(inspect(&[ARG.into()], &env(), pid).is_err());
        }
        for at in 0..3 {
            for value in [
                "",
                "074",
                "+74",
                "74 ",
                "12",
                "15",
                super::super::ingress::NAMES,
            ] {
                let mut bad = env();
                bad[at].1 = value.into();
                assert!(inspect(&[ARG.into()], &bad, 74).is_err());
            }
            let mut bad = env();
            bad[at] = bad[(at + 1) % 3].clone();
            assert!(inspect(&[ARG.into()], &bad, 74).is_err());
        }
        let mut bad = env();
        bad.push(("LD_PRELOAD".into(), "".into()));
        assert!(inspect(&[ARG.into()], &bad, 74).is_err());
        for at in 0..NAMES.len() {
            let mut bad = env();
            let mut bytes = NAMES.as_bytes().to_vec();
            bytes[at] ^= 1;
            bad[2].1 = String::from_utf8(bytes).unwrap().into();
            assert!(inspect(&[ARG.into()], &bad, 74).is_err());
        }
    }
}
