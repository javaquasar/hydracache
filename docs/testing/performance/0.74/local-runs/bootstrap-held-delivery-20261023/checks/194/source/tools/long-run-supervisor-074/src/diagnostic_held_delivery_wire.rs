//! Held wire mechanics, not a start permit. Only the consuming owner binds trust.
use super::*;
use crate::systemd_unit::UnitProperty;
use std::{
    io,
    num::NonZeroU32,
    os::fd::{AsRawFd, BorrowedFd},
};
use zbus::{
    message::{Message, Type},
    zvariant::{Fd, OwnedObjectPath, Value},
};
#[path = "diagnostic_held_child_exchange.rs"]
pub(crate) mod child_exchange;
pub(crate) const ARG: &str = "--systemd-bootstrap-held-readiness-v1";
const IMAGE: &str = "/opt/hydracache-perf/bin/diagnostic-held-bootstrap-074";
const NAMES: [&str; 15] = [
    "release",
    "worker",
    "config",
    "creator",
    "root-a",
    "root-b",
    "alarm",
    "bootstrap",
    "host",
    "launcher",
    "bootstrap-image",
    "entry",
    "installed-image",
    "installed-entry",
    "readiness",
];
const PATH: &str = "/org/freedesktop/systemd1";
const INTERFACE: &str = "org.freedesktop.systemd1.Manager";
type Body<'a> = (
    &'a str,
    &'a str,
    Vec<(&'a str, Value<'a>)>,
    Vec<(&'a str, Vec<(&'a str, Value<'a>)>)>,
);
fn denied() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "held delivery binding")
}
fn check(ok: bool) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(denied())
    }
}
fn name(v: &str) -> bool {
    v.strip_prefix(':').is_some_and(|r| {
        r.len() <= 64
            && r.split('.').count() == 2
            && r.split('.')
                .all(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
    })
}
#[derive(Debug, PartialEq, Eq)]
struct Stamp {
    metadata: [u64; 11],
    flags: [i32; 2],
    offset: Option<i64>,
}
fn snapshot(fd: i32) -> io::Result<Stamp> {
    let mut s: libc::stat = unsafe { std::mem::zeroed() };
    check(fd >= 0 && unsafe { libc::fstat(fd, &mut s) } == 0)?;
    let flags = [unsafe { libc::fcntl(fd, libc::F_GETFD) }, unsafe {
        libc::fcntl(fd, libc::F_GETFL)
    }];
    check(flags[0] == libc::FD_CLOEXEC && flags[1] >= 0 && flags[1] & libc::O_PATH == 0)?;
    let offset = if s.st_mode & libc::S_IFMT == libc::S_IFREG {
        let p = unsafe { libc::lseek(fd, 0, libc::SEEK_CUR) };
        check(p >= 0)?;
        Some(p)
    } else {
        None
    };
    Ok(Stamp {
        metadata: [
            s.st_dev,
            s.st_ino,
            u64::from(s.st_uid),
            u64::from(s.st_gid),
            u64::from(s.st_mode),
            s.st_nlink,
            s.st_size as u64,
            s.st_mtime as u64,
            s.st_mtime_nsec as u64,
            s.st_ctime as u64,
            s.st_ctime_nsec as u64,
        ],
        flags,
        offset,
    })
}
fn value(p: &UnitProperty) -> Value<'_> {
    match p {
        UnitProperty::Text(v) => Value::from(v),
        UnitProperty::Boolean(v) => Value::from(*v),
        UnitProperty::Unsigned(v) => Value::from(*v),
        UnitProperty::Bytes(v) => Value::from(v),
        UnitProperty::Strings(v) => Value::from(v),
        UnitProperty::Commands(v) => Value::from(
            v.iter()
                .map(|c| {
                    (
                        c.path.as_str(),
                        c.argv.iter().map(String::as_str).collect::<Vec<_>>(),
                        c.ignore_failure,
                    )
                })
                .collect::<Vec<_>>(),
        ),
    }
}
#[derive(Debug, PartialEq, Eq)]
enum Phase {
    Prepared,
    Uncertain,
    JobObserved,
    Refused,
}
pub(crate) struct HeldRequest {
    message: Message,
    original: [i32; 18],
    aliases: [i32; 18],
    stamps: Vec<Stamp>,
    body: Vec<u8>,
    manager: String,
    client: String,
    serial: NonZeroU32,
    phase: Phase,
}
impl HeldRequest {
    pub(crate) fn inspect(&self) -> io::Result<()> {
        check(self.phase != Phase::Refused)?;
        let h = self.message.header();
        check(
            self.message.message_type() == Type::MethodCall
                && self.message.primary_header().flags().is_empty()
                && self.message.primary_header().serial_num() == self.serial
                && self.message.data().bytes().len() <= crate::protocol::MAX_PACKET_BYTES,
        )?;
        check(
            h.destination().map(|v| v.as_str()) == Some(self.manager.as_str())
                && h.path().map(|v| v.as_str()) == Some(PATH)
                && h.interface().map(|v| v.as_str()) == Some(INTERFACE)
                && h.member().map(|v| v.as_str()) == Some("StartTransientUnit")
                && h.sender().is_none()
                && h.reply_serial().is_none()
                && h.error_name().is_none(),
        )?;
        check(
            h.unix_fds() == Some(18)
                && self.message.data().fds().len() == 18
                && self.message.body().signature().to_string() == "(ssa(sv)a(sa(sv)))"
                && self.message.body().data().bytes() == self.body,
        )?;
        for i in 0..18 {
            check(
                self.message.data().fds()[i].as_raw_fd() == self.aliases[i]
                    && snapshot(self.original[i])? == self.stamps[i]
                    && snapshot(self.aliases[i])? == self.stamps[i],
            )?;
        }
        Ok(())
    }
    pub(crate) fn names_match(&self, manager: &str, client: &str) -> bool {
        self.manager == manager && self.client == client
    }
    pub(crate) fn arm(&mut self) -> io::Result<&Message> {
        if self.phase != Phase::Prepared {
            self.refuse();
            return Err(denied());
        }
        self.phase = Phase::Uncertain;
        if let Err(e) = self.inspect() {
            self.refuse();
            return Err(e);
        }
        Ok(&self.message)
    }
    pub(crate) fn serial(&self) -> NonZeroU32 {
        self.serial
    }
    pub(crate) fn observe(&mut self, reply: &Message) -> io::Result<OwnedObjectPath> {
        let previous = std::mem::replace(&mut self.phase, Phase::Refused);
        check(previous == Phase::Uncertain)?;
        let h = reply.header();
        check(
            reply.message_type() == Type::MethodReturn
                && reply.data().bytes().len() <= crate::protocol::MAX_PACKET_BYTES
                && reply.primary_header().flags().bits()
                    & !(zbus::message::Flags::NoReplyExpected as u8)
                    == 0,
        )?;
        check(
            h.sender().map(|v| v.as_str()) == Some(self.manager.as_str())
                && h.destination().map(|v| v.as_str()) == Some(self.client.as_str())
                && h.reply_serial() == Some(self.serial)
                && h.path().is_none()
                && h.interface().is_none()
                && h.member().is_none()
                && h.error_name().is_none()
                && h.unix_fds().unwrap_or(0) == 0
                && reply.data().fds().is_empty()
                && reply.body().signature().to_string() == "o",
        )?;
        let path: OwnedObjectPath = reply.body().deserialize().map_err(|_| denied())?;
        let job = path
            .as_str()
            .strip_prefix("/org/freedesktop/systemd1/job/")
            .ok_or_else(denied)?;
        check(
            !job.is_empty()
                && job.len() <= 10
                && !job.starts_with('0')
                && job.bytes().all(|b| b.is_ascii_digit())
                && job.parse::<u32>().is_ok(),
        )?;
        self.phase = Phase::JobObserved;
        Ok(path)
    }
    pub(crate) fn refuse(&mut self) {
        self.phase = Phase::Refused;
    }
}
fn build(
    unit: &str,
    properties: &[(&'static str, UnitProperty)],
    manager: &str,
    client: &str,
    serial: NonZeroU32,
    original: [i32; 18],
    all: &[i32],
) -> io::Result<HeldRequest> {
    check(
        name(manager)
            && name(client)
            && manager != client
            && unit.starts_with("hydracache-diagnostic-074-")
            && unit.ends_with("-1.service")
            && unit.len() <= 128
            && unit
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.')),
    )?;
    check(
        properties.len() <= 64
            && !properties
                .iter()
                .enumerate()
                .any(|(i, (k, _))| properties[..i].iter().any(|(old, _)| old == k)),
    )?;
    check(!properties.iter().any(|(k, _)| {
        matches!(
            *k,
            "ExecStart"
                | "ExtraFileDescriptors"
                | "StandardInput"
                | "StandardInputFileDescriptor"
                | "StandardOutputFileDescriptor"
                | "StandardErrorFileDescriptor"
                | "StandardOutputFileToAppend"
                | "StandardErrorFileToAppend"
        )
    }))?;
    for (i, fd) in original.iter().enumerate() {
        check(!original[..i].contains(fd))?;
    }
    let stamps = original
        .iter()
        .map(|fd| snapshot(*fd))
        .collect::<io::Result<Vec<_>>>()?;
    let mut props = properties
        .iter()
        .map(|(k, v)| (*k, value(v)))
        .collect::<Vec<_>>();
    props.push((
        "ExecStart",
        Value::from(vec![(IMAGE, vec![IMAGE, ARG], false)]),
    ));
    for (i, k) in [
        "StandardInputFileDescriptor",
        "StandardOutputFileDescriptor",
        "StandardErrorFileDescriptor",
    ]
    .into_iter()
    .enumerate()
    {
        // Borrowed only during encoding; zbus owns distinct CLOEXEC aliases.
        props.push((
            k,
            Value::from(Fd::from(unsafe { BorrowedFd::borrow_raw(original[i]) })),
        ));
    }
    props.push((
        "ExtraFileDescriptors",
        Value::from(
            (3..18)
                .map(|i| {
                    (
                        Fd::from(unsafe { BorrowedFd::borrow_raw(original[i]) }),
                        NAMES[i - 3],
                    )
                })
                .collect::<Vec<_>>(),
        ),
    ));
    let body: Body<'_> = (unit, "fail", props, vec![]);
    let message = Message::method_call(PATH, "StartTransientUnit")
        .and_then(|b| b.destination(manager))
        .and_then(|b| b.interface(INTERFACE))
        .and_then(|b| b.serial(serial).build(&body))
        .map_err(|_| denied())?;
    check(message.data().fds().len() == 18)?;
    let aliases = std::array::from_fn(|i| message.data().fds()[i].as_raw_fd());
    for (i, fd) in aliases.iter().enumerate() {
        check(!aliases[..i].contains(fd) && !all.contains(fd) && !original.contains(fd))?;
    }
    let raw = message.body().data().bytes().to_vec();
    let result = HeldRequest {
        message,
        original,
        aliases,
        stamps,
        body: raw,
        manager: manager.into(),
        client: client.into(),
        serial,
        phase: Phase::Prepared,
    };
    result.inspect()?;
    Ok(result)
}
impl InstalledEntryBindings {
    pub(crate) fn held_entry_digest(&self) -> io::Result<[u8; 32]> {
        let fd = self.roles[16];
        let before = snapshot(fd)?;
        check(before.metadata[6] == 1280)?;
        let mut raw = [0; 1280];
        let mut at = 0;
        while at < raw.len() {
            let n = unsafe {
                libc::pread(
                    fd,
                    raw[at..].as_mut_ptr().cast(),
                    raw.len() - at,
                    at as libc::off_t,
                )
            };
            check(n > 0 && n as usize <= raw.len() - at)?;
            at += n as usize;
        }
        crate::diagnostic_builder::bootstrap_build::installed_entry_format::AssertedInstalledEntry::decode(&raw).map_err(|_|denied())?;
        check(snapshot(fd)? == before)?;
        use sha2::{Digest, Sha256};
        Ok(Sha256::digest(raw).into())
    }
    pub(crate) fn prepare_held_request(
        &self,
        unit: &str,
        properties: &[(&'static str, UnitProperty)],
        manager: &str,
        client: &str,
        serial: NonZeroU32,
        readiness: BorrowedFd<'_>,
    ) -> io::Result<HeldRequest> {
        let mut roles = [-1; 18];
        roles[..17].copy_from_slice(&self.roles);
        roles[17] = readiness.as_raw_fd();
        check(!self.fds.contains(&roles[17]))?;
        build(unit, properties, manager, client, serial, roles, &self.fds)
    }
}
#[cfg(test)]
#[path = "diagnostic_held_delivery_wire_tests.rs"]
mod tests;
