//! Local post-install mutation policy; no handoff, custody or worker authority.
use super::*;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

const DENIED: [u32; 15] = [
    272, 308, 161, 80, 81, 165, 166, 155, 428, 429, 430, 431, 432, 433, 442,
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Probe {
    Unshare,
    Setns,
    Chroot,
    Chdir,
    Fchdir,
    Mount,
    Umount,
    Pivot,
    OpenTree,
    MoveMount,
    Fsopen,
    Fsconfig,
    Fsmount,
    Fspick,
    MountSetattr,
}
const PROBES: [Probe; 15] = [
    Probe::Unshare,
    Probe::Setns,
    Probe::Chroot,
    Probe::Chdir,
    Probe::Fchdir,
    Probe::Mount,
    Probe::Umount,
    Probe::Pivot,
    Probe::OpenTree,
    Probe::MoveMount,
    Probe::Fsopen,
    Probe::Fsconfig,
    Probe::Fsmount,
    Probe::Fspick,
    Probe::MountSetattr,
];
impl Probe {
    fn index(self) -> usize {
        PROBES.iter().position(|p| *p == self).unwrap()
    }
    fn nr(self) -> u64 {
        DENIED[self.index()] as u64
    }
    fn arg(self) -> &'static std::ffi::CStr {
        match self {
            Self::Unshare => c"unshare",
            Self::Setns => c"setns",
            Self::Chroot => c"chroot",
            Self::Chdir => c"chdir",
            Self::Fchdir => c"fchdir",
            Self::Mount => c"mount",
            Self::Umount => c"umount",
            Self::Pivot => c"pivot",
            Self::OpenTree => c"open-tree",
            Self::MoveMount => c"move-mount",
            Self::Fsopen => c"fsopen",
            Self::Fsconfig => c"fsconfig",
            Self::Fsmount => c"fsmount",
            Self::Fspick => c"fspick",
            Self::MountSetattr => c"mount-setattr",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Case {
    Probe(Probe),
    ControlUnshare,
    ControlChdir,
    ControlFchdir,
    ControlSetns,
    SelfExec,
    OtherExec,
    X32,
    I386,
    Flags,
    Policy,
    Length,
    Header,
    Pointer,
    EarlyExec,
    EarlyMount,
    Env,
}
impl Case {
    pub(super) fn control(self) -> bool {
        matches!(
            self,
            Self::ControlUnshare | Self::ControlChdir | Self::ControlFchdir | Self::ControlSetns
        )
    }
    pub(super) fn arg(self) -> &'static std::ffi::CStr {
        match self {
            Self::Probe(p) => p.arg(),
            Self::ControlUnshare => c"control-unshare",
            Self::ControlChdir => c"control-chdir",
            Self::ControlFchdir => c"control-fchdir",
            Self::ControlSetns => c"control-setns",
            Self::SelfExec | Self::Env => c"self",
            Self::OtherExec => c"other",
            Self::X32 => c"x32",
            Self::I386 => c"i386",
            Self::Flags => c"flags",
            Self::Policy => c"policy",
            Self::Length => c"length",
            Self::Header => c"header",
            Self::Pointer => c"pointer",
            Self::EarlyExec => c"early-exec",
            Self::EarlyMount => c"early-mount",
        }
    }
}
fn program() -> [libc::sock_filter; 64] {
    let old = policy();
    let mut out = [libc::sock_filter {
        code: 0,
        jt: 0,
        jf: 0,
        k: 0,
    }; 64];
    out[..22].copy_from_slice(&old[..22]);
    for (index, nr) in DENIED.iter().enumerate() {
        out[22 + index * 2] = libc::sock_filter {
            code: 0x15,
            jt: 0,
            jf: 1,
            k: *nr,
        };
        out[23 + index * 2] = libc::sock_filter {
            code: 0x06,
            jt: 0,
            jf: 0,
            k: libc::SECCOMP_RET_ERRNO | libc::EPERM as u32,
        };
    }
    out[52..].copy_from_slice(&old[22..]);
    out
}
pub(super) fn expected(case: Case) -> Vec<u8> {
    if case.control() {
        encoded(&policy())
    } else {
        encoded(&program())
    }
}
pub(super) fn root() -> io::Result<Root> {
    // SAFETY: only the current nonroot fixture identity, no account changes.
    let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
    if uid == 0 || gid == 0 {
        return Err(invalid());
    }
    static IMAGES: OnceLock<Result<[Vec<u8>; 4], String>> = OnceLock::new();
    let images = IMAGES.get_or_init(|| {
        let prepare = || -> io::Result<[Vec<u8>; 4]> {
            Ok([
                compile()?,
                super::super::read_bounded(&File::open("/lib64/ld-linux-x86-64.so.2")?)?,
                super::super::read_bounded(&File::open("/lib/x86_64-linux-gnu/libc.so.6")?)?,
                super::super::read_bounded(&File::open("/lib/x86_64-linux-gnu/libgcc_s.so.1")?)?,
            ])
        };
        prepare().map_err(|e| e.to_string())
    });
    Root::materialize(
        images.as_ref().map_err(|e| io::Error::other(e.clone()))?,
        uid,
        gid,
    )
}
fn compile() -> io::Result<Vec<u8>> {
    compile_helper(
        include_str!("diagnostic_signer_namespace_helper.rs"),
        "namespace",
    )
}
pub(super) fn compile_helper(source_text: &str, label: &str) -> io::Result<Vec<u8>> {
    let home = std::env::var_os("HOME").ok_or_else(invalid)?;
    let rustup = PathBuf::from(&home).join(".cargo/bin/rustup");
    let tool = Command::new(rustup)
        .args(["which", "--toolchain", "1.94.0", "rustc"])
        .env_clear()
        .env("HOME", home)
        .output()?;
    if !tool.status.success() {
        return Err(invalid());
    }
    let compiler = PathBuf::from(
        String::from_utf8(tool.stdout)
            .map_err(|_| invalid())?
            .trim(),
    );
    if !compiler.is_absolute() || !compiler.is_file() {
        return Err(invalid());
    }
    let version = Command::new(&compiler)
        .arg("--version")
        .env_clear()
        .output()?;
    if !version.status.success() || !version.stdout.starts_with(b"rustc 1.94.0 ") {
        return Err(invalid());
    }
    let temp = tempfile::tempdir()?;
    let source = temp.path().join("helper.rs");
    let binary = temp.path().join("helper");
    fs::write(&source, source_text)?;
    let result = Command::new(compiler)
        .arg(&source)
        .args([
            "--edition=2021",
            "--target=x86_64-unknown-linux-gnu",
            "-Dwarnings",
            "-Copt-level=3",
            "-Cpanic=abort",
            "-Cdebuginfo=0",
        ])
        .arg(format!(
            "--remap-path-prefix={}=/owned-namespace-fixture",
            temp.path().display()
        ))
        .arg("-o")
        .arg(&binary)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("TMPDIR", temp.path())
        .output()?;
    if !result.status.success() {
        return Err(io::Error::other(format!(
            "fixed helper compile failed: {}",
            String::from_utf8_lossy(&result.stderr[..result.stderr.len().min(8192)])
        )));
    }
    let bytes = super::super::read_bounded(&File::open(&binary)?)?;
    let digest: String = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    println!("fixture_{label}_helper_sha256={digest} compiler=Rust1.94 profile=O3_abort runtime_admission=false");
    Ok(bytes)
}
fn owner(case: Case) -> io::Result<RootedExec> {
    let mut owner = RootedExec::namespace(case)?;
    owner.enter()?;
    owner.first_exec()?;
    owner.install()?;
    Ok(owner)
}
fn directory_records(raw: &[u8], names: &mut BTreeSet<std::ffi::OsString>) -> io::Result<()> {
    if raw.len() > 512 {
        return Err(invalid());
    }
    let mut offset = 0usize;
    while offset < raw.len() {
        let head = raw
            .get(offset..offset.checked_add(19).ok_or_else(invalid)?)
            .ok_or_else(invalid)?;
        let count = u16::from_le_bytes(head[16..18].try_into().unwrap()) as usize;
        if count < 24 || !count.is_multiple_of(8) || names.len() >= 8 {
            return Err(invalid());
        }
        let end = offset.checked_add(count).ok_or_else(invalid)?;
        let bytes = raw.get(offset + 19..end).ok_or_else(invalid)?;
        let zero = bytes.iter().position(|b| *b == 0).ok_or_else(invalid)?;
        let name = std::str::from_utf8(&bytes[..zero]).map_err(|_| invalid())?;
        if !((name == "." || name == "..") && head[18] == libc::DT_DIR
            || !name.is_empty()
                && name.len() <= 10
                && name.bytes().all(|b| b.is_ascii_digit())
                && head[18] == libc::DT_LNK)
            || !names.insert(name.into())
        {
            return Err(invalid());
        }
        offset = end;
    }
    Ok(())
}
pub(super) fn descriptors(bound: &(File, [u64; 2])) -> io::Result<BTreeSet<std::ffi::OsString>> {
    let (file, original) = bound;
    // SAFETY: initialized output and fixed queries on this retained original directory.
    let mut fs: libc::statfs = unsafe { std::mem::zeroed() };
    // SAFETY: flags/statfs/lseek only on an owned private readonly directory FD.
    let (result, flags, fd_flags, position) = unsafe {
        (
            libc::fstatfs(file.as_raw_fd(), &mut fs),
            libc::fcntl(file.as_raw_fd(), libc::F_GETFL),
            libc::fcntl(file.as_raw_fd(), libc::F_GETFD),
            libc::lseek(file.as_raw_fd(), 0, libc::SEEK_SET),
        )
    };
    if result != 0
        || fs.f_type != 0x9fa0
        || flags < 0
        || fd_flags < 0
        || position != 0
        || flags & (libc::O_ACCMODE | libc::O_PATH) != libc::O_RDONLY
        || fd_flags & libc::FD_CLOEXEC == 0
        || !file.metadata()?.is_dir()
        || super::super::Stamp::of(file.metadata()?).identity() != *original
    {
        return Err(invalid());
    }
    let mut names = BTreeSet::new();
    for _ in 0..3 {
        let mut raw = [0u8; 512];
        // SAFETY: exact retained original directory and bounded initialized writable buffer.
        let count = unsafe {
            libc::syscall(
                libc::SYS_getdents64,
                file.as_raw_fd(),
                raw.as_mut_ptr(),
                raw.len(),
            )
        };
        if count < 0 {
            return Err(io::Error::last_os_error());
        }
        if count == 0 {
            if !names.remove(std::ffi::OsStr::new(".")) || !names.remove(std::ffi::OsStr::new(".."))
            {
                return Err(invalid());
            }
            return Ok(names);
        }
        let count = usize::try_from(count).map_err(|_| invalid())?;
        directory_records(raw.get(..count).ok_or_else(invalid)?, &mut names)?;
    }
    Err(invalid())
}
fn finish(owner: &mut RootedExec) -> io::Result<()> {
    let result = finish_inner(owner);
    if result.is_err() {
        owner.refuse()?;
    }
    result
}
fn finish_inner(owner: &mut RootedExec) -> io::Result<()> {
    if owner.phase != Phase::Installed {
        return Err(invalid());
    }
    let case = owner.namespace_case.ok_or_else(invalid)?;
    owner.context(false)?;
    maps::validate(
        &text(format!("/proc/{}/maps", owner.pid))?,
        &owner.root,
        true,
    )?;
    let (nr, value, error) = match case {
        Case::Probe(p) => (p.nr(), -(libc::EPERM as i64), true),
        Case::ControlUnshare => (libc::SYS_unshare as u64, -(libc::EPERM as i64), true),
        Case::ControlChdir => (libc::SYS_chdir as u64, 0, false),
        Case::ControlFchdir => (libc::SYS_fchdir as u64, -(libc::ENOTDIR as i64), true),
        Case::ControlSetns => (libc::SYS_setns as u64, -(libc::EBADF as i64), true),
        _ => return Err(invalid()),
    };
    match owner.event()? {
        Event::Entry {
            arch: ARCH,
            nr: actual,
            ..
        } if actual == nr => {}
        _ => return Err(invalid()),
    }
    let observed = owner.event()?;
    if observed
        != (Event::Exit {
            arch: ARCH,
            value,
            error,
        })
    {
        return Err(io::Error::other(format!(
            "namespace fixed probe {case:?} exit refused: {observed:?}"
        )));
    }
    if case == Case::ControlUnshare {
        owner.context(false)?;
        maps::validate(
            &text(format!("/proc/{}/maps", owner.pid))?,
            &owner.root,
            true,
        )?;
        owner.refuse()?;
        println!("old_policy_nested_userns=kernel_EPERM regain_not_reproduced=true control_refused=true host_namespace_changed=false");
        return Ok(());
    }
    owner.context(false)?;
    maps::validate(
        &text(format!("/proc/{}/maps", owner.pid))?,
        &owner.root,
        true,
    )?;
    match owner.event()? {
        Event::Entry {
            arch: ARCH,
            nr,
            args,
        } if nr == libc::SYS_exit_group as u64 && args[0] == 0 => {}
        _ => return Err(invalid()),
    }
    owner.trace(libc::PTRACE_SYSCALL, 0)?;
    if owner.wait()? != 0 {
        return Err(invalid());
    }
    owner.root.check()?;
    owner.phase = Phase::Terminal;
    Ok(())
}
fn fatal(owner: &mut RootedExec) -> io::Result<()> {
    let result = fatal_inner(owner);
    if result.is_err() {
        owner.refuse()?;
    }
    result
}
fn fatal_inner(owner: &mut RootedExec) -> io::Result<()> {
    if owner.phase != Phase::Installed {
        return Err(invalid());
    }
    owner.context(false)?;
    let (arch, nr) = match owner.namespace_case {
        Some(Case::X32) => (ARCH, libc::SYS_getpid as u64 | 0x4000_0000),
        Some(Case::I386) => (0x4000_0003, 20),
        _ => return Err(invalid()),
    };
    match owner.event()? {
        Event::Entry {
            arch: actual,
            nr: actual_nr,
            ..
        } if actual == arch && actual_nr == nr => {}
        _ => return Err(invalid()),
    }
    owner.trace(libc::PTRACE_SYSCALL, 0)?;
    let mut status = owner.wait()?;
    if status == SYSCALL_STOP {
        // Some kernels report one exact ABI exit before the terminal SIGSYS.
        // SAFETY: initialized sized syscall-info output on the original stopped child.
        let mut info: libc::ptrace_syscall_info = unsafe { std::mem::zeroed() };
        // SAFETY: original tracee and initialized correctly sized buffer.
        let size = unsafe {
            libc::ptrace(
                libc::PTRACE_GET_SYSCALL_INFO,
                owner.pid,
                std::mem::size_of_val(&info),
                &mut info,
            )
        };
        if size < 0
            || decode(&info, size as usize)?
                != (Event::Exit {
                    arch,
                    value: nr as i64,
                    error: false,
                })
        {
            return Err(invalid());
        }
        owner.trace(libc::PTRACE_SYSCALL, 0)?;
        status = owner.wait()?;
    }
    if !libc::WIFSIGNALED(status) || libc::WTERMSIG(status) != libc::SIGSYS {
        return Err(invalid());
    }
    owner.root.check()?;
    owner.phase = Phase::Terminal;
    Ok(())
}
#[path = "diagnostic_signer_namespace_fixture_tests.rs"]
mod tests;
