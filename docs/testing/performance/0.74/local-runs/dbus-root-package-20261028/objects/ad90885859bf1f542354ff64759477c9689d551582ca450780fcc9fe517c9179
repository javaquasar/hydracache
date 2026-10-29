//! Parent-owned proc-less restriction observation. No authority or handoff.
use super::{maps, refused, require, FixedProbe, Owner, Phase, Stamp, STOP};
use std::collections::BTreeSet;
use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::FileExt;
const SYSCALL: i32 = ((libc::SIGTRAP | 0x80) << 8) | 0x7f;
const INSTALLED: &[u8; 16] = b"HC074ROOTGUARD1\n";
const DONE: &[u8; 16] = b"HC074ROOTDONEv1\n";
struct ProcRead {
    file: File,
    identity: Stamp,
}
impl ProcRead {
    fn capture(file: File, directory: bool) -> io::Result<Self> {
        let identity = Stamp::of(&file.metadata()?);
        let result = Self { file, identity };
        result.check(directory)?;
        Ok(result)
    }
    fn check(&self, directory: bool) -> io::Result<()> {
        let mut fs: libc::statfs = unsafe { std::mem::zeroed() };
        let flags = unsafe { libc::fcntl(self.file.as_raw_fd(), libc::F_GETFL) };
        let descriptor = unsafe { libc::fcntl(self.file.as_raw_fd(), libc::F_GETFD) };
        require(
            unsafe { libc::fstatfs(self.file.as_raw_fd(), &mut fs) } == 0
                && fs.f_type == 0x9fa0
                && flags >= 0
                && flags & (libc::O_ACCMODE | libc::O_PATH) == libc::O_RDONLY
                && descriptor >= 0
                && descriptor & libc::FD_CLOEXEC != 0
                && self
                    .identity
                    .same_object(&Stamp::of(&self.file.metadata()?))
                && self.file.metadata()?.is_dir() == directory,
        )?;
        Ok(())
    }
}
pub(super) struct Readers {
    environment: ProcRead,
    descriptors: ProcRead,
}
impl Readers {
    fn capture(owner: &Owner) -> io::Result<Self> {
        Ok(Self {
            environment: ProcRead::capture(
                owner.relative("environ", libc::O_RDONLY | libc::O_NOFOLLOW)?,
                false,
            )?,
            descriptors: ProcRead::capture(
                owner.relative("fd", libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_DIRECTORY)?,
                true,
            )?,
        })
    }
    pub(super) fn empty_environment(&self) -> io::Result<bool> {
        self.environment.check(false)?;
        let mut byte = [0];
        let empty = self.environment.file.read_at(&mut byte, 0)? == 0;
        self.environment.check(false)?;
        Ok(empty)
    }
    pub(super) fn names(&self) -> io::Result<BTreeSet<String>> {
        self.descriptors.check(true)?;
        let fd = self.descriptors.file.as_raw_fd();
        require(unsafe { libc::lseek(fd, 0, libc::SEEK_SET) } == 0)?;
        let mut names = BTreeSet::new();
        let mut eof = false;
        for _ in 0..3 {
            let mut raw = [0u8; 512];
            let size =
                unsafe { libc::syscall(libc::SYS_getdents64, fd, raw.as_mut_ptr(), raw.len()) };
            require(size >= 0 && size as usize <= raw.len())?;
            if size == 0 {
                eof = true;
                break;
            }
            records(&raw[..size as usize], &mut names)?;
        }
        require(eof && names.remove(".") && names.remove(".."))?;
        self.descriptors.check(true)?;
        Ok(names)
    }
}
fn records(raw: &[u8], names: &mut BTreeSet<String>) -> io::Result<()> {
    require(raw.len() <= 512)?;
    let mut at = 0;
    while at < raw.len() {
        let head = raw.get(at..at + 19).ok_or_else(refused)?;
        let len = u16::from_ne_bytes([head[16], head[17]]) as usize;
        require(len >= 24 && len.is_multiple_of(8) && names.len() < 8)?;
        let bytes = raw.get(at + 19..at + len).ok_or_else(refused)?;
        let end = bytes.iter().position(|b| *b == 0).ok_or_else(refused)?;
        let name = std::str::from_utf8(&bytes[..end]).map_err(|_| refused())?;
        if name == "." || name == ".." {
            require(head[18] == libc::DT_DIR)?;
        } else {
            let number: u32 = name.parse().map_err(|_| refused())?;
            require(
                !name.is_empty()
                    && name.len() <= 10
                    && number.to_string() == name
                    && head[18] == libc::DT_LNK,
            )?;
        }
        require(names.insert(name.to_owned()))?;
        at += len;
    }
    Ok(())
}
pub(super) fn filter_state(raw: &str) -> io::Result<(u64, u64)> {
    require(!raw.is_empty() && raw.len() <= 65536 && raw.ends_with('\n'))?;
    let mut pair = [0; 2];
    for (index, key) in ["Seccomp:", "Seccomp_filters:"].into_iter().enumerate() {
        let fields: Vec<_> = raw
            .lines()
            .filter_map(|line| line.strip_prefix(key))
            .collect();
        require(fields.len() == 1)?;
        let value = fields[0].trim();
        require(!value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()))?;
        pair[index] = value.parse::<u64>().map_err(|_| refused())?;
    }
    require((pair[0] == 0 && pair[1] == 0) || (pair[0] == 2 && (1..=65).contains(&pair[1])))?;
    Ok((pair[0], pair[1]))
}
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Event {
    Entry { nr: u64, args: [u64; 6] },
    Exit { value: i64, error: bool },
}
pub(super) fn decode(info: &libc::ptrace_syscall_info, bytes: usize) -> io::Result<Event> {
    require(bytes <= std::mem::size_of_val(info) && info.flags == 0 && info.arch == 0xc000_003e)?;
    match info.op {
        1 if bytes >= 80 => {
            let entry = unsafe { info.u.entry };
            Ok(Event::Entry {
                nr: entry.nr,
                args: entry.args,
            })
        }
        2 if bytes >= 33 => {
            let exit = unsafe { info.u.exit };
            require(exit.is_error <= 1)?;
            Ok(Event::Exit {
                value: exit.sval,
                error: exit.is_error == 1,
            })
        }
        _ => Err(refused()),
    }
}
pub(super) fn memory(owner: &Owner, address: usize, count: usize) -> io::Result<Vec<u8>> {
    let (low, high) = owner.stack.ok_or_else(refused)?;
    let end = address.checked_add(count).ok_or_else(refused)?;
    let begin = address & !7;
    let padded = end.checked_add(7).ok_or_else(refused)? & !7;
    require((1..=512).contains(&count) && low <= begin && begin < padded && padded <= high)?;
    let mut raw = Vec::with_capacity(padded - begin);
    for cursor in (begin..padded).step_by(8) {
        unsafe {
            *libc::__errno_location() = 0;
        }
        let word = unsafe { libc::ptrace(libc::PTRACE_PEEKDATA, owner.pid, cursor, 0usize) };
        require(word != -1 || unsafe { *libc::__errno_location() } == 0)?;
        raw.extend_from_slice(&word.to_ne_bytes());
    }
    Ok(raw[address - begin..address - begin + count].to_vec())
}
fn policy(owner: &Owner, header: usize) -> io::Result<()> {
    let bytes = memory(owner, header, 16)?;
    require(u16::from_ne_bytes(bytes[..2].try_into().map_err(|_| refused())?) == 64)?;
    let pointer = usize::from_ne_bytes(bytes[8..16].try_into().map_err(|_| refused())?);
    require(
        memory(owner, pointer, 512)?
            == hydracache_long_run_supervisor_074::diagnostic_signer_isolation::policy_bytes(),
    )
}
// Ordered fixed shared-core sequence; no open/close/dup/exec/fork/thread calls.
fn entry(owner: &Owner, step: usize, nr: u64, args: [u64; 6]) -> io::Result<i64> {
    let expected = match step {
        0 => {
            require(
                nr == libc::SYS_prlimit64 as u64
                    && args[0] == 0
                    && args[1] == libc::RLIMIT_CORE as u64
                    && args[3] == 0,
            )?;
            require(memory(owner, args[2] as usize, 16)? == [0; 16])?;
            0
        }
        1 | 2 | 4 => {
            require(nr == libc::SYS_prctl as u64)?;
            let (option, value, result) = match step {
                1 => (libc::PR_SET_NO_NEW_PRIVS, 1, 0),
                2 => (libc::PR_GET_NO_NEW_PRIVS, 0, 1),
                4 => (libc::PR_SET_DUMPABLE, 0, 0),
                _ => return Err(refused()),
            };
            require(args[..5] == [option as u64, value, 0, 0, 0])?;
            result
        }
        3 => {
            require(
                nr == libc::SYS_seccomp as u64
                    && args[0] == libc::SECCOMP_SET_MODE_FILTER as u64
                    && args[1] == libc::SECCOMP_FILTER_FLAG_TSYNC,
            )?;
            policy(owner, args[2] as usize)?;
            0
        }
        5 => {
            require(nr == libc::SYS_write as u64 && args[0] == 3 && args[2] == 16)?;
            16
        }
        6 => {
            require(nr == libc::SYS_getpid as u64)?;
            owner.pid as i64
        }
        7 => {
            require(
                nr == libc::SYS_kill as u64
                    && args[0] == owner.pid as u64
                    && args[1] == libc::SIGSTOP as u64,
            )?;
            0
        }
        _ => return Err(refused()),
    };
    Ok(expected)
}
pub(super) fn install(owner: &mut Owner) -> io::Result<()> {
    require(owner.phase == Phase::Loaded && owner.probe.is_some())?;
    owner.context(false)?;
    let (_, before) = filter_state(&owner.text("status")?)?;
    require(before <= 64)?;
    owner.filters = Some(before);
    owner.readers = Some(Readers::capture(owner)?);
    let mut step = 0;
    let mut expected = None;
    let mut stopped = false;
    for _ in 0..128 {
        owner.trace(libc::PTRACE_SYSCALL, 0)?;
        let status = owner.wait()?;
        if status == STOP {
            require(step == 8 && expected.is_none())?;
            stopped = true;
            break;
        }
        require(status == SYSCALL)?;
        let mut info: libc::ptrace_syscall_info = unsafe { std::mem::zeroed() };
        let count = unsafe {
            libc::ptrace(
                libc::PTRACE_GET_SYSCALL_INFO,
                owner.pid,
                std::mem::size_of_val(&info),
                &mut info,
            )
        };
        require(count >= 0)?;
        let event = decode(&info, count as usize)?;
        accept(owner, &mut step, &mut expected, event)?;
    }
    require(stopped)?;
    owner.phase = Phase::Installed;
    owner.context(false)?;
    maps::validate(&owner.text("maps")?, &owner.root, true)?;
    marker(owner, INSTALLED)
}
fn accept(
    owner: &Owner,
    step: &mut usize,
    expected: &mut Option<i64>,
    event: Event,
) -> io::Result<()> {
    match event {
        Event::Entry { nr, args } => {
            require(expected.is_none())?;
            *expected = Some(entry(owner, *step, nr, args)?);
        }
        Event::Exit { value, error } => {
            require(!error && expected.take().ok_or_else(refused)? == value)?;
            *step += 1;
        }
    }
    Ok(())
}
fn marker(owner: &mut Owner, expected: &[u8; 16]) -> io::Result<()> {
    use std::io::Read;
    let mut bytes = [0; 17];
    require(owner.pipe.read(&mut bytes)? == 16 && &bytes[..16] == expected)
}
pub(super) fn finish(owner: &mut Owner) -> io::Result<()> {
    require(owner.phase == Phase::Installed)?;
    owner.context(false)?;
    owner.trace(libc::PTRACE_CONT, 0)?;
    let status = owner.wait()?;
    match owner.probe.ok_or_else(refused)? {
        FixedProbe::Native => {
            require(status == 0)?;
            marker(owner, DONE)?;
        }
        FixedProbe::X32 | FixedProbe::I386 => require(status == libc::SIGSYS)?,
    }
    // No remaining markers or writer survive completion; extra bytes refuse.
    use std::io::Read;
    let mut extra = [0];
    require(owner.pipe.read(&mut extra)? == 0)?;
    owner.terminate()?;
    owner.root.check()?;
    owner.root.cleanup()?;
    owner.phase = Phase::Terminal;
    Ok(())
}
pub(super) fn child(probe: FixedProbe) -> ! {
    install_child();
    if !super::core::exercise(probe) {
        unsafe { libc::_exit(95) }
    }
    if unsafe { libc::write(3, DONE.as_ptr().cast(), DONE.len()) } != 16 {
        unsafe { libc::_exit(94) }
    }
    unsafe { libc::_exit(0) }
}
pub(super) fn install_child() {
    let bytes = hydracache_long_run_supervisor_074::diagnostic_signer_isolation::policy_bytes();
    let mut filters = [libc::sock_filter {
        code: 0,
        jt: 0,
        jf: 0,
        k: 0,
    }; 64];
    for (item, bytes) in filters.iter_mut().zip(bytes.chunks_exact(8)) {
        *item = libc::sock_filter {
            code: u16::from_le_bytes([bytes[0], bytes[1]]),
            jt: bytes[2],
            jf: bytes[3],
            k: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        };
    }
    if super::core::install(&mut filters).is_err() {
        unsafe { libc::_exit(94) }
    }
    if unsafe { libc::write(3, INSTALLED.as_ptr().cast(), INSTALLED.len()) } != 16
        || unsafe { libc::kill(libc::getpid(), libc::SIGSTOP) } != 0
    {
        unsafe { libc::_exit(94) }
    }
}

#[cfg(test)]
#[path = "diagnostic_signer_owned_root_isolation_tests.rs"]
mod tests;
