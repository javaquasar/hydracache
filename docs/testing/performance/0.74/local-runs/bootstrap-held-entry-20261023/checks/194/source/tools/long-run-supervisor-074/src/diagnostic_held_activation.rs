//! Strict activation assertions; these values do not enroll an authority.
use std::{ffi::OsString, io};
pub(crate) const ARG: &str = "--systemd-bootstrap-held-readiness-v1";
pub(crate) const NAMES: &str = "release:worker:config:creator:root-a:root-b:alarm:bootstrap:host:launcher:bootstrap-image:entry:installed-image:installed-entry:readiness";
#[cfg_attr(not(test), allow(dead_code))] // Parent wire uses this finite manager-side list.
pub(crate) const UNSET: &[&str] = &[
    "PATH", "USER", "LOGNAME", "HOME", "SHELL", "LANG", "LANGUAGE", "LC_ALL",
    "LC_CTYPE", "LC_NUMERIC", "LC_TIME", "LC_COLLATE", "LC_MONETARY", "LC_MESSAGES",
    "LC_PAPER", "LC_NAME", "LC_ADDRESS", "LC_TELEPHONE", "LC_MEASUREMENT", "LC_IDENTIFICATION",
];
#[cfg_attr(not(test), allow(dead_code))] // Also compiled by the parent wire contract.
pub(crate) struct Checked {
    pub(crate) invocation: [u8; 16],
}
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn inspect(args: &[OsString], env: &[(OsString, OsString)], pid: u32) -> io::Result<Checked> {
    let denied = || io::Error::new(io::ErrorKind::PermissionDenied, "held activation environment");
    if pid <= 1 || args != [OsString::from(ARG)] || env.len() != 5 {
        return Err(denied());
    }
    let keys = ["LISTEN_PID", "LISTEN_FDS", "LISTEN_FDNAMES", "INVOCATION_ID", "SYSTEMD_EXEC_PID"];
    if env.iter().any(|(k, _)| !keys.iter().any(|v| k == v)) {
        return Err(denied());
    }
    let mut values = Vec::with_capacity(5);
    for key in keys {
        let mut found = env.iter().filter(|(k, _)| k == key);
        let value = found.next().and_then(|(_, v)| v.to_str()).ok_or_else(denied)?;
        if found.next().is_some() { return Err(denied()); }
        values.push(value);
    }
    if values[0] != pid.to_string() || values[1] != "15" || values[2] != NAMES || values[4] != pid.to_string() {
        return Err(denied());
    }
    let raw = values[3].as_bytes();
    if raw.len() != 32 || !raw.iter().all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(v)) {
        return Err(denied());
    }
    let digit = |v: u8| if v <= b'9' { v - b'0' } else { v - b'a' + 10 };
    let invocation = std::array::from_fn(|i| digit(raw[2*i]) * 16 + digit(raw[2*i+1]));
    if invocation == [0; 16] { return Err(denied()); }
    Ok(Checked { invocation })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn environment() -> Vec<(OsString, OsString)> {
        [ ("LISTEN_PID", "74"), ("LISTEN_FDS", "15"), ("LISTEN_FDNAMES", NAMES),
          ("INVOCATION_ID", "0102030405060708090a0b0c0d0e0f10"), ("SYSTEMD_EXEC_PID", "74") ]
            .map(|(k,v)| (k.into(), v.into())).into()
    }
    #[test]
    fn held_entry_activation_exact_and_permutation() {
        let args = [ARG.into()];
        let mut env = environment();
        assert_eq!(inspect(&args, &env, 74).unwrap().invocation[15], 16);
        env.reverse();
        assert!(inspect(&args, &env, 74).is_ok());
        assert!(UNSET.iter().all(|k| !k.starts_with("LISTEN_") && *k != "INVOCATION_ID" && *k != "SYSTEMD_EXEC_PID"));
    }
    #[test]
    fn held_entry_activation_missing_duplicate_foreign_and_noncanonical_refuse() {
        let args = [ARG.into()];
        for (slot, values) in [ (0, vec!["074", "75", "", "+74"]), (1, vec!["14", "015", "16"]),
            (2, vec!["", "readiness"]), (3, vec!["00000000000000000000000000000000", "0102030405060708090A0b0c0d0e0f10", "1"]), (4, vec!["75", "074"]) ] {
            for v in values { let mut env = environment(); env[slot].1 = v.into(); assert!(inspect(&args, &env, 74).is_err()); }
        }
        for slot in 0..5 {
            let mut env = environment(); env.remove(slot); assert!(inspect(&args, &env, 74).is_err());
            let mut env = environment(); env[slot].0 = "UNKNOWN".into(); assert!(inspect(&args, &env, 74).is_err());
            let mut env = environment(); env[slot] = env[(slot+1)%5].clone(); assert!(inspect(&args, &env, 74).is_err());
        }
        assert!(inspect(&[], &environment(), 74).is_err());
        assert!(inspect(&[ARG.into(), "x".into()], &environment(), 74).is_err());
        assert!(inspect(&args, &environment(), 1).is_err());
    }
}
