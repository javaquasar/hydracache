use super::*;
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
pub const SCOPE: &str = "linux_owned_session_nested_groups_per_process_address_space_limit";
const SESSION_OWNER: &str = "HYDRACACHE_IMAP075_PROCESS_SESSION_OWNER";
const GROUP_CHAIN: &str = "HYDRACACHE_IMAP075_PROCESS_GROUP_CHAIN";
fn absent(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::NotFound || error.raw_os_error() == Some(libc::ESRCH)
}
fn chain() -> io::Result<Vec<i32>> {
    chain_environment(std::env::var(GROUP_CHAIN))
}
fn chain_environment(raw: Result<String, std::env::VarError>) -> io::Result<Vec<i32>> {
    match raw {
        Ok(value) => parse_chain(&value),
        Err(std::env::VarError::NotPresent) => Ok(vec![]),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err(io::Error::other("invalid process group marker encoding"))
        }
    }
}
fn parse_chain(raw: &str) -> io::Result<Vec<i32>> {
    if raw.len() > 2048 {
        return Err(io::Error::other("owned group chain byte bound"));
    }
    if raw.is_empty() {
        return Ok(vec![]);
    }
    let values = raw
        .split(',')
        .map(|s| {
            s.parse::<i32>()
                .map_err(|_| io::Error::other("owned group chain integer"))
        })
        .collect::<io::Result<Vec<_>>>()?;
    if values.len() > 128 || values.iter().any(|v| *v <= 0) {
        return Err(io::Error::other("owned group chain bound"));
    }
    Ok(values)
}
fn descendant(pid: i32, group: i32) -> io::Result<bool> {
    // Read only an operational marker from owned processes; never retain or log environment data.
    retry_marker(
        || {
            let mut bytes = Vec::new();
            std::fs::File::open(format!("/proc/{pid}/environ"))?
                .take(1024 * 1024 + 1)
                .read_to_end(&mut bytes)?;
            Ok(bytes)
        },
        || member_live(member(pid)),
        group,
    )
}
fn member_live(value: io::Result<Member>) -> io::Result<bool> {
    match value {
        Ok(value) => Ok(value.live),
        Err(error) if absent(&error) => Ok(false),
        Err(error) => Err(error),
    }
}
fn retry_marker(
    mut read: impl FnMut() -> io::Result<Vec<u8>>,
    mut live: impl FnMut() -> io::Result<bool>,
    group: i32,
) -> io::Result<bool> {
    let mut attempt = 0;
    loop {
        match read() {
            Ok(bytes) => {
                if bytes.len() > 1024 * 1024 {
                    return Err(io::Error::other("owned process environment byte bound"));
                }
                let key = format!("{GROUP_CHAIN}=");
                for entry in bytes.split(|b| *b == 0) {
                    if let Some(value) = entry.strip_prefix(key.as_bytes()) {
                        return Ok(parse_chain(
                            std::str::from_utf8(value).map_err(io::Error::other)?,
                        )?
                        .contains(&group));
                    }
                }
                return Ok(false);
            }
            Err(error) if absent(&error) => return Ok(false),
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                // procfs can deny mm access during exit/exec while the preceding stat
                // snapshot still says live. Never silently drop an inaccessible live owner.
                if !live()? {
                    return Ok(false);
                }
                if attempt == 5 {
                    return Err(error);
                }
                attempt += 1;
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(error) => return Err(error),
        }
    }
}
#[derive(Clone)]
struct Member {
    pid: i32,
    parent: i32,
    group: i32,
    session: i32,
    birth: u64,
    live: bool,
}
fn member(pid: i32) -> io::Result<Member> {
    let mut text = String::new();
    std::fs::File::open(format!("/proc/{pid}/stat"))?
        .take(4097)
        .read_to_string(&mut text)?;
    parse_member(pid, &text)
}
fn parse_member(pid: i32, text: &str) -> io::Result<Member> {
    if text.len() > 4096 {
        return Err(io::Error::other("process stat bound"));
    }
    let (_, tail) = text
        .rsplit_once(')')
        .ok_or_else(|| io::Error::other("process stat malformed"))?;
    let fields = tail.split_whitespace().collect::<Vec<_>>();
    let invalid = || io::Error::other("process stat fields malformed");
    Ok(Member {
        pid,
        parent: fields
            .get(1)
            .ok_or_else(invalid)?
            .parse()
            .map_err(|_| invalid())?,
        group: fields
            .get(2)
            .ok_or_else(invalid)?
            .parse()
            .map_err(|_| invalid())?,
        session: fields
            .get(3)
            .ok_or_else(invalid)?
            .parse()
            .map_err(|_| invalid())?,
        birth: fields
            .get(19)
            .ok_or_else(invalid)?
            .parse()
            .map_err(|_| invalid())?,
        live: !matches!(fields.first(), Some(&"Z" | &"X")),
    })
}
fn inventory() -> io::Result<Vec<Member>> {
    inventory_from(
        std::fs::read_dir("/proc")?.map(|entry| entry.map(|entry| entry.file_name())),
        member,
    )
}
fn inventory_from(
    entries: impl IntoIterator<Item = io::Result<std::ffi::OsString>>,
    mut read: impl FnMut(i32) -> io::Result<Member>,
) -> io::Result<Vec<Member>> {
    let mut result = Vec::new();
    for (index, entry) in entries.into_iter().enumerate() {
        if index >= 65536 {
            return Err(io::Error::other("process inventory bound"));
        }
        let entry = entry?;
        let Some(pid) = entry.to_str().and_then(|s| s.parse::<i32>().ok()) else {
            continue;
        };
        match read(pid) {
            Ok(value) => result.push(value),
            Err(error) if absent(&error) || error.kind() == io::ErrorKind::PermissionDenied => {}
            Err(error) => return Err(error),
        }
    }
    Ok(result)
}
fn inherited_session() -> io::Result<Option<i32>> {
    inherited_session_from(
        std::env::var_os(SESSION_OWNER),
        std::process::id() as i32,
        member,
    )
}
fn inherited_session_from(
    marker: Option<std::ffi::OsString>,
    current_pid: i32,
    mut read: impl FnMut(i32) -> io::Result<Member>,
) -> io::Result<Option<i32>> {
    let Some(marker) = marker else {
        return Ok(None);
    };
    let marker = marker
        .to_str()
        .ok_or_else(|| io::Error::other("invalid process session marker"))?;
    let (pid, birth) = marker
        .split_once(':')
        .ok_or_else(|| io::Error::other("invalid process session marker"))?;
    let owner = read(
        pid.parse()
            .map_err(|_| io::Error::other("invalid process owner PID"))?,
    )?;
    let current = read(current_pid)?;
    let leader = read(current.session)?;
    if birth.parse::<u64>().ok() != Some(owner.birth)
        || !owner.live
        || leader.parent != owner.pid
        || owner.session == current.session
    {
        return Err(io::Error::other("foreign/stale process session marker"));
    }
    Ok(Some(current.session))
}
pub trait Pipe: AsRawFd {}
impl<T: AsRawFd> Pipe for T {}
pub struct Tree {
    group: i32,
    session: Option<i32>,
    nested_session: Option<i32>,
    terminated: bool,
    finished: bool,
}
fn configure_session(
    command: &mut Command,
    inherited: Option<i32>,
    owner: impl FnOnce() -> io::Result<Member>,
    parents: impl FnOnce() -> io::Result<Vec<i32>>,
    group: i32,
) -> io::Result<()> {
    if inherited.is_some() {
        command.process_group(0);
        let mut parents = parents()?;
        if parents.last() != Some(&group) {
            parents.push(group);
        }
        command.env(
            GROUP_CHAIN,
            parents
                .iter()
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(","),
        );
    } else {
        let owner = owner()?;
        command.env(SESSION_OWNER, format!("{}:{}", owner.pid, owner.birth));
        command.env_remove(GROUP_CHAIN);
    }
    Ok(())
}
fn child_setup(
    root_session: bool,
    memory: libc::rlim_t,
    mut setsid: impl FnMut() -> libc::pid_t,
    mut set_limit: impl FnMut(&libc::rlimit) -> libc::c_int,
) -> io::Result<()> {
    if root_session && setsid() < 0 {
        return Err(io::Error::last_os_error());
    }
    let limit = libc::rlimit {
        rlim_cur: memory,
        rlim_max: memory,
    };
    if set_limit(&limit) == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
fn child_session(inherited: Option<i32>, pid: u32) -> Option<i32> {
    if inherited.is_none() {
        Some(pid as i32)
    } else {
        None
    }
}
fn reap_after_drain(
    remaining: usize,
    reap: impl FnOnce() -> io::Result<Option<ExitStatus>>,
) -> io::Result<Option<ExitStatus>> {
    if remaining != 0 {
        return Ok(None);
    }
    reap()
}
impl Tree {
    pub fn spawn(command: &mut Command, bounds: ProcessBounds) -> io::Result<(Self, Child)> {
        // Cooperative nested controllers use separate groups in the root's session.
        // The root owns all those groups, including when an inner controller is killed.
        // This is process ownership, not isolation against a program deliberately calling setsid.
        let inherited = inherited_session()?;
        configure_session(
            command,
            inherited,
            || member(std::process::id() as i32),
            chain,
            unsafe { libc::getpgrp() },
        )?;
        let memory = bounds.memory_bytes as libc::rlim_t;
        // Only an async-signal-safe syscall is used after fork, before exec.
        unsafe {
            command.pre_exec(move || {
                child_setup(
                    inherited.is_none(),
                    memory,
                    || libc::setsid(),
                    |limit| libc::setrlimit(libc::RLIMIT_AS, limit),
                )
            });
        }
        let child = command.spawn()?;
        Ok((
            Self {
                group: child.id() as i32,
                session: child_session(inherited, child.id()),
                nested_session: inherited,
                terminated: false,
                finished: false,
            },
            child,
        ))
    }
    pub fn terminate(&mut self) -> io::Result<()> {
        self.terminate_using(Self::owned_members, |group, signal| unsafe {
            libc::kill(group, signal)
        })
    }
    fn terminate_using(
        &mut self,
        members: impl FnOnce(&Self) -> io::Result<Vec<Member>>,
        mut kill: impl FnMut(libc::pid_t, libc::c_int) -> libc::c_int,
    ) -> io::Result<()> {
        if self.finished {
            return Ok(());
        }
        let groups = members(self)?
            .into_iter()
            .map(|m| m.group)
            .collect::<std::collections::BTreeSet<_>>();
        if kill(-self.group, libc::SIGKILL) != 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(error);
            }
        }
        for group in groups {
            if group <= 0 {
                return Err(io::Error::other("invalid owned process group"));
            }
            if kill(-group, libc::SIGKILL) != 0 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() != Some(libc::ESRCH) {
                    return Err(error);
                }
            }
        }
        self.terminated = true;
        Ok(())
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
        reap_after_drain(self.live_members()?, || child.try_wait())
    }
    pub fn live_members(&mut self) -> io::Result<usize> {
        if self.finished {
            return Ok(0);
        }
        let count = self.owned_members()?.len();
        // Release ownership while the root PID is still reserved by its zombie.
        // Drop must never signal a reused group/session after the root is reaped.
        if count == 0 && self.terminated {
            self.finished = true;
        }
        Ok(count)
    }
    fn owned_members(&self) -> io::Result<Vec<Member>> {
        self.owned_members_using(inventory, descendant)
    }
    fn owned_members_using(
        &self,
        inventory: impl FnOnce() -> io::Result<Vec<Member>>,
        mut descendant: impl FnMut(i32, i32) -> io::Result<bool>,
    ) -> io::Result<Vec<Member>> {
        let mut result = Vec::new();
        for value in inventory()? {
            if !value.live {
                continue;
            }
            if self.session == Some(value.session) || value.group == self.group {
                result.push(value);
                continue;
            }
            if self.nested_session == Some(value.session) {
                match descendant(value.pid, self.group) {
                    Ok(true) => result.push(value),
                    Ok(false) => {}
                    Err(error) if absent(&error) => {}
                    Err(error) => return Err(error),
                }
            }
        }
        Ok(result)
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
#[cfg(test)]
#[path = "../tests/support/imap_proof_process_linux_075.rs"]
mod tests;
