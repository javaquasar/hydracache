//! Activation metadata assertions, not sender or descriptor authentication.
use std::ffi::{OsStr, OsString};
use std::io;
pub(super) const ARG: &str = "--systemd-bootstrap-entry-v1";
pub(super) const NAMES: &str = "release:worker:config:creator:root-a:root-b:alarm:bootstrap:host:launcher:bootstrap-image:entry";
const KEYS: [&str; 3] = ["LISTEN_PID", "LISTEN_FDS", "LISTEN_FDNAMES"];
pub(super) fn inspect(args: &[OsString], env: &[(OsString, OsString)], pid: u32) -> io::Result<()> {
    let refused = || {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            "bootstrap activation assertions",
        )
    };
    if args.len() != 1 || args[0] != OsStr::new(ARG) || pid <= 1 || env.len() != 3 {
        return Err(refused());
    }
    let text = pid.to_string();
    let values = [text.as_str(), "12", NAMES];
    let mut seen = 0;
    for (key, value) in env {
        let at = KEYS
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

#[cfg(test)]
mod tests {
    use super::*;
    fn valid() -> Vec<(OsString, OsString)> {
        KEYS.into_iter()
            .zip(["74", "12", NAMES])
            .map(|(k, v)| (k.into(), v.into()))
            .collect()
    }
    #[test]
    fn bootstrap_ingress_exact_metadata_permutations_are_only_assertions() {
        let raw = valid();
        for order in [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ] {
            let values = order.map(|i| raw[i].clone());
            inspect(&[ARG.into()], &values, 74).unwrap();
            assert_eq!(values, order.map(|i| raw[i].clone()));
        }
    }
    #[test]
    fn bootstrap_ingress_bad_argument_pid_unknown_duplicate_and_noncanonical_values_refuse() {
        for args in [
            vec![],
            vec!["--systemd-bootstrap-entry-v2".into()],
            vec![ARG.into(), "extra".into()],
        ] {
            assert!(inspect(&args, &valid(), 74).is_err());
        }
        for pid in [0, 1, 73, 75] {
            assert!(inspect(&[ARG.into()], &valid(), pid).is_err());
        }
        for at in 0..3 {
            let mut bad = valid();
            bad.remove(at);
            assert!(inspect(&[ARG.into()], &bad, 74).is_err());
            let mut bad = valid();
            bad.push(bad[at].clone());
            assert!(inspect(&[ARG.into()], &bad, 74).is_err());
            let mut bad = valid();
            bad[at] = bad[(at + 1) % 3].clone();
            assert!(inspect(&[ARG.into()], &bad, 74).is_err());
            for v in ["", "074", "+74", "74 ", "11", "13"] {
                let mut bad = valid();
                bad[at].1 = v.into();
                assert!(inspect(&[ARG.into()], &bad, 74).is_err());
            }
        }
        for key in ["PATH", "LD_PRELOAD", "RUST_LOG", "INVOCATION_ID"] {
            let mut bad = valid();
            bad.push((key.into(), "".into()));
            assert!(inspect(&[ARG.into()], &bad, 74).is_err());
        }
    }
    #[test]
    fn bootstrap_ingress_every_activation_name_byte_refuses_mutation() {
        for at in 0..NAMES.len() {
            let mut bytes = NAMES.as_bytes().to_vec();
            bytes[at] ^= 1;
            let mut bad = valid();
            bad[2].1 = String::from_utf8(bytes).unwrap().into();
            assert!(inspect(&[ARG.into()], &bad, 74).is_err());
        }
    }
}
