use super::*;
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
pub const SCOPE: &str = "linux_inherited_process_group_per_process_address_space_limit";
pub trait Pipe: AsRawFd {}
impl<T: AsRawFd> Pipe for T {}
pub struct Tree {
    group: i32,
    terminated: bool,
}
impl Tree {
    pub fn spawn(command: &mut Command, bounds: ProcessBounds) -> io::Result<(Self, Child)> {
        command.process_group(0);
        let memory = bounds.memory_bytes as libc::rlim_t;
        // Only an async-signal-safe syscall is used after fork, before exec.
        unsafe {
            command.pre_exec(move || {
                let limit = libc::rlimit {
                    rlim_cur: memory,
                    rlim_max: memory,
                };
                if libc::setrlimit(libc::RLIMIT_AS, &limit) == 0 {
                    Ok(())
                } else {
                    Err(io::Error::last_os_error())
                }
            });
        }
        let child = command.spawn()?;
        Ok((
            Self {
                group: child.id() as i32,
                terminated: false,
            },
            child,
        ))
    }
    pub fn terminate(&mut self) -> io::Result<()> {
        if self.terminated {
            return Ok(());
        }
        if unsafe { libc::kill(-self.group, libc::SIGKILL) } == 0 {
            self.terminated = true;
            Ok(())
        } else {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ESRCH) {
                self.terminated = true;
                Ok(())
            } else {
                Err(error)
            }
        }
    }
    pub fn poll_exit(&mut self, child: &mut Child) -> io::Result<Option<ExitStatus>> {
        // Keep the root zombie until its owned process group is terminated. This
        // prevents PID/group reuse between observing exit and killing descendants.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        if unsafe {
            libc::waitid(
                libc::P_PID,
                child.id(),
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        if unsafe { info.si_pid() } == 0 {
            return Ok(None);
        }
        self.terminate()?;
        child.try_wait()
    }
    pub fn live_members(&self) -> io::Result<usize> {
        let mut live = 0;
        let mut examined = 0;
        for entry in std::fs::read_dir("/proc")? {
            examined += 1;
            if examined > 65_536 {
                return Err(io::Error::other("process inventory bound"));
            }
            let entry = entry?;
            if !entry
                .file_name()
                .to_string_lossy()
                .bytes()
                .all(|b| b.is_ascii_digit())
            {
                continue;
            }
            let file = match std::fs::File::open(entry.path().join("stat")) {
                Ok(file) => file,
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
                    ) =>
                {
                    continue
                }
                Err(error) => return Err(error),
            };
            let mut stat = String::new();
            if file.take(4097).read_to_string(&mut stat).is_err() {
                continue;
            }
            if stat.len() > 4096 {
                return Err(io::Error::other("process stat bound"));
            }
            let Some((_, tail)) = stat.rsplit_once(')') else {
                continue;
            };
            let mut fields = tail.split_whitespace();
            let state = fields.next();
            fields.next();
            let group = fields.next().and_then(|s| s.parse::<i32>().ok());
            // Zombies are already terminated and own no inherited pipe handles.
            // Their reaping belongs to their current parent; we reap our own child.
            if group == Some(self.group) && !matches!(state, Some("Z" | "X")) {
                live += 1;
            }
        }
        Ok(live)
    }
}
impl Drop for Tree {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}
pub fn prepare_pipe(pipe: &impl Pipe) -> io::Result<()> {
    let fd = pipe.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
pub fn available(_: &impl Pipe) -> io::Result<Option<usize>> {
    Ok(Some(8192))
}
