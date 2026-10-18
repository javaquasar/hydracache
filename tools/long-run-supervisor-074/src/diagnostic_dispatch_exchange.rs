//! Private message mechanics, not an authenticated original-start capability.
use std::num::NonZeroU32;
use std::os::fd::{AsRawFd, RawFd};
use zbus::message::{Message, Type};
use zbus::zvariant::OwnedObjectPath;

const PATH: &str = "/org/freedesktop/systemd1";
const INTERFACE: &str = "org.freedesktop.systemd1.Manager";
const METHOD: &str = "StartTransientUnit";
const JOB: &str = "/org/freedesktop/systemd1/job/";

#[derive(Debug, PartialEq, Eq)]
enum ExchangeError {
    Request,
    AlreadyAttempted,
    Reply,
    ManagerError,
}
#[derive(Debug, PartialEq, Eq)]
enum Phase {
    Prepared,
    Uncertain,
    JobObserved,
    Refused,
}

// Names below are checked claims, NOT trusted manager or connection evidence.
struct DispatchExchange {
    request: Message,
    manager: String,
    client: String,
    serial: NonZeroU32,
    phase: Phase,
    reply_open: bool,
}
struct PreparationFailure {
    error: ExchangeError,
    request: Message,
}
#[derive(Debug)]
struct JobObservation {
    path: OwnedObjectPath,
}
impl std::fmt::Debug for DispatchExchange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DispatchExchange")
            .field("phase", &self.phase)
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for PreparationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
fn unique_name(value: &str) -> bool {
    let Some(value) = value.strip_prefix(':') else {
        return false;
    };
    value.len() <= 64
        && value.split('.').count() == 2
        && value
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|v| v.is_ascii_digit()))
}
fn writer_shape(fd: RawFd) -> Result<(u64, u64), ExchangeError> {
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    // The message owns this FD throughout these bounded metadata observations.
    if unsafe { libc::fstat(fd, &mut stat) } != 0
        || stat.st_mode & libc::S_IFMT != libc::S_IFIFO
        || unsafe { libc::fcntl(fd, libc::F_GETFL) } != libc::O_WRONLY
        || unsafe { libc::fcntl(fd, libc::F_GETFD) } != libc::FD_CLOEXEC
    {
        return Err(ExchangeError::Request);
    }
    Ok((stat.st_dev, stat.st_ino))
}
fn request_matches(message: &Message, manager: &str, client: &str) -> Result<(), ExchangeError> {
    let h = message.header();
    if !unique_name(manager)
        || !unique_name(client)
        || manager == client
        || message.data().bytes().len() > crate::protocol::MAX_PACKET_BYTES
        || message.message_type() != Type::MethodCall
        || !message.primary_header().flags().is_empty()
        || h.path().map(|v| v.as_str()) != Some(PATH)
        || h.interface().map(|v| v.as_str()) != Some(INTERFACE)
        || h.member().map(|v| v.as_str()) != Some(METHOD)
        || h.destination().map(|v| v.as_str()) != Some(manager)
        || h.sender().is_some()
        || h.reply_serial().is_some()
        || h.error_name().is_some()
        || h.unix_fds() != Some(2)
        || message.data().fds().len() != 2
        || message.body().signature().to_string() != "(ssa(sv)a(sa(sv)))"
    {
        return Err(ExchangeError::Request);
    }
    let fds = message.data().fds();
    if writer_shape(fds[0].as_raw_fd())? == writer_shape(fds[1].as_raw_fd())? {
        return Err(ExchangeError::Request);
    }
    Ok(())
}
impl DispatchExchange {
    fn prepare(request: Message, manager: &str, client: &str) -> Result<Self, PreparationFailure> {
        if let Err(error) = request_matches(&request, manager, client) {
            return Err(PreparationFailure { error, request });
        }
        let serial = request.primary_header().serial_num();
        Ok(Self {
            request,
            manager: manager.into(),
            client: client.into(),
            serial,
            phase: Phase::Prepared,
            reply_open: false,
        })
    }
    // Call before any possible transport side effect. This module deliberately
    // has no send/get-message/callback API and cannot acquire execution authority.
    fn arm(&mut self) -> Result<(), ExchangeError> {
        if self.phase != Phase::Prepared {
            self.phase = Phase::Refused;
            return Err(ExchangeError::AlreadyAttempted);
        }
        self.phase = Phase::Uncertain;
        self.reply_open = true;
        Ok(())
    }
    // The future bounded transport must call this on timeout/disconnect/error.
    // Closing cannot reset the attempt or admit a delayed acknowledgement.
    fn lose_reply(&mut self) {
        self.reply_open = false;
        if self.phase == Phase::Prepared {
            self.phase = Phase::Refused;
        }
    }
    fn observe(&mut self, reply: &Message) -> Result<JobObservation, ExchangeError> {
        let attempted = self.phase == Phase::Uncertain && self.reply_open;
        self.reply_open = false;
        // Poison BEFORE parsing: a malformed/duplicate/early reply cannot heal.
        self.phase = Phase::Refused;
        if !attempted {
            return Err(ExchangeError::Reply);
        }
        let h = reply.header();
        if reply.data().bytes().len() > crate::protocol::MAX_PACKET_BYTES
            || h.reply_serial() != Some(self.serial)
            || h.sender().map(|v| v.as_str()) != Some(self.manager.as_str())
            || h.destination().map(|v| v.as_str()) != Some(self.client.as_str())
            || !reply.primary_header().flags().is_empty()
            || !reply.data().fds().is_empty()
            || h.unix_fds().unwrap_or(0) != 0
            || h.path().is_some()
            || h.interface().is_some()
            || h.member().is_some()
        {
            return Err(ExchangeError::Reply);
        }
        if reply.message_type() == Type::Error && h.error_name().is_some() {
            // Even a correctly correlated error is not absence/retry/release.
            // Do not decode, retain in diagnostics, or print its secret-bearing body.
            return Err(ExchangeError::ManagerError);
        }
        if reply.message_type() != Type::MethodReturn
            || h.error_name().is_some()
            || reply.body().signature().to_string() != "o"
        {
            return Err(ExchangeError::Reply);
        }
        let path: OwnedObjectPath = reply
            .body()
            .deserialize()
            .map_err(|_| ExchangeError::Reply)?;
        let suffix = path
            .as_str()
            .strip_prefix(JOB)
            .ok_or(ExchangeError::Reply)?;
        let id = suffix.parse::<u32>().map_err(|_| ExchangeError::Reply)?;
        if id == 0 || id.to_string() != suffix {
            return Err(ExchangeError::Reply);
        }
        self.phase = Phase::JobObserved;
        Ok(JobObservation { path })
    }
}

#[cfg(test)]
#[path = "diagnostic_dispatch_exchange_tests.rs"]
mod tests;
