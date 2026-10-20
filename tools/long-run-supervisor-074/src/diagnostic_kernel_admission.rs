//! Kernel message/time producer; independent production trust enrollment is absent.
use crate::diagnostic_ipc::{
    self, DiagnosticPeer, DiagnosticPolicy, DiagnosticRequest, MAX_PACKET_BYTES,
};
use std::fs::{File, OpenOptions};
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};
use std::sync::Arc;
use thiserror::Error;

// BEGIN TEST ONLY KERNEL REFUSAL TRACE
#[cfg(test)]
#[path = "diagnostic_kernel_refusal_trace.rs"]
pub(crate) mod refusal_trace;
macro_rules! kernel_invalid {
    ($site:literal) => {
        kernel_invalid!($site, [0; 6])
    };
    ($site:literal, $facts:expr) => {{
        #[cfg(test)]
        $crate::diagnostic_kernel_admission::refusal_trace::record($site, file!(), line!(), $facts);
        KernelError::Invalid
    }};
}
macro_rules! kernel_required {
    ($option:expr, $site:literal) => {{
        #[cfg(test)]
        {
            ($option).ok_or_else(|| kernel_invalid!($site))
        }
        #[cfg(not(test))]
        {
            ($option).ok_or(KernelError::Invalid)
        }
    }};
}
// END TEST ONLY KERNEL REFUSAL TRACE

#[path = "diagnostic_kernel_ancillary.rs"]
mod ancillary;
#[path = "diagnostic_kernel_collector_inputs.rs"]
mod collector_inputs;
#[path = "diagnostic_kernel_origin.rs"]
mod origin;
pub use origin::KernelTimePoint;
pub(crate) use origin::OriginalReceiverBinding;
use origin::{Origin, TimePoint};

const DOC_BYTES: usize = 65_536;
const HOLD_NS: u64 = 60_000_000_000;

#[derive(Debug, Error)]
pub enum KernelError {
    #[error("kernel diagnostic packet, ancillary data or original context refused")]
    Invalid,
    #[error("original sender is exited or uncertain")]
    NotLive,
    #[error("kernel input owner previously refused")]
    Refused,
    #[error("kernel diagnostic I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("independently configured signature/principal admission failed: {0}")]
    Authentication(#[from] diagnostic_ipc::DiagnosticIpcError),
}

pub struct KernelReceiver {
    fd: OwnedFd,
    id: (u64, u64),
    origin: Arc<Origin>,
    refused: bool,
}
/// Original received message; no raw descriptors, constructor or start authority.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_kernel_admission::KernelPacket;
/// fn launch(p: KernelPacket) { p.start_worker(); }
/// ```
pub struct KernelPacket {
    bytes: Vec<u8>,
    origin: Arc<Origin>,
    received: KernelTimePoint,
    sender: Sender,
    refused: bool,
}
impl KernelReceiver {
    pub(crate) fn new(fd: OwnedFd) -> Result<Self, KernelError> {
        ancillary::configure(&fd)?;
        let file = File::from(fd.try_clone()?);
        let m = file.metadata()?;
        let origin = Arc::new(Origin::open()?);
        Ok(Self {
            fd,
            id: (m.dev(), m.ino()),
            origin,
            refused: false,
        })
    }
    pub fn try_receive(&mut self) -> Result<Option<KernelPacket>, KernelError> {
        if self.refused {
            return Err(KernelError::Refused);
        }
        let result = self.receive_inner();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
    fn receive_inner(&self) -> Result<Option<KernelPacket>, KernelError> {
        ancillary::revalidate_socket(&self.fd)?;
        let file = File::from(self.fd.try_clone()?);
        let m = file.metadata()?;
        if (m.dev(), m.ino()) != self.id {
            return Err(kernel_invalid!("receiver-object"));
        }
        let before = self.origin.sample()?;
        let Some((bytes, credentials, pidfd)) = ancillary::receive(&self.fd)? else {
            return Ok(None);
        };
        let after = self.origin.sample()?;
        ordered(&before, &after)?;
        let sender = Sender::pin(&self.origin, credentials, pidfd)?;
        Ok(Some(KernelPacket {
            bytes,
            origin: Arc::clone(&self.origin),
            received: after,
            sender,
            refused: false,
        }))
    }
}
impl KernelPacket {
    pub(crate) fn latch_refusal(&mut self) {
        self.refused = true;
    }
    pub(crate) fn is_refused(&self) -> bool {
        self.refused
    }
    /// Compare retained receiver context and current sender/time, not principal admission.
    pub(crate) fn require_worker_context(
        &mut self,
        host: &crate::diagnostic_worker_policy::AssertedWorkerHost,
    ) -> Result<(), KernelError> {
        #[cfg(test)]
        let _live_walk = crate::diagnostic_ipc::validation_trace::live_walk::span(
            crate::diagnostic_ipc::validation_trace::live_walk::Kind::KernelContext,
        );
        #[cfg(test)]
        let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
            crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::PacketContext,
        );
        if self.refused {
            return Err(KernelError::Refused);
        }
        let result = (|| {
            self.sender.revalidate(&self.origin)?;
            self.origin.require_worker_context(host)?;
            valid_hold(&self.received, &self.origin.sample()?)?;
            self.sender.revalidate(&self.origin)
        })();
        if result.is_err() {
            self.latch_refusal();
        }
        result
    }
    /// Require the reviewed controller key/repository before kernel authentication.
    /// The supplied principal policy is not enrolled host/epoch/replay authority.
    /// Like generic authentication, this returns request data, never a start guard.
    pub fn authenticate_reviewed_controller(
        &mut self,
        policy: &DiagnosticPolicy,
    ) -> Result<DiagnosticRequest, KernelError> {
        if self.refused {
            return Err(KernelError::Refused);
        }
        if crate::diagnostic_reviewed_trust::require_controller_pin(policy).is_err() {
            self.refused = true;
            return Err(kernel_invalid!("reviewed-controller-pin"));
        }
        self.authenticate(policy)
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn sender_pid(&self) -> u32 {
        self.sender.credentials.pid as u32
    }
    pub fn received_time(&self) -> &KernelTimePoint {
        &self.received
    }
    // Private fresh origin sample; never a caller timestamp or a renewed packet hold.
    pub(crate) fn sample_original_unix_seconds(&mut self) -> Result<u64, KernelError> {
        let result = (|| {
            if self.refused {
                return Err(KernelError::Refused);
            }
            self.sender.revalidate(&self.origin)?;
            let now = self.origin.sample()?;
            valid_hold(&self.received, &now)?;
            self.sender.revalidate(&self.origin)?;
            Ok(now.unix_seconds())
        })();
        if result.is_err() {
            self.latch_refusal();
        }
        result
    }
    pub fn revalidate_sender(&mut self) -> Result<(), KernelError> {
        if self.refused {
            return Err(KernelError::Refused);
        }
        let result = self.sender.revalidate(&self.origin);
        if result.is_err() {
            self.refused = true;
        }
        result
    }
    /// Kernel peer/time inputs, but supplied policy is NOT production enrollment.
    /// This verifies a request only; durable replay/fence admission remains separate.
    pub fn authenticate(
        &mut self,
        policy: &DiagnosticPolicy,
    ) -> Result<DiagnosticRequest, KernelError> {
        if self.refused {
            return Err(KernelError::Refused);
        }
        let result = (|| {
            self.sender.revalidate(&self.origin)?;
            let now = self.origin.sample()?;
            valid_hold(&self.received, &now)?;
            let mut gids = self.sender.projection.groups.clone();
            if !gids.contains(&self.sender.credentials.gid) {
                gids.push(self.sender.credentials.gid);
            }
            let request = diagnostic_ipc::authenticate(
                &self.bytes,
                &DiagnosticPeer {
                    uid: self.sender.credentials.uid,
                    gids,
                },
                policy,
                now.unix_seconds(),
            )?;
            if request.identity.boot_id != now.boot_id() {
                return Err(kernel_invalid!("request-boot"));
            }
            self.sender.revalidate(&self.origin)?;
            Ok(request)
        })();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
}
fn ordered(before: &TimePoint, after: &TimePoint) -> Result<(), KernelError> {
    if before.boot_id() != after.boot_id()
        || after.boottime_ns() < before.boottime_ns()
        || after.realtime_ns < before.realtime_ns
    {
        return Err(kernel_invalid!(
            "clock-order",
            [
                before.boottime_ns(),
                after.boottime_ns(),
                before.realtime_ns,
                after.realtime_ns,
                u64::from(before.boot_id() == after.boot_id()),
                HOLD_NS
            ]
        ));
    }
    Ok(())
}
fn valid_hold(before: &TimePoint, after: &TimePoint) -> Result<(), KernelError> {
    ordered(before, after)?;
    if after.boottime_ns() - before.boottime_ns() >= HOLD_NS {
        return Err(kernel_invalid!(
            "packet-hold",
            [
                before.boottime_ns(),
                after.boottime_ns(),
                before.realtime_ns,
                after.realtime_ns,
                u64::from(before.boot_id() == after.boot_id()),
                HOLD_NS
            ]
        ));
    }
    Ok(())
}
fn bounded(file: &File) -> Result<Vec<u8>, KernelError> {
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Document,
    );
    let mut bytes = vec![0; DOC_BYTES + 1];
    let count = file.read_at(&mut bytes, 0)?;
    if count == 0 || count > DOC_BYTES {
        return Err(kernel_invalid!("proc-document-budget"));
    }
    bytes.truncate(count);
    Ok(bytes)
}
#[derive(Debug, PartialEq, Eq)]
struct Projection {
    uid: [u32; 4],
    gid: [u32; 4],
    groups: Vec<u32>,
}
fn projection(bytes: &[u8], credentials: &libc::ucred) -> Result<Projection, KernelError> {
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Projection,
    );
    let text = std::str::from_utf8(bytes).map_err(|_| kernel_invalid!("projection-utf8"))?;
    let field = |prefix: &str| -> Result<Vec<u32>, KernelError> {
        let mut lines = text.lines().filter_map(|line| line.strip_prefix(prefix));
        let body = kernel_required!(lines.next(), "projection-missing-field")?;
        if lines.next().is_some() {
            return Err(kernel_invalid!("projection-duplicate-field"));
        }
        let values = body
            .split_ascii_whitespace()
            .take(33)
            .map(|v| v.parse::<u32>().map_err(|_| kernel_invalid!("proc-u32")))
            .collect::<Result<Vec<_>, _>>()?;
        if values.len() > 32 {
            return Err(kernel_invalid!("projection-group-budget"));
        }
        Ok(values)
    };
    if field("Pid:")? != [credentials.pid as u32] || field("Tgid:")? != [credentials.pid as u32] {
        return Err(kernel_invalid!("projection-process-id"));
    }
    let uid: [u32; 4] = field("Uid:")?
        .try_into()
        .map_err(|_| kernel_invalid!("projection-uid-width"))?;
    let gid: [u32; 4] = field("Gid:")?
        .try_into()
        .map_err(|_| kernel_invalid!("projection-gid-width"))?;
    let groups = field("Groups:")?;
    if uid != [credentials.uid; 4]
        || gid != [credentials.gid; 4]
        || groups.windows(2).any(|v| v[0] >= v[1])
    {
        return Err(kernel_invalid!("projection-credentials-or-groups"));
    }
    Ok(Projection { uid, gid, groups })
}
struct Sender {
    credentials: libc::ucred,
    pidfd: OwnedFd,
    directory: File,
    directory_id: (u64, u64),
    status: File,
    status_id: (u64, u64),
    projection: Projection,
}
impl Sender {
    fn pin(origin: &Origin, credentials: libc::ucred, pidfd: OwnedFd) -> Result<Self, KernelError> {
        alive(&pidfd, credentials.pid)?;
        let directory = origin.child(&credentials.pid.to_string(), true)?;
        let m = directory.metadata()?;
        let status = origin::child(&directory, "status", false)?;
        let sm = status.metadata()?;
        let projection = projection(&bounded(&status)?, &credentials)?;
        let sender = Self {
            credentials,
            pidfd,
            directory,
            directory_id: (m.dev(), m.ino()),
            status,
            status_id: (sm.dev(), sm.ino()),
            projection,
        };
        sender.revalidate(origin)?;
        Ok(sender)
    }
    fn revalidate(&self, origin: &Origin) -> Result<(), KernelError> {
        #[cfg(test)]
        let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
            crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Sender,
        );
        origin.revalidate()?;
        alive(&self.pidfd, self.credentials.pid)?;
        let directory = origin.child(&self.credentials.pid.to_string(), true)?;
        for file in [&directory, &self.directory] {
            let m = file.metadata()?;
            if (m.dev(), m.ino()) != self.directory_id {
                return Err(kernel_invalid!("sender-directory"));
            }
        }
        let named = origin::child(&self.directory, "status", false)?;
        for file in [&named, &self.status] {
            let m = file.metadata()?;
            if (m.dev(), m.ino()) != self.status_id
                || projection(&bounded(file)?, &self.credentials)? != self.projection
            {
                return Err(kernel_invalid!("sender-status-binding"));
            }
        }
        alive(&self.pidfd, self.credentials.pid)?;
        origin.revalidate()
    }
}
fn alive(pidfd: &OwnedFd, pid: libc::pid_t) -> Result<(), KernelError> {
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Alive,
    );
    let mut poll = libc::pollfd {
        fd: pidfd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let ready = unsafe { libc::poll(&mut poll, 1, 0) };
    if ready < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if ready != 0 || poll.revents != 0 {
        return Err(KernelError::NotLive);
    }
    let info = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(format!("/proc/thread-self/fdinfo/{}", pidfd.as_raw_fd()))?;
    let raw = bounded(&info)?;
    origin::filesystem(&info, 0x9fa0)?;
    let text = std::str::from_utf8(&raw).map_err(|_| kernel_invalid!("pidfd-info-utf8"))?;
    let mut lines = text.lines().filter_map(|line| line.strip_prefix("Pid:"));
    let observed = kernel_required!(lines.next(), "pidfd-info-missing-pid")?
        .trim()
        .parse::<i32>()
        .map_err(|_| kernel_invalid!("pidfd-info-pid-number"))?;
    if lines.next().is_some() || observed != pid || pid <= 0 {
        return Err(KernelError::NotLive);
    }
    Ok(())
}
#[cfg(test)]
#[path = "diagnostic_kernel_admission_tests.rs"]
mod tests;
