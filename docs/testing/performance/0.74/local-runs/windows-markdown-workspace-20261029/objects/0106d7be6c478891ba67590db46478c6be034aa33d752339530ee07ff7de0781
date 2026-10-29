//! Original creating-task proc object, not numeric TID adoption or production custody.
use super::*;
pub(super) struct Creator {
    pub(super) task: File,
    pub(super) tgid: libc::pid_t,
    pub(super) tid: libc::pid_t,
}
impl Creator {
    pub(super) fn capture(tgid: libc::pid_t) -> io::Result<Self> {
        // SAFETY: original calling thread only, captured before its fork.
        let tid = unsafe { libc::syscall(libc::SYS_gettid) } as libc::pid_t;
        // Deliberately follow this fixed proc magic link, never a caller path or supplied TID.
        let task = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC)
            .open("/proc/thread-self")?;
        if !directory(&task) || !live(task.as_raw_fd(), tgid, tid) {
            return Err(invalid());
        }
        Ok(Self { task, tgid, tid })
    }
}
pub(super) fn directory(task: &File) -> bool {
    // SAFETY: initialized fixed kernel output on an original retained directory.
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    let (ok, flags) = unsafe {
        (
            libc::fstatfs(task.as_raw_fd(), &mut stat),
            libc::fcntl(task.as_raw_fd(), libc::F_GETFL),
        )
    };
    ok == 0
        && stat.f_type == libc::PROC_SUPER_MAGIC
        && flags >= 0
        && flags & (libc::O_ACCMODE | libc::O_PATH) == libc::O_RDONLY
        && task.metadata().is_ok_and(|m| m.is_dir())
        && unsafe { libc::fcntl(task.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC != 0
}
fn trim(bytes: &[u8]) -> &[u8] {
    let first = bytes
        .iter()
        .position(|b| !matches!(b, b' ' | b'\t'))
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|b| !matches!(b, b' ' | b'\t'))
        .map_or(first, |n| n + 1);
    &bytes[first..end]
}
fn decimal(bytes: &[u8]) -> Option<i32> {
    let bytes = trim(bytes);
    if bytes.is_empty() {
        return None;
    }
    let mut value = 0i32;
    for b in bytes {
        if !b.is_ascii_digit() {
            return None;
        }
        value = value.checked_mul(10)?.checked_add((b - b'0') as i32)?;
    }
    (value > 0).then_some(value)
}
pub(super) fn status(bytes: &[u8], tgid: i32, tid: i32) -> bool {
    if bytes.is_empty()
        || bytes.len() > 8192
        || bytes.last() != Some(&b'\n')
        || tgid <= 0
        || tid <= 0
    {
        return false;
    }
    let (mut group, mut task, mut state) = (None, None, None);
    for line in bytes.split(|b| *b == b'\n') {
        if let Some(value) = line.strip_prefix(b"Tgid:") {
            if group.is_some() {
                return false;
            }
            group = decimal(value);
            if group.is_none() {
                return false;
            }
        } else if let Some(value) = line.strip_prefix(b"Pid:") {
            if task.is_some() {
                return false;
            }
            task = decimal(value);
            if task.is_none() {
                return false;
            }
        } else if let Some(value) = line.strip_prefix(b"State:") {
            if state.is_some() {
                return false;
            }
            let value = trim(value);
            state = Some(
                matches!(value.first(), Some(b'R' | b'S' | b'D' | b'T' | b't' | b'I'))
                    && value.get(1).is_some_and(u8::is_ascii_whitespace),
            );
        }
    }
    group == Some(tgid) && task == Some(tid) && state == Some(true)
}
pub(super) fn live(task: i32, tgid: i32, tid: i32) -> bool {
    let mut bytes = [0u8; 8193];
    // SAFETY: original inherited task directory, fixed status component, bounded stack outputs.
    unsafe {
        let fd = libc::openat(
            task,
            c"status".as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        );
        if fd < 0 {
            return false;
        }
        let mut used = 0usize;
        let mut complete = false;
        for _ in 0..8 {
            let n = libc::read(fd, bytes[used..].as_mut_ptr().cast(), bytes.len() - used);
            if n < 0 {
                break;
            }
            if n == 0 {
                complete = true;
                break;
            }
            used += n as usize;
            if used > 8192 {
                break;
            }
        }
        let closed = libc::close(fd) == 0;
        complete && closed && status(&bytes[..used], tgid, tid)
    }
}
