//! Isolated fresh-command diagnostic, never the installed artifact helper.
//! Pipes remain pipes. The control uses std's original output() unchanged.
use std::io;
use std::process::{Command, Output, Stdio};
use std::time::Instant;

pub fn control(command: &mut Command) -> io::Result<Output> {
    command.output()
}

#[derive(Debug, serde::Serialize)]
pub struct Phases {
    pub spawn_ns: u128,
    pub handle_observation_ns: u128,
    pub wait_and_collect_ns: u128,
    pub terminal_observation_ns: u128,
    pub child_start: ProcessTimes,
    pub child_end: ProcessTimes,
}

#[derive(Debug, serde::Serialize)]
pub struct ProcessTimes {
    pub pid: u32,
    pub created_100ns: u64,
    pub exited_100ns: u64,
    pub kernel_ns: u64,
    pub user_ns: u64,
}

#[cfg(windows)]
pub fn times(
    handle: &impl std::os::windows::io::AsRawHandle,
    pid: u32,
) -> io::Result<ProcessTimes> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::GetProcessTimes;
    let [mut created, mut exited, mut kernel, mut user] = [FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    }; 4];
    // SAFETY: caller retains the original or its duplicated process handle.
    // Distinct initialized output values live throughout the call; no PID reopen.
    let ok = unsafe {
        GetProcessTimes(
            handle.as_raw_handle(),
            &mut created,
            &mut exited,
            &mut kernel,
            &mut user,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let ticks = |v: FILETIME| (u64::from(v.dwHighDateTime) << 32) | u64::from(v.dwLowDateTime);
    Ok(ProcessTimes {
        pid,
        created_100ns: ticks(created),
        exited_100ns: ticks(exited),
        kernel_ns: ticks(kernel) * 100,
        user_ns: ticks(user) * 100,
    })
}

/// Only fresh commands with unspecified stdio are in the comparison contract.
/// Explicit null/piped settings are not a general replacement for output().
#[cfg(windows)]
pub fn profiled(command: &mut Command) -> io::Result<(Output, Phases)> {
    use std::os::windows::io::{AsRawHandle, BorrowedHandle};
    let start = Instant::now();
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let spawn_ns = start.elapsed().as_nanos();
    let start = Instant::now();
    // SAFETY: Child owns this handle during borrowing and duplication. The
    // resulting OwnedHandle survives wait_with_output consuming the Child.
    let retained =
        unsafe { BorrowedHandle::borrow_raw(child.as_raw_handle()) }.try_clone_to_owned();
    let observed =
        retained.and_then(|handle| times(&handle, child.id()).map(|value| (handle, value)));
    let (handle, child_start) = match observed {
        Ok(value) => value,
        Err(error) => {
            // Diagnostic failure is not a fallback to the control. Stop/reap
            // only this directly owned child; outer runner bounds the fixture.
            let _ = child.kill();
            child.wait()?;
            return Err(error);
        }
    };
    let handle_observation_ns = start.elapsed().as_nanos();
    let start = Instant::now();
    let output = child.wait_with_output()?;
    let wait_and_collect_ns = start.elapsed().as_nanos();
    let start = Instant::now();
    let child_end = times(&handle, child_start.pid)?;
    let terminal_observation_ns = start.elapsed().as_nanos();
    if child_start.created_100ns != child_end.created_100ns
        || child_end.exited_100ns == 0
        || child_start.kernel_ns > child_end.kernel_ns
        || child_start.user_ns > child_end.user_ns
    {
        return Err(io::Error::other("retained process identity/times refused"));
    }
    Ok((
        output,
        Phases {
            spawn_ns,
            handle_observation_ns,
            wait_and_collect_ns,
            terminal_observation_ns,
            child_start,
            child_end,
        },
    ))
}
