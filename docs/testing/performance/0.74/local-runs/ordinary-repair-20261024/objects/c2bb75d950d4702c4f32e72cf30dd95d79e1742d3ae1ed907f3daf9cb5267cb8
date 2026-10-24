//! Explicit activation metadata ingress, never manager or original-owner proof.
use std::ffi::{OsStr, OsString};
use std::io;

pub(super) const ARG: &str = "--systemd-activation-v1";
const NAMES: &str = "release:worker:config:creator:root-a:root-b:alarm:bootstrap";
const KEYS: [&str; 3] = ["LISTEN_PID", "LISTEN_FDS", "LISTEN_FDNAMES"];

fn need(ok: bool) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "held activation environment",
        ))
    }
}
pub(super) fn requested() -> bool {
    let mut args = std::env::args_os().skip(1);
    args.next().as_deref() == Some(OsStr::new(ARG)) && args.next().is_none()
}
fn inspect(environment: &[(OsString, OsString)], pid: u32) -> io::Result<()> {
    need(pid > 1 && environment.len() == 3)?;
    let process = pid.to_string();
    let values = [process.as_str(), "8", NAMES];
    let mut seen = 0u8;
    for (key, value) in environment {
        let at = KEYS.iter().position(|name| key == OsStr::new(name));
        let at = at.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "held activation environment",
            )
        })?;
        need(seen & (1 << at) == 0 && value == OsStr::new(values[at]))?;
        seen |= 1 << at;
    }
    need(seen == 7)
}
pub(super) fn prepare() -> io::Result<()> {
    let environment: Vec<_> = std::env::vars_os().take(4).collect();
    inspect(&environment, std::process::id())?;
    // Fresh dedicated binary, before any thread creation. Refuse a foreign
    // extra thread rather than mutate its shared process environment.
    let tasks = std::fs::read_dir("/proc/self/task")?
        .take(2)
        .collect::<io::Result<Vec<_>>>()?;
    need(tasks.len() == 1)?;
    // Only the three exact validated metadata fields are consumed. Never use
    // env_clear here: unknown input must refuse, not silently disappear.
    for key in KEYS {
        std::env::remove_var(key);
    }
    need(std::env::vars_os().next().is_none())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStringExt;
    fn valid() -> Vec<(OsString, OsString)> {
        KEYS.into_iter()
            .zip(["123", "8", NAMES])
            .map(|(k, v)| (k.into(), v.into()))
            .collect()
    }
    #[test]
    fn activation_ingress_exact_fields_accept_all_orders_without_mutating_input() {
        let expected = valid();
        for order in [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ] {
            let values = order.map(|at| expected[at].clone());
            inspect(&values, 123).unwrap();
            assert_eq!(values, order.map(|at| expected[at].clone()));
        }
    }
    #[test]
    fn activation_ingress_unknown_missing_duplicate_and_noncanonical_fields_refuse() {
        for pid in [0, 1, 122, 124] {
            assert!(inspect(&valid(), pid).is_err());
        }
        for at in 0..3 {
            let mut bad = valid();
            bad.remove(at);
            assert!(inspect(&bad, 123).is_err());
            let mut bad = valid();
            bad.push(bad[at].clone());
            assert!(inspect(&bad, 123).is_err());
            let mut bad = valid();
            bad[at] = bad[(at + 1) % 3].clone();
            assert!(inspect(&bad, 123).is_err());
            for key in [
                OsString::from("PATH"),
                OsString::from("LD_PRELOAD"),
                OsString::from_vec(vec![255]),
            ] {
                let mut bad = valid();
                bad[at].0 = key;
                assert!(inspect(&bad, 123).is_err());
            }
            for value in [
                OsString::new(),
                OsString::from_vec(vec![255]),
                OsString::from(" 123"),
                OsString::from("+123"),
                OsString::from("0123"),
            ] {
                let mut bad = valid();
                bad[at].1 = value;
                assert!(inspect(&bad, 123).is_err());
            }
        }
        let mut bad = valid();
        bad.push(("OTHER".into(), "".into()));
        assert!(inspect(&bad, 123).is_err());
    }
    #[test]
    fn activation_ingress_every_name_byte_and_descriptor_count_are_exact() {
        for at in 0..NAMES.len() {
            let mut bytes = NAMES.as_bytes().to_vec();
            bytes[at] ^= 1;
            let mut bad = valid();
            bad[2].1 = OsString::from_vec(bytes);
            assert!(inspect(&bad, 123).is_err());
        }
        for count in ["0", "7", "9", "08", "+8", "8 "] {
            let mut bad = valid();
            bad[1].1 = count.into();
            assert!(inspect(&bad, 123).is_err());
        }
    }
}
