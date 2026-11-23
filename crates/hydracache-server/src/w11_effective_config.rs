//! One non-secret startup observation for the isolated W11 verifier.

use std::ffi::OsStr;
use std::io;
use std::os::unix::net::UnixDatagram;

const ENABLE: &str = "HYDRACACHE_W11_CONFIG_READBACK";
const PROFILE: &str = "HYDRACACHE_CLIENT_BATCH_PROFILE";
const SOCKET: &str = "w11-effective-config.sock";

#[derive(Clone, Copy)]
pub(crate) enum Profile {
    Default,
    W11,
}

pub(crate) struct Limits {
    pub resp_array_items: usize,
    pub resp_frame_bytes: usize,
    pub shared_batch_entries: usize,
    pub shared_batch_payload_bytes: usize,
    pub shared_value_bytes: usize,
}

pub(crate) fn requested() -> io::Result<Option<Profile>> {
    match std::env::var_os(ENABLE).as_deref() {
        None => return Ok(None),
        Some(value) if value == OsStr::new("1") => {}
        Some(_) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid W11 config readback setting",
            ));
        }
    }
    match std::env::var_os(PROFILE).as_deref() {
        None => Ok(Some(Profile::Default)),
        Some(value) if value == OsStr::new("w11-256") => Ok(Some(Profile::W11)),
        Some(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid W11 config readback profile",
        )),
    }
}

fn encode(profile: Profile, limits: &Limits) -> String {
    let selector = match profile {
        Profile::Default => "null",
        Profile::W11 => r#""w11-256""#,
    };
    format!(
        concat!(
            r#"{{"resp_array_items":{},"resp_frame_bytes":{},"selector":{},"#,
            r#""shared_batch_entries":{},"shared_batch_payload_bytes":{},"shared_value_bytes":{}}}"#,
        ),
        limits.resp_array_items,
        limits.resp_frame_bytes,
        selector,
        limits.shared_batch_entries,
        limits.shared_batch_payload_bytes,
        limits.shared_value_bytes,
    )
}

pub(crate) fn publish(profile: Profile, limits: Limits) -> io::Result<()> {
    let document = encode(profile, &limits);
    let socket = UnixDatagram::unbound()?;
    socket.set_nonblocking(true)?;
    if socket.send_to(document.as_bytes(), SOCKET)? != document.len() {
        return Err(io::Error::new(
            io::ErrorKind::WriteZero,
            "incomplete W11 config readback",
        ));
    }
    Ok(())
}
