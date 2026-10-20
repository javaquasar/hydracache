//! Owned local process cuts, no original-frame or whole-cgroup admission claim.
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Child, Command, Stdio};
const NS: u64 = 1_000_000_000;
const SEED: u64 = 0x0074_e7ec;
fn now() -> u64 {
    let mut t = unsafe { std::mem::zeroed::<libc::timespec>() };
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut t) },
        0
    );
    t.tv_sec as u64 * NS + t.tv_nsec as u64
}
fn pin(pid: u32) -> File {
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0u32) };
    assert!(fd >= 0);
    unsafe { File::from_raw_fd(fd as _) }
}
fn signal(fd: &File, sig: i32) {
    let n = unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            fd.as_raw_fd(),
            sig,
            0usize,
            0u32,
        )
    };
    assert!(n == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH));
}
fn pair() -> [File; 2] {
    let mut fds = [-1; 2];
    assert_eq!(
        unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC,
                0,
                fds.as_mut_ptr(),
            )
        },
        0
    );
    fds.map(|fd| unsafe { File::from_raw_fd(fd) })
}
fn pipe() -> [File; 2] {
    let mut fds = [-1; 2];
    assert_eq!(unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) }, 0);
    fds.map(|fd| unsafe { File::from_raw_fd(fd) })
}
fn ready(fd: RawFd, ms: i32) -> i16 {
    let mut p = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    assert!(unsafe { libc::poll(&mut p, 1, ms) } >= 0);
    p.revents
}
fn readonly(fd: &File) -> File {
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC)
        .open(format!("/proc/self/fd/{}", fd.as_raw_fd()))
        .unwrap()
}
fn memfd(raw: &[u8], seals: bool, executable: bool) -> File {
    let fd = unsafe {
        libc::memfd_create(
            c"hc074-test".as_ptr(),
            libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING,
        )
    };
    assert!(fd >= 0);
    let mut f = unsafe { File::from_raw_fd(fd) };
    f.write_all(raw).unwrap();
    assert_eq!(
        unsafe { libc::fchmod(fd, if executable { 0o500 } else { 0o400 }) },
        0
    );
    if seals {
        assert_eq!(
            unsafe {
                libc::fcntl(
                    fd,
                    libc::F_ADD_SEALS,
                    libc::F_SEAL_SEAL
                        | libc::F_SEAL_GROW
                        | libc::F_SEAL_SHRINK
                        | libc::F_SEAL_WRITE,
                )
            },
            0
        );
    }
    readonly(&f)
}
fn elf() -> Vec<u8> {
    // One RX PT_LOAD; direct syscalls only: marker, SIGSTOP, stdin read, exit.
    let base = 0x400000u64;
    let mut code = Vec::new();
    fn mov(code: &mut Vec<u8>, op: u8, value: u32) {
        code.push(op);
        code.extend(value.to_le_bytes());
    }
    mov(&mut code, 0xb8, 1);
    mov(&mut code, 0xbf, 1);
    code.extend([0x48, 0xbe]);
    let ptr = code.len();
    code.extend([0; 8]);
    mov(&mut code, 0xba, 6);
    code.extend([0x0f, 0x05]);
    mov(&mut code, 0xb8, 39);
    code.extend([0x0f, 0x05, 0x89, 0xc7]);
    mov(&mut code, 0xbe, libc::SIGSTOP as u32);
    mov(&mut code, 0xb8, 62);
    code.extend([0x0f, 0x05]);
    mov(&mut code, 0xb8, 0);
    mov(&mut code, 0xbf, 0);
    code.extend([0x48, 0xbe]);
    let buf = code.len();
    code.extend([0; 8]);
    mov(&mut code, 0xba, 1);
    code.extend([0x0f, 0x05]);
    mov(&mut code, 0xbf, 0);
    mov(&mut code, 0xb8, 60);
    code.extend([0x0f, 0x05]);
    let data = base + 120 + code.len() as u64;
    code[ptr..ptr + 8].copy_from_slice(&data.to_le_bytes());
    // Read buffer is on the process stack; avoid a write into the RX image.
    code[buf - 2..buf].copy_from_slice(&[0x48, 0xbe]); // replaced below with mov rsi,rsp
    code.splice(buf - 2..buf + 8, [0x48, 0x89, 0xe6]);
    let data = base + 120 + code.len() as u64;
    code[ptr..ptr + 8].copy_from_slice(&data.to_le_bytes());
    code.extend(b"effect");
    let mut raw = vec![0; 120];
    raw[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    raw[16..18].copy_from_slice(&2u16.to_le_bytes());
    raw[18..20].copy_from_slice(&62u16.to_le_bytes());
    raw[20..24].copy_from_slice(&1u32.to_le_bytes());
    raw[24..32].copy_from_slice(&(base + 120).to_le_bytes());
    raw[32..40].copy_from_slice(&64u64.to_le_bytes());
    raw[52..54].copy_from_slice(&64u16.to_le_bytes());
    raw[54..56].copy_from_slice(&56u16.to_le_bytes());
    raw[56..58].copy_from_slice(&1u16.to_le_bytes());
    raw[64..68].copy_from_slice(&1u32.to_le_bytes());
    raw[68..72].copy_from_slice(&5u32.to_le_bytes());
    raw[80..88].copy_from_slice(&base.to_le_bytes());
    raw[88..96].copy_from_slice(&base.to_le_bytes());
    let len = raw.len() + code.len();
    raw[96..104].copy_from_slice(&(len as u64).to_le_bytes());
    raw[104..112].copy_from_slice(&(len as u64).to_le_bytes());
    raw[112..120].copy_from_slice(&4096u64.to_le_bytes());
    raw.extend(code);
    raw
}
fn id(file: &File) -> [u64; 3] {
    let m = file.metadata().unwrap();
    [m.dev(), m.ino(), m.len()]
}
fn put(raw: &mut [u8], at: usize, values: &[u64]) {
    for (i, v) in values.iter().enumerate() {
        raw[at + i * 8..at + i * 8 + 8].copy_from_slice(&v.to_le_bytes());
    }
}
fn ticks(pid: u32) -> String {
    let raw = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
    raw.rsplit_once(") ")
        .unwrap()
        .1
        .split_whitespace()
        .nth(19)
        .unwrap()
        .into()
}
struct Sleeper(Child, File);
impl Sleeper {
    fn new() -> Self {
        let child = Command::new("/bin/sleep")
            .arg("120")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let p = pin(child.id());
        Self(child, p)
    }
}
impl Drop for Sleeper {
    fn drop(&mut self) {
        signal(&self.1, libc::SIGKILL);
        self.0.wait().unwrap();
    }
}
struct Attempt {
    child: Child,
    pidfd: File,
    socket: File,
    output: File,
    input: File,
    _error: File,
    guards: [Sleeper; 2],
    alarm: File,
    packet: [u8; 80],
    deadline: u64,
    ticks: String,
    exe: [u64; 2],
    consumed: bool,
    status: Option<std::process::ExitStatus>,
}
impl Attempt {
    fn new(case: &str) -> Self {
        let guards = [Sleeper::new(), Sleeper::new()];
        let [socket, peer] = pair();
        let [stdin, input] = pipe();
        let stdin = if case == "opath-stdin" {
            OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_PATH | libc::O_CLOEXEC)
                .open(format!("/proc/self/fd/{}", stdin.as_raw_fd()))
                .unwrap()
        } else {
            stdin
        };
        let [output, stdout] = pipe();
        let [error, stderr] = pipe();
        let worker = if case == "root-file-wrong-hash" {
            let f = File::open("/usr/bin/true").unwrap();
            assert_eq!(f.metadata().unwrap().uid(), 0);
            f
        } else {
            memfd(&elf(), case != "unsealed-worker", case != "exec-refused")
        };
        let config = memfd(b"config\n", case != "unsealed-config", false);
        let owner = pin(std::process::id());
        let deadline = now()
            + if case == "expiry" {
                300_000_000
            } else {
                10 * NS
            };
        let timer = unsafe {
            libc::timerfd_create(libc::CLOCK_BOOTTIME, libc::TFD_CLOEXEC | libc::TFD_NONBLOCK)
        };
        assert!(timer >= 0);
        let timer = unsafe { File::from_raw_fd(timer) };
        let spec = libc::itimerspec {
            it_interval: libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            it_value: libc::timespec {
                tv_sec: (deadline / NS) as _,
                tv_nsec: (deadline % NS) as _,
            },
        };
        assert_eq!(
            unsafe {
                libc::timerfd_settime(
                    timer.as_raw_fd(),
                    libc::TFD_TIMER_ABSTIME,
                    &spec,
                    std::ptr::null_mut(),
                )
            },
            0
        );
        let mut boot = [0; 256];
        boot[..8].copy_from_slice(b"HCHL0741");
        put(&mut boot, 8, &[deadline]);
        for (at, v) in [
            (16, std::process::id()),
            (20, unsafe { libc::geteuid() }),
            (24, unsafe { libc::getegid() }),
            (28, guards[0].0.id()),
            (32, guards[1].0.id()),
        ] {
            boot[at..at + 4].copy_from_slice(&v.to_le_bytes());
        }
        boot[40..72].fill(74);
        boot[72..104].copy_from_slice(&Sha256::digest(elf()));
        boot[104..136].copy_from_slice(&Sha256::digest(b"config\n"));
        put(&mut boot, 136, &id(&worker));
        put(&mut boot, 160, &id(&config));
        for (i, file) in [&stdin, &stdout, &stderr].iter().enumerate() {
            put(&mut boot, 184 + i * 16, &id(file)[..2]);
        }
        put(&mut boot, 232, &id(&peer)[..2]);
        if case == "bad-hash" {
            boot[72] ^= 1;
        }
        if case == "bad-config-hash" {
            boot[104] ^= 1;
        }
        if case == "bad-stream" {
            boot[184] ^= 1;
        }
        if case == "bad-pidfd" {
            boot[28..32].copy_from_slice(&999_999u32.to_le_bytes());
        }
        let digest: [u8; 32] = Sha256::digest(boot).into();
        let bootstrap = memfd(&boot, true, false);
        let alarm = timer.try_clone().unwrap();
        let roles = [
            peer,
            worker,
            config,
            owner,
            guards[0].1.try_clone().unwrap(),
            guards[1].1.try_clone().unwrap(),
            timer,
            bootstrap,
        ];
        let high: Vec<File> = roles
            .iter()
            .map(|f| {
                let fd = unsafe { libc::fcntl(f.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 64) };
                assert!(fd >= 64);
                unsafe { File::from_raw_fd(fd) }
            })
            .collect();
        let mapping: Vec<_> = high.iter().map(AsRawFd::as_raw_fd).collect();
        let mut command = Command::new(env!("CARGO_BIN_EXE_diagnostic_held_launcher"));
        command
            .env_clear()
            .stdin(Stdio::from(stdin))
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
        unsafe {
            command.pre_exec(move || {
                for (i, fd) in mapping.iter().enumerate() {
                    if libc::dup2(*fd, 3 + i as i32) < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                }
                Ok(())
            });
        }
        let child = command.spawn().unwrap();
        drop(command);
        drop(high);
        drop(roles);
        let pidfd = pin(child.id());
        let start = ticks(child.id());
        // Before Ready there can still be an image transition from Command's fork;
        // take the actual helper image only after its Ready control below.
        let mut packet = [0; 80];
        packet[..8].copy_from_slice(b"HCLR0741");
        packet[8] = 2;
        packet[16..48].fill(74);
        packet[48..].copy_from_slice(&digest);
        Self {
            child,
            pidfd,
            socket,
            output,
            input,
            _error: error,
            guards,
            alarm,
            packet,
            deadline,
            ticks: start,
            exe: [0; 2],
            consumed: false,
            status: None,
        }
    }
    fn prepared(&mut self) {
        assert_eq!(
            ready(self.socket.as_raw_fd(), 5000),
            libc::POLLIN,
            "helper refused before Ready"
        );
        let mut raw = [0; 80];
        assert_eq!(
            unsafe {
                libc::recv(
                    self.socket.as_raw_fd(),
                    raw.as_mut_ptr().cast(),
                    80,
                    libc::MSG_TRUNC,
                )
            },
            80
        );
        let mut expected = self.packet;
        expected[8] = 1;
        assert_eq!(raw, expected);
        let m = std::fs::metadata(format!("/proc/{}/exe", self.child.id())).unwrap();
        self.exe = [m.dev(), m.ino()];
        assert_eq!(ticks(self.child.id()), self.ticks);
        assert_eq!(ready(self.output.as_raw_fd(), 0), 0);
    }
    fn send(&mut self, raw: &[u8]) -> bool {
        if self.consumed {
            return false;
        }
        self.consumed = true;
        (unsafe {
            libc::send(
                self.socket.as_raw_fd(),
                raw.as_ptr().cast(),
                raw.len(),
                libc::MSG_NOSIGNAL,
            )
        }) == raw.len() as isize
    }
    fn release(&mut self) {
        let p = self.packet;
        assert!(self.send(&p));
        assert!(!self.send(&p));
    }
    fn stopped(&mut self) {
        assert_ne!(ready(self.output.as_raw_fd(), 5000) & libc::POLLIN, 0);
        let mut data = [0; 6];
        self.output.read_exact(&mut data).unwrap();
        assert_eq!(&data, b"effect");
        let mut status = 0;
        assert_eq!(
            unsafe { libc::waitpid(self.child.id() as _, &mut status, libc::WUNTRACED) },
            self.child.id() as i32
        );
        assert!(libc::WIFSTOPPED(status));
        assert_eq!(ticks(self.child.id()), self.ticks);
        let m = std::fs::metadata(format!("/proc/{}/exe", self.child.id())).unwrap();
        assert_ne!([m.dev(), m.ino()], self.exe);
        let mut fds: Vec<_> = std::fs::read_dir(format!("/proc/{}/fd", self.child.id()))
            .unwrap()
            .map(|e| {
                e.unwrap()
                    .file_name()
                    .to_string_lossy()
                    .parse::<i32>()
                    .unwrap()
            })
            .collect();
        fds.sort_unstable();
        assert_eq!(fds, vec![0, 1, 2, 5]);
        let args = std::fs::read(format!("/proc/{}/cmdline", self.child.id())).unwrap();
        let args: Vec<_> = args.split(|b| *b == 0).collect();
        assert_eq!(args.len(), 6);
        assert_eq!(
            args[0],
            hydracache_long_run_supervisor_074::diagnostic_lease::BINARY_PATH.as_bytes()
        );
        assert_eq!(args[1], b"--run");
        assert_eq!(args[2], b"/proc/self/fd/5");
        let digest: String = Sha256::digest(elf())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(args[3], digest.as_bytes());
        assert_eq!(
            args[4],
            hydracache_long_run_supervisor_074::diagnostic_lease::SOURCE_COMMIT.as_bytes()
        );
        assert!(args[5].is_empty());
        assert!(std::fs::read(format!("/proc/{}/environ", self.child.id()))
            .unwrap()
            .is_empty());
        println!("held_exec_observed pid={} start_ticks={} helper_image={}:{} worker_image={}:{} retained_fds=0,1,2,5 argv_verified=true environment_empty=true",self.child.id(),self.ticks,self.exe[0],self.exe[1],m.dev(),m.ino());
    }
    fn reap(&mut self) -> std::process::ExitStatus {
        if let Some(s) = self.status {
            return s;
        }
        assert_ne!(ready(self.pidfd.as_raw_fd(), 5000) & libc::POLLIN, 0);
        let s = self.child.wait().unwrap();
        self.status = Some(s);
        s
    }
    fn empty(&mut self) {
        assert_eq!(self.reap().code(), Some(2));
        let mut bytes = Vec::new();
        self.output.read_to_end(&mut bytes).unwrap();
        assert!(bytes.is_empty());
    }
}
impl Drop for Attempt {
    fn drop(&mut self) {
        if self.status.is_none() {
            signal(&self.pidfd, libc::SIGKILL);
            let _ = self.child.wait();
        }
    }
}

fn foreign_send(fd: RawFd, packet: &[u8; 80]) {
    let block = u64::MAX;
    let mut previous = 0u64;
    assert_eq!(
        unsafe {
            libc::syscall(
                libc::SYS_rt_sigprocmask,
                libc::SIG_SETMASK,
                &block,
                &mut previous,
                8usize,
            )
        },
        0
    );
    let pid = unsafe { libc::syscall(libc::SYS_fork) };
    if pid == 0 {
        unsafe {
            let n = libc::send(fd, packet.as_ptr().cast(), 80, libc::MSG_NOSIGNAL);
            libc::_exit(if n == 80 { 0 } else { 2 });
        }
    }
    assert_eq!(
        unsafe {
            libc::syscall(
                libc::SYS_rt_sigprocmask,
                libc::SIG_SETMASK,
                &previous,
                0usize,
                8usize,
            )
        },
        0
    );
    assert!(pid > 0);
    let mut status = 0;
    assert_eq!(
        unsafe { libc::waitpid(pid as _, &mut status, 0) },
        pid as i32
    );
    assert_eq!(status, 0);
}
fn owner_loss_child() -> ! {
    let a = Attempt::new("healthy");
    let mut a = a;
    a.prepared();
    let rights = [
        a.pidfd.as_raw_fd(),
        a.guards[0].1.as_raw_fd(),
        a.guards[1].1.as_raw_fd(),
        a.output.as_raw_fd(),
    ];
    let mut control = [0usize; 16];
    let mut iov = libc::iovec {
        iov_base: b"HCLF0741".as_ptr() as *mut _,
        iov_len: 8,
    };
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = control.as_mut_ptr().cast();
    msg.msg_controllen = unsafe { libc::CMSG_SPACE(16) } as _;
    unsafe {
        let c = libc::CMSG_FIRSTHDR(&msg);
        (*c).cmsg_level = libc::SOL_SOCKET;
        (*c).cmsg_type = libc::SCM_RIGHTS;
        (*c).cmsg_len = libc::CMSG_LEN(16) as _;
        std::ptr::copy_nonoverlapping(rights.as_ptr(), libc::CMSG_DATA(c).cast::<RawFd>(), 4);
        assert_eq!(libc::sendmsg(64, &msg, libc::MSG_NOSIGNAL), 8);
        assert_eq!(libc::kill(libc::getpid(), libc::SIGSTOP), 0);
    }
    panic!("owner unexpectedly resumed")
}
struct OwnerHarness {
    child: Child,
    pin: File,
    targets: Vec<File>,
    subreaper: i32,
}
impl Drop for OwnerHarness {
    fn drop(&mut self) {
        signal(&self.pin, libc::SIGKILL);
        self.child.wait().unwrap();
        for target in &self.targets {
            signal(target, libc::SIGKILL);
            loop {
                let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
                if unsafe {
                    libc::waitid(
                        libc::P_PIDFD,
                        target.as_raw_fd() as _,
                        &mut info,
                        libc::WEXITED,
                    )
                } == 0
                {
                    break;
                }
                if std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                    continue;
                }
                panic!("owned orphan not reaped");
            }
        }
        assert_eq!(
            unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, self.subreaper, 0, 0, 0) },
            0
        );
    }
}
fn owner_loss() {
    let mut previous = 0i32;
    assert_eq!(
        unsafe { libc::prctl(libc::PR_GET_CHILD_SUBREAPER, &mut previous, 0, 0, 0) },
        0
    );
    assert_eq!(
        unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) },
        0
    );
    let [outer, inner] = pair();
    let yes = 1i32;
    assert_eq!(
        unsafe {
            libc::setsockopt(
                outer.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PASSCRED,
                (&yes as *const i32).cast(),
                4,
            )
        },
        0
    );
    let source = unsafe { libc::fcntl(inner.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 65) };
    assert!(source >= 65);
    let source = unsafe { File::from_raw_fd(source) };
    let fd = source.as_raw_fd();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .env_clear()
        .env("HC074_HELD_OWNER_CHILD", "1")
        .args([
            "--ignored",
            "--exact",
            "owned_launcher_process_fault_matrix",
            "--nocapture",
            "--test-threads=1",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    unsafe {
        command.pre_exec(move || {
            if libc::dup2(fd, 64) != 64 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let child = command.spawn().unwrap();
    drop(command);
    drop(source);
    drop(inner);
    let p = pin(child.id());
    let mut owner = OwnerHarness {
        child,
        pin: p,
        targets: vec![],
        subreaper: previous,
    };
    assert_eq!(ready(outer.as_raw_fd(), 5000), libc::POLLIN);
    let mut raw = [0; 8];
    let mut control = [0usize; 16];
    let mut iov = libc::iovec {
        iov_base: raw.as_mut_ptr().cast(),
        iov_len: 8,
    };
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = control.as_mut_ptr().cast();
    msg.msg_controllen = std::mem::size_of_val(&control);
    let count = unsafe {
        libc::recvmsg(
            outer.as_raw_fd(),
            &mut msg,
            libc::MSG_CMSG_CLOEXEC | libc::MSG_TRUNC,
        )
    };
    let mut credentials = 0;
    let mut rights = vec![];
    unsafe {
        let mut c = libc::CMSG_FIRSTHDR(&msg);
        while !c.is_null() {
            assert_eq!((*c).cmsg_level, libc::SOL_SOCKET);
            if (*c).cmsg_type == libc::SCM_RIGHTS {
                let len = (*c).cmsg_len - libc::CMSG_LEN(0) as usize;
                assert_eq!(len % 4, 0);
                for i in 0..len / 4 {
                    rights.push(File::from_raw_fd(
                        libc::CMSG_DATA(c).cast::<RawFd>().add(i).read_unaligned(),
                    ));
                }
            } else {
                assert_eq!((*c).cmsg_type, libc::SCM_CREDENTIALS);
                let cred = libc::CMSG_DATA(c).cast::<libc::ucred>().read_unaligned();
                assert_eq!(cred.pid, owner.child.id() as i32);
                assert_eq!(cred.uid, libc::geteuid());
                assert_eq!(cred.gid, libc::getegid());
                credentials += 1;
            }
            c = libc::CMSG_NXTHDR(&msg, c);
        }
    }
    assert_eq!(count, 8);
    assert_eq!(&raw, b"HCLF0741");
    assert_eq!(msg.msg_flags & !libc::MSG_CMSG_CLOEXEC, 0);
    assert_eq!(credentials, 1);
    assert_eq!(rights.len(), 4);
    let mut output = rights.pop().unwrap();
    owner.targets = rights;
    let mut status = 0;
    assert_eq!(
        unsafe { libc::waitpid(owner.child.id() as _, &mut status, libc::WUNTRACED) },
        owner.child.id() as i32
    );
    assert!(libc::WIFSTOPPED(status));
    signal(&owner.pin, libc::SIGKILL);
    assert_eq!(owner.child.wait().unwrap().signal(), Some(libc::SIGKILL));
    assert_ne!(ready(owner.targets[0].as_raw_fd(), 5000) & libc::POLLIN, 0);
    let mut bytes = Vec::new();
    output.read_to_end(&mut bytes).unwrap();
    assert!(bytes.is_empty());
}

pub(super) fn matrix() {
    assert_ne!(unsafe { libc::geteuid() }, 0, "nonroot fixture only");
    if std::env::var_os("HC074_HELD_OWNER_CHILD").is_some() {
        owner_loss_child();
    }
    let cases = [
        "healthy",
        "owner-loss",
        "foreign-sender",
        "sigkill-held",
        "sigkill-after-exec",
        "lost-confirmation",
        "guardian-one-loss",
        "guardian-two-loss",
        "expiry",
        "guard-wins-over-release",
        "socket-loss",
        "short",
        "long",
        "wrong-token",
        "wrong-digest",
        "rights",
        "duplicate",
        "bad-hash",
        "bad-stream",
        "bad-pidfd",
        "unsealed-config",
        "exec-refused",
        "changed-alarm",
        "bad-config-hash",
        "unsealed-worker",
        "root-file-wrong-hash",
        "opath-stdin",
    ];
    for case in cases {
        if case == "owner-loss" {
            owner_loss();
            println!("held_launcher seed={SEED:#x} case=owner-loss original_join=false product=false all_writer_revocation=false");
            continue;
        }
        let mut a = Attempt::new(case);
        let deadline = a.deadline;
        if [
            "bad-hash",
            "bad-stream",
            "bad-pidfd",
            "unsealed-config",
            "bad-config-hash",
            "unsealed-worker",
            "root-file-wrong-hash",
            "opath-stdin",
        ]
        .contains(&case)
        {
            a.empty();
        } else {
            a.prepared();
            match case {
                "changed-alarm" => {
                    let later = deadline + NS;
                    let spec = libc::itimerspec {
                        it_interval: libc::timespec {
                            tv_sec: 0,
                            tv_nsec: 0,
                        },
                        it_value: libc::timespec {
                            tv_sec: (later / NS) as _,
                            tv_nsec: (later % NS) as _,
                        },
                    };
                    assert_eq!(
                        unsafe {
                            libc::timerfd_settime(
                                a.alarm.as_raw_fd(),
                                libc::TFD_TIMER_ABSTIME,
                                &spec,
                                std::ptr::null_mut(),
                            )
                        },
                        0
                    );
                    a.release();
                    a.empty();
                }
                "foreign-sender" => {
                    foreign_send(a.socket.as_raw_fd(), &a.packet);
                    a.empty();
                }
                "healthy" => {
                    a.release();
                    a.stopped();
                    signal(&a.pidfd, libc::SIGCONT);
                    a.input.write_all(b"x").unwrap();
                    assert!(a.reap().success());
                }
                "sigkill-held" => {
                    signal(&a.pidfd, libc::SIGKILL);
                    assert_eq!(a.reap().signal(), Some(libc::SIGKILL));
                    let mut raw = Vec::new();
                    a.output.read_to_end(&mut raw).unwrap();
                    assert!(raw.is_empty());
                }
                "sigkill-after-exec" | "lost-confirmation" => {
                    a.release();
                    a.stopped();
                    assert_ne!(ready(a.socket.as_raw_fd(), 1000) & libc::POLLHUP, 0);
                    let packet = a.packet;
                    assert!(!a.send(&packet));
                    signal(&a.pidfd, libc::SIGKILL);
                    assert_eq!(a.reap().signal(), Some(libc::SIGKILL));
                }
                "guardian-one-loss" | "guardian-two-loss" => {
                    let i = usize::from(case == "guardian-two-loss");
                    signal(&a.guards[i].1, libc::SIGKILL);
                    a.guards[i].0.wait().unwrap();
                    a.empty();
                }
                "expiry" => {
                    a.empty();
                    assert!(now() >= deadline);
                }
                "guard-wins-over-release" => {
                    signal(&a.pidfd, libc::SIGSTOP);
                    let mut s = 0;
                    assert_eq!(
                        unsafe { libc::waitpid(a.child.id() as _, &mut s, libc::WUNTRACED) },
                        a.child.id() as i32
                    );
                    assert!(libc::WIFSTOPPED(s));
                    signal(&a.guards[0].1, libc::SIGKILL);
                    a.guards[0].0.wait().unwrap();
                    a.release();
                    signal(&a.pidfd, libc::SIGCONT);
                    a.empty();
                }
                "socket-loss" => {
                    assert_eq!(
                        unsafe { libc::shutdown(a.socket.as_raw_fd(), libc::SHUT_RDWR) },
                        0
                    );
                    a.empty();
                }
                "short" | "long" | "wrong-token" | "wrong-digest" => {
                    let mut p = a.packet.to_vec();
                    match case {
                        "short" => {
                            p.pop();
                        }
                        "long" => p.push(0),
                        "wrong-token" => p[16] ^= 1,
                        _ => p[48] ^= 1,
                    };
                    assert!(a.send(&p));
                    a.empty();
                }
                "duplicate" => {
                    signal(&a.pidfd, libc::SIGSTOP);
                    let mut s = 0;
                    unsafe { libc::waitpid(a.child.id() as _, &mut s, libc::WUNTRACED) };
                    let p = a.packet;
                    a.release();
                    assert_eq!(
                        unsafe {
                            libc::send(
                                a.socket.as_raw_fd(),
                                p.as_ptr().cast(),
                                p.len(),
                                libc::MSG_NOSIGNAL,
                            )
                        },
                        80
                    );
                    signal(&a.pidfd, libc::SIGCONT);
                    a.empty();
                }
                "rights" => {
                    let mut control = [0usize; 8];
                    let mut iov = libc::iovec {
                        iov_base: a.packet.as_mut_ptr().cast(),
                        iov_len: 80,
                    };
                    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
                    msg.msg_iov = &mut iov;
                    msg.msg_iovlen = 1;
                    msg.msg_control = control.as_mut_ptr().cast();
                    msg.msg_controllen = unsafe { libc::CMSG_SPACE(4) } as _;
                    unsafe {
                        let c = libc::CMSG_FIRSTHDR(&msg);
                        (*c).cmsg_level = libc::SOL_SOCKET;
                        (*c).cmsg_type = libc::SCM_RIGHTS;
                        (*c).cmsg_len = libc::CMSG_LEN(4) as _;
                        libc::CMSG_DATA(c)
                            .cast::<RawFd>()
                            .write(a.input.as_raw_fd());
                        assert_eq!(
                            libc::sendmsg(a.socket.as_raw_fd(), &msg, libc::MSG_NOSIGNAL),
                            80
                        );
                    }
                    a.empty();
                }
                "exec-refused" => {
                    a.release();
                    a.empty();
                }
                _ => unreachable!(),
            }
        }
        assert_eq!(a.deadline, deadline);
        println!("held_launcher seed={SEED:#x} case={case} deadline_unchanged=true original_join=false product=false all_writer_revocation=false");
    }
}
