use std::io::{self, Read};
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Output};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const PREFIX_BYTES: usize = 4096;
pub const READ_ATTEMPTS: usize = 8;

#[derive(Debug, PartialEq, Eq)]
pub enum End {
    Eof,
    WouldBlock,
    ByteLimit,
    ReadBudget,
    ReadError(Option<i32>),
    SetupError(Option<i32>),
    Missing,
}

pub struct Capture {
    pub bytes: Vec<u8>,
    pub end: End,
}

impl std::fmt::Debug for Capture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Capture")
            .field("prefix", &String::from_utf8_lossy(&self.bytes))
            .field("end", &self.end)
            .finish()
    }
}

pub fn read_bounded(reader: &mut impl Read) -> Capture {
    let mut bytes = Vec::new();
    for _ in 0..READ_ATTEMPTS {
        let mut buffer = [0; PREFIX_BYTES / READ_ATTEMPTS];
        let end = match reader.read(&mut buffer) {
            Ok(0) => Some(End::Eof),
            Ok(n) => {
                bytes.extend_from_slice(&buffer[..n]);
                None
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => None,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Some(End::WouldBlock),
            Err(error) => Some(End::ReadError(error.raw_os_error())),
        };
        if let Some(end) = end {
            return Capture { bytes, end };
        }
    }
    let end = if bytes.len() == PREFIX_BYTES {
        End::ByteLimit
    } else {
        End::ReadBudget
    };
    Capture { bytes, end }
}

pub fn capture<T: Read + AsRawFd>(pipe: Option<T>) -> Capture {
    let Some(mut pipe) = pipe else {
        return Capture {
            bytes: vec![],
            end: End::Missing,
        };
    };
    // Only the guard's owned read end, after signalling/reaping the original
    // probe. Never read to EOF while an unobserved writer might still exist.
    let flags = unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_GETFL) };
    if flags < 0
        || unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        return Capture {
            bytes: vec![],
            end: End::SetupError(io::Error::last_os_error().raw_os_error()),
        };
    }
    read_bounded(&mut pipe)
}

pub fn output(command: &mut Command, gate: &Mutex<()>) -> Output {
    let guard = gate.lock().unwrap();
    let mut child = command.spawn().unwrap();
    drop(guard);
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if Instant::now() >= until {
            let kill = child.kill();
            let wait = child.wait();
            let pid = child.id();
            let signal = wait.as_ref().ok().and_then(ExitStatusExt::signal);
            // Child still owns its unreaped direct process at kill/wait. Do not
            // query/reap the numeric PID again after release in this parallel
            // parent: that could address a subsequently reused PID.
            let original_wait_ok = wait.is_ok();
            let argument = command.get_args().next().map(|arg| {
                let bytes = arg.as_bytes();
                String::from_utf8_lossy(&bytes[..bytes.len().min(64)]).into_owned()
            });
            let stdout = capture(child.stdout.take());
            let stderr = capture(child.stderr.take());
            panic!(
                "normal root preparation stalled; pid={pid} arg0_prefix={argument:?} kill={kill:?} signal={signal:?} wait={wait:?} original_wait_ok={original_wait_ok} stdout={stdout:?} stderr={stderr:?}"
            );
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
