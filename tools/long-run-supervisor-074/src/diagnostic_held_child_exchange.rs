//! Private readiness mechanics. Kernel peer proof is not execution admission.
use super::exchange_imports::{KernelError, MAX_PACKET_BYTES};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io,
    os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
    os::unix::fs::{FileExt, MetadataExt},
};
// Reuse the unchanged bounded ancillary parser, including error-path FD cleanup.
// Intentional second compilation of a frozen private leaf, not a fork of its
// implementation or a visibility change to the original admission boundary.
#[allow(clippy::duplicate_mod)]
#[path = "diagnostic_kernel_ancillary.rs"]
mod ancillary;
const CHALLENGE: usize = 256;
const ACK: usize = 80;
fn denied() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "held readiness exchange refused",
    )
}
fn need(ok: bool) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(denied())
    }
}
fn kernel<T>(r: Result<T, KernelError>) -> io::Result<T> {
    r.map_err(|_| denied())
}
pub(crate) fn clock() -> io::Result<u64> {
    let mut t: libc::timespec = unsafe { std::mem::zeroed() };
    if unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut t) } != 0 {
        return Err(io::Error::last_os_error());
    }
    need(t.tv_sec >= 0 && (0..1_000_000_000).contains(&t.tv_nsec))?;
    (t.tv_sec as u64)
        .checked_mul(1_000_000_000)
        .and_then(|v| v.checked_add(t.tv_nsec as u64))
        .ok_or_else(denied)
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Challenge {
    pub(crate) deadline: u64,
    pub(crate) creator: [u32; 3],
    pub(crate) child: u32,
    pub(crate) nonce: [u8; 32],
    pub(crate) entry: [u8; 32],
    pub(crate) context: [u8; 32],
    pub(crate) manager: [u8; 32],
    pub(crate) client: [u8; 32],
    pub(crate) invocation: [u8; 16],
}
impl Challenge {
    pub(crate) fn release_binding(&self) -> io::Result<([u8; 32], [u8; 32])> {
        let frame = self.encode()?;
        let mut nonce = Sha256::new();
        nonce.update(b"hydracache-held-worker-release-nonce-074-v1\0");
        nonce.update(frame);
        Ok((Sha256::digest(frame).into(), nonce.finalize().into()))
    }
    fn encode(&self) -> io::Result<[u8; CHALLENGE]> {
        need(
            self.deadline > 0
                && self.deadline <= i64::MAX as u64
                && self.creator[0] > 1
                && self.creator[0] <= i32::MAX as u32
                && self.child > 1
                && self.child <= i32::MAX as u32
                && self.child != self.creator[0],
        )?;
        need(
            self.creator[1..].iter().all(|v| *v != u32::MAX)
                && self.manager != self.client
                && self.invocation != [0; 16],
        )?;
        need(
            [
                self.nonce,
                self.entry,
                self.context,
                self.manager,
                self.client,
            ]
            .iter()
            .all(|v| *v != [0; 32]),
        )?;
        let mut raw = [0; CHALLENGE];
        raw[..8].copy_from_slice(b"HCHR0741");
        raw[8..16].copy_from_slice(&self.deadline.to_le_bytes());
        for (at, v) in self.creator.iter().chain([&self.child]).enumerate() {
            raw[16 + 4 * at..20 + 4 * at].copy_from_slice(&v.to_le_bytes());
        }
        for (at, v) in [
            self.nonce,
            self.entry,
            self.context,
            self.manager,
            self.client,
        ]
        .iter()
        .enumerate()
        {
            raw[32 + 32 * at..64 + 32 * at].copy_from_slice(v);
        }
        raw[192..208].copy_from_slice(&self.invocation);
        Ok(raw)
    }
    #[cfg_attr(not(test), allow(dead_code))] // Reserved for the separately gated child entry.
    fn decode(raw: &[u8]) -> io::Result<Self> {
        need(
            raw.len() == CHALLENGE
                && &raw[..8] == b"HCHR0741"
                && raw[208..].iter().all(|v| *v == 0),
        )?;
        let word = |at| u32::from_le_bytes(raw[at..at + 4].try_into().expect("fixed length"));
        let hash = |at| raw[at..at + 32].try_into().expect("fixed length");
        let value = Self {
            deadline: u64::from_le_bytes(raw[8..16].try_into().expect("fixed length")),
            creator: [word(16), word(20), word(24)],
            child: word(28),
            nonce: hash(32),
            entry: hash(64),
            context: hash(96),
            manager: hash(128),
            client: hash(160),
            invocation: raw[192..208].try_into().expect("fixed length"),
        };
        need(value.encode()?.as_slice() == raw)?;
        Ok(value)
    }
    fn ack(&self) -> io::Result<[u8; ACK]> {
        let mut raw = [0; ACK];
        raw[..8].copy_from_slice(b"HCHA0741");
        raw[8..16].copy_from_slice(&(ACK as u64).to_le_bytes());
        let mut h = Sha256::new();
        h.update(b"hydracache-held-readiness-074-v1\0");
        h.update(self.encode()?);
        raw[16..48].copy_from_slice(&h.finalize());
        raw[48..].copy_from_slice(&self.nonce);
        Ok(raw)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Fresh,
    Uncertain,
    Ready,
    ReleaseSent,
    ReleaseAcknowledged,
    Refused,
}
pub(crate) struct Endpoint {
    fd: OwnedFd,
    id: [u64; 2],
}
impl Endpoint {
    pub(crate) fn new(fd: OwnedFd) -> io::Result<Self> {
        kernel(ancillary::configure(&fd))?;
        Self::from_inherited(fd)
    }
    pub(crate) fn from_inherited(fd: OwnedFd) -> io::Result<Self> {
        kernel(ancillary::revalidate_socket(&fd))?;
        let m = File::from(fd.try_clone()?).metadata()?;
        Ok(Self {
            fd,
            id: [m.dev(), m.ino()],
        })
    }
    fn check(&self) -> io::Result<()> {
        kernel(ancillary::revalidate_socket(&self.fd))?;
        let m = File::from(self.fd.try_clone()?).metadata()?;
        need([m.dev(), m.ino()] == self.id)
    }
    fn send(&self, raw: &[u8]) -> io::Result<()> {
        self.check()?;
        let n = unsafe {
            libc::send(
                self.fd.as_raw_fd(),
                raw.as_ptr().cast(),
                raw.len(),
                libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL,
            )
        };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        need(n as usize == raw.len())?;
        self.check()
    }
    pub(crate) fn receive(&self, expected: [u32; 3]) -> io::Result<Option<Vec<u8>>> {
        self.check()?;
        let Some((raw, cred, pidfd)) = kernel(ancillary::receive(&self.fd))? else {
            return Ok(None);
        };
        need([cred.pid as u32, cred.uid, cred.gid] == expected)?;
        // This pidfd was delivered atomically with SCM_CREDENTIALS by recvmsg,
        // never opened by a packet-supplied PID or inferred from SO_PEERCRED.
        require_live_peer(pidfd.as_raw_fd(), expected[0])?;
        self.check()?;
        Ok(Some(raw))
    }
}

pub(crate) fn require_live_peer(fd: i32, pid: u32) -> io::Result<()> {
        let mut p = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        need(unsafe { libc::poll(&mut p, 1, 0) } == 0 && p.revents == 0)?;
        let info = File::open(format!("/proc/thread-self/fdinfo/{fd}"))?;
        let mut fs: libc::statfs = unsafe { std::mem::zeroed() };
        need(
            unsafe { libc::fstatfs(info.as_raw_fd(), &mut fs) } == 0
                && fs.f_type == libc::PROC_SUPER_MAGIC as libc::c_long,
        )?;
        let mut buf = [0; 4097];
        let n = info.read_at(&mut buf, 0)?;
        need(n > 0 && n <= 4096)?;
        let text = std::str::from_utf8(&buf[..n]).map_err(|_| denied())?;
        let mut pids = text.lines().filter_map(|s| s.strip_prefix("Pid:"));
        need(
            pids.next().and_then(|s| s.trim().parse::<u32>().ok()) == Some(pid)
                && pids.next().is_none(),
        )?;
        p.revents = 0;
        need(unsafe { libc::poll(&mut p, 1, 0) } == 0 && p.revents == 0)
}

pub(crate) const WORKER_RUN_ARG: &str = "--run-held-v1";

pub(crate) fn run_permission(ack: &[u8; 48]) -> [u8; 40] {
    let mut hash = Sha256::new();
    hash.update(b"hydracache-held-worker-run-074-v1\0");
    hash.update(ack);
    let mut packet = [0; 40];
    packet[..8].copy_from_slice(b"HCRW0741");
    packet[8..].copy_from_slice(&hash.finalize());
    packet
}
pub(crate) struct Pair {
    parent: Endpoint,
    child: Endpoint,
    phase: Phase,
    challenge: Option<Challenge>,
    release_reply: Option<[u8; 48]>,
    run_permission: Option<[u8; 40]>,
}
impl Pair {
    pub(crate) fn create() -> io::Result<Self> {
        let mut fds = [-1; 2];
        if unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
                0,
                fds.as_mut_ptr(),
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        let [a, b] = fds.map(|fd| unsafe { OwnedFd::from_raw_fd(fd) });
        Ok(Self {
            parent: Endpoint::new(a)?,
            child: Endpoint::new(b)?,
            phase: Phase::Fresh,
            challenge: None,
            release_reply: None,
            run_permission: None,
        })
    }
    pub(crate) fn child_fd(&self) -> BorrowedFd<'_> {
        self.child.fd.as_fd()
    }
    pub(crate) fn check(&self) -> io::Result<()> {
        need(self.phase != Phase::Refused)?;
        self.parent.check()?;
        self.child.check()
    }
    pub(crate) fn begin(&mut self, challenge: Challenge) -> io::Result<()> {
        let result = (|| {
            need(self.phase == Phase::Fresh)?;
            // Latch before validation, clock sampling or the first possible send.
            self.phase = Phase::Uncertain;
            self.challenge = Some(challenge);
            self.check()?;
            let c = self.challenge.as_ref().ok_or_else(denied)?;
            need(clock()? < c.deadline)?;
            self.parent.send(&c.encode()?)?;
            need(clock()? < c.deadline)
        })();
        if result.is_err() {
            self.refuse();
        }
        result
    }
    pub(crate) fn try_ack(&mut self, peer: [u32; 3]) -> io::Result<bool> {
        let result = (|| {
            need(self.phase == Phase::Uncertain)?;
            self.check()?;
            let c = self.challenge.as_ref().ok_or_else(denied)?;
            need(peer[0] == c.child && clock()? < c.deadline)?;
            let Some(raw) = self.parent.receive(peer)? else {
                return Ok(false);
            };
            // Consume the reply window BEFORE inspecting untrusted bytes.
            self.phase = Phase::Refused;
            need(raw.as_slice() == c.ack()? && clock()? < c.deadline)?;
            self.phase = Phase::Ready;
            Ok(true)
        })();
        if result.is_err() {
            self.refuse();
        }
        result
    }
    #[cfg(test)]
    pub(crate) fn discard_ready_ack_for_test(&self, peer: [u32; 3]) -> io::Result<bool> {
        need(self.phase == Phase::Uncertain)?;
        self.check()?;
        let challenge = self.challenge.as_ref().ok_or_else(denied)?;
        need(peer[0] == challenge.child && clock()? < challenge.deadline)?;
        let Some(raw) = self.parent.receive(peer)? else {
            return Ok(false);
        };
        // Verify the genuine child packet, then lose it at the test boundary.
        // The production receive path and both state machines stay unchanged.
        need(raw.as_slice() == challenge.ack()? && clock()? < challenge.deadline)?;
        Ok(true)
    }

    pub(crate) fn refuse(&mut self) {
        self.phase = Phase::Refused;
    }

    pub(crate) fn ready_challenge(&self) -> io::Result<&Challenge> {
        need(self.phase == Phase::Ready)?;
        self.challenge.as_ref().ok_or_else(denied)
    }

    pub(crate) fn send_release_once(&mut self, raw: &[u8], reply: [u8; 48]) -> io::Result<()> {
        let deadline = self.ready_challenge()?.deadline;
        self.phase = Phase::ReleaseSent;
        self.release_reply = Some(reply);
        self.run_permission = Some(run_permission(&reply));
        self.check()?;
        need(clock()? < deadline)?;
        self.parent.send(raw)?;
        need(clock()? < deadline)
    }

    pub(crate) fn worker_run_permission(&self) -> io::Result<[u8; 40]> {
        need(self.phase == Phase::ReleaseAcknowledged)?;
        self.run_permission.ok_or_else(denied)
    }

    pub(crate) fn send_worker_run_once(&mut self) -> io::Result<()> {
        let packet = self.worker_run_permission()?;
        self.run_permission = None;
        let deadline = self.challenge.as_ref().ok_or_else(denied)?.deadline;
        self.check()?;
        need(clock()? < deadline)?;
        self.parent.send(&packet)?;
        need(clock()? < deadline)
    }

    pub(crate) fn try_release_ack(&mut self, peer: [u32; 3]) -> io::Result<bool> {
        let result = (|| {
            need(self.phase == Phase::ReleaseSent && self.release_reply.is_some())?;
            self.check()?;
            let challenge = self.challenge.as_ref().ok_or_else(denied)?;
            need(peer[0] == challenge.child && clock()? < challenge.deadline)?;
            let Some(raw) = self.parent.receive(peer)? else {
                return Ok(false);
            };
            let expected = self.release_reply.take().ok_or_else(denied)?;
            need(raw == expected && clock()? < challenge.deadline)?;
            self.phase = Phase::ReleaseAcknowledged;
            Ok(true)
        })();
        if result.is_err() {
            // End this reply window without disabling observation of the
            // original process generation or reopening the send window.
            self.release_reply = None;
        }
        result
    }

    #[cfg(test)]
    pub(crate) fn discard_release_ack_for_test(&self, peer: [u32; 3]) -> io::Result<bool> {
        need(self.phase == Phase::ReleaseSent)?;
        let challenge = self.challenge.as_ref().ok_or_else(denied)?;
        need(peer[0] == challenge.child && clock()? < challenge.deadline)?;
        let Some(raw) = self.parent.receive(peer)? else {
            return Ok(false);
        };
        need(self.release_reply.as_ref().is_some_and(|ack| raw == *ack))?;
        Ok(true)
    }
}

// Child-side one-shot primitive, deliberately not wired into an execution CLI.
// The caller must bracket this with unchanged original-role/creator checks.
#[cfg_attr(not(test), allow(dead_code))] // No production child CLI is activated.
pub(crate) struct Child {
    endpoint: Endpoint,
    phase: Phase,
}
#[cfg_attr(not(test), allow(dead_code))]
impl Child {
    /// Readiness only: learn the nonce from the authenticated packet, not argv.
    /// Full fresh role checks bracket receive and the only possible ACK.
    pub(crate) fn answer_framed_once(
        &mut self,
        expected: &Challenge,
        mut frame: impl FnMut() -> io::Result<()>,
    ) -> io::Result<Challenge> {
        let result = (|| {
            need(self.phase == Phase::Fresh)?;
            self.phase = Phase::Uncertain;
            frame()?;
            need(
                expected.nonce == [0; 32]
                    && expected.child == std::process::id()
                    && clock()? < expected.deadline,
            )?;
            let raw = self
                .endpoint
                .receive(expected.creator)?
                .ok_or_else(denied)?;
            let received = Challenge::decode(&raw)?;
            let mut bound = expected.clone();
            bound.nonce = received.nonce;
            need(received == bound)?;
            frame()?;
            need(clock()? < expected.deadline)?;
            self.endpoint.send(&received.ack()?)?;
            frame()?;
            need(clock()? < expected.deadline)?;
            self.phase = Phase::Ready;
            Ok(received)
        })();
        if result.is_err() {
            self.phase = Phase::Refused;
        }
        result
    }

    // Invoked only by the release-enabled bootstrap after the unchanged
    // readiness exchange. Latch before receive, validation or acceptance.
    pub(crate) fn accept_release_once(
        &mut self,
        challenge: &Challenge,
        mut frame: impl FnMut() -> io::Result<()>,
        authorize: impl FnOnce(&[u8]) -> io::Result<[u8; 48]>,
    ) -> io::Result<()> {
        let prior = std::mem::replace(&mut self.phase, Phase::Refused);
        need(prior == Phase::Ready)?;
        frame()?;
        need(clock()? < challenge.deadline)?;
        let raw = self
            .endpoint
            .receive(challenge.creator)?
            .ok_or_else(denied)?;
        let acknowledgement = authorize(&raw)?;
        frame()?;
        need(clock()? < challenge.deadline)?;
        self.endpoint.send(&acknowledgement)?;
        frame()?;
        need(clock()? < challenge.deadline)
    }
    pub(crate) fn from_owned(fd: OwnedFd) -> io::Result<Self> {
        Ok(Self {
            endpoint: Endpoint::new(fd)?,
            phase: Phase::Fresh,
        })
    }
    pub(crate) fn answer_once(&mut self, expected: &Challenge) -> io::Result<()> {
        let result = (|| {
            need(self.phase == Phase::Fresh)?;
            self.phase = Phase::Uncertain;
            need(expected.child == std::process::id() && clock()? < expected.deadline)?;
            let raw = self
                .endpoint
                .receive(expected.creator)?
                .ok_or_else(denied)?;
            need(Challenge::decode(&raw)? == *expected)?;
            need(clock()? < expected.deadline)?;
            self.endpoint.send(&expected.ack()?)?;
            need(clock()? < expected.deadline)?;
            self.phase = Phase::Ready;
            Ok(())
        })();
        if result.is_err() {
            self.phase = Phase::Refused;
        }
        result
    }
}

#[cfg(test)]
#[path = "diagnostic_held_child_exchange_tests.rs"]
mod tests;
