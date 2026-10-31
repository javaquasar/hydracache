use super::*;
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_BROKEN_PIPE, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::*;
use windows_sys::Win32::System::JobObjects::*;
use windows_sys::Win32::System::Pipes::PeekNamedPipe;
use windows_sys::Win32::System::Threading::{
    OpenThread, ResumeThread, CREATE_NO_WINDOW, CREATE_SUSPENDED, THREAD_SUSPEND_RESUME,
};
pub const SCOPE: &str = "windows_job_kill_on_close_process_commit_limit";
pub trait Pipe: AsRawHandle {}
impl<T: AsRawHandle> Pipe for T {}
struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
pub struct Tree {
    job: Handle,
    terminated: bool,
}
impl Tree {
    pub fn spawn(command: &mut Command, bounds: ProcessBounds) -> io::Result<(Self, Child)> {
        // Job membership is established before the suspended worker can fork.
        // https://learn.microsoft.com/windows/win32/api/jobapi2/nf-jobapi2-assignprocesstojobobject
        let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if job.is_null() {
            return Err(io::Error::last_os_error());
        }
        let mut tree = Self {
            job: Handle(job),
            terminated: false,
        };
        let mut limit: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        limit.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            | JOB_OBJECT_LIMIT_PROCESS_MEMORY
            | JOB_OBJECT_LIMIT_JOB_MEMORY
            | JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
        limit.BasicLimitInformation.ActiveProcessLimit = bounds.maximum_processes;
        limit.ProcessMemoryLimit = bounds.memory_bytes;
        limit.JobMemoryLimit = bounds
            .memory_bytes
            .checked_mul(2)
            .ok_or_else(|| io::Error::other("job memory overflow"))?;
        if unsafe {
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &limit as *const _ as *const _,
                std::mem::size_of_val(&limit) as u32,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        command.creation_flags(CREATE_SUSPENDED | CREATE_NO_WINDOW);
        let mut child = command.spawn()?;
        if unsafe { AssignProcessToJobObject(job, child.as_raw_handle()) } == 0 {
            let error = io::Error::last_os_error();
            let _ = child.kill();
            let _ = stop_child(&mut child, bounds.cleanup_timeout);
            return Err(error);
        }
        if let Err(error) = resume(child.id()) {
            let _ = tree.terminate();
            let _ = stop_child(&mut child, bounds.cleanup_timeout);
            return Err(error);
        }
        Ok((tree, child))
    }
    pub fn terminate(&mut self) -> io::Result<()> {
        if self.terminated {
            return Ok(());
        }
        if unsafe { TerminateJobObject(self.job.0, 0xc075) } == 0 {
            Err(io::Error::last_os_error())
        } else {
            self.terminated = true;
            Ok(())
        }
    }
    pub fn poll_exit(&mut self, child: &mut Child) -> io::Result<Option<ExitStatus>> {
        child.try_wait()
    }
    pub fn live_members(&self) -> io::Result<usize> {
        let mut info: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = unsafe { std::mem::zeroed() };
        if unsafe {
            QueryInformationJobObject(
                self.job.0,
                JobObjectBasicAccountingInformation,
                &mut info as *mut _ as *mut _,
                std::mem::size_of_val(&info) as u32,
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(info.ActiveProcesses as usize)
    }
}
fn stop_child(child: &mut Child, timeout: Duration) -> io::Result<()> {
    let started = Instant::now();
    while child.try_wait()?.is_none() {
        if started.elapsed() >= timeout {
            return Err(io::Error::other("suspended child cleanup deadline"));
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Ok(())
}
fn resume(pid: u32) -> io::Result<()> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let snapshot = Handle(snapshot);
    let mut thread: THREADENTRY32 = unsafe { std::mem::zeroed() };
    thread.dwSize = std::mem::size_of_val(&thread) as u32;
    let mut found = unsafe { Thread32First(snapshot.0, &mut thread) };
    // A freshly suspended process has one primary thread. Bound enumeration too.
    for _ in 0..65_536 {
        if found == 0 {
            break;
        }
        if thread.th32OwnerProcessID == pid {
            let handle = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, thread.th32ThreadID) };
            if handle.is_null() {
                return Err(io::Error::last_os_error());
            }
            let handle = Handle(handle);
            return if unsafe { ResumeThread(handle.0) } == u32::MAX {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            };
        }
        found = unsafe { Thread32Next(snapshot.0, &mut thread) };
    }
    Err(io::Error::other(
        "suspended worker primary thread not found within bound",
    ))
}
pub fn prepare_pipe(_: &impl Pipe) -> io::Result<()> {
    Ok(())
}
pub fn available(pipe: &impl Pipe) -> io::Result<Option<usize>> {
    let mut available = 0;
    if unsafe {
        PeekNamedPipe(
            pipe.as_raw_handle(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            &mut available,
            std::ptr::null_mut(),
        )
    } == 0
    {
        let error = io::Error::last_os_error();
        return if error.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) {
            Ok(None)
        } else {
            Err(error)
        };
    }
    Ok(Some(available as usize))
}
