//! Own a registered command's Windows process tree before any of its code runs.

use std::io;
use std::mem::{size_of, zeroed};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::process::Child;
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectBasicAccountingInformation,
    JobObjectExtendedLimitInformation, QueryInformationJobObject, SetInformationJobObject,
    TerminateJobObject, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};

pub(super) struct ProcessTree(OwnedHandle);

impl ProcessTree {
    pub(super) fn attach_and_resume(child: &Child) -> io::Result<Self> {
        // SAFETY: null security/name pointers create a private, non-inheritable job.
        let raw_job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if raw_job.is_null() {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: CreateJobObjectW returned a new valid handle; this is its sole owner.
        let tree = Self(unsafe { OwnedHandle::from_raw_handle(raw_job) });
        // SAFETY: the C POD structure is valid when zero-initialized.
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: the job handle and structure are valid for the call and its exact size.
        if unsafe {
            SetInformationJobObject(
                tree.0.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: both owned handles are live. The command was created suspended, so
        // assignment precedes execution and no descendant can escape the initial job.
        if unsafe { AssignProcessToJobObject(tree.0.as_raw_handle(), child.as_raw_handle()) } == 0 {
            return Err(io::Error::last_os_error());
        }
        resume_initial_thread(child.id())?;
        Ok(tree)
    }

    pub(super) fn is_empty(&self) -> io::Result<bool> {
        // SAFETY: the C POD structure is valid when zero-initialized.
        let mut accounting: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = unsafe { zeroed() };
        // SAFETY: the job and writable accounting buffer are valid for the call.
        if unsafe {
            QueryInformationJobObject(
                self.0.as_raw_handle(),
                JobObjectBasicAccountingInformation,
                (&mut accounting as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
                size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(accounting.ActiveProcesses == 0)
    }

    pub(super) fn terminate(&self) -> io::Result<()> {
        // SAFETY: this private owned job contains only the registered command's tree.
        if unsafe { TerminateJobObject(self.0.as_raw_handle(), 124) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

fn resume_initial_thread(process_id: u32) -> io::Result<()> {
    // Rust's Child retains the process handle but not CreateProcess's initial thread handle.
    // The suspended process has one initial thread; enumerate it without running its code.
    // SAFETY: the flags request a read-only thread snapshot with no inherited handle.
    let raw_snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if raw_snapshot == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the snapshot is a new valid handle and this is its sole owner.
    let snapshot = unsafe { OwnedHandle::from_raw_handle(raw_snapshot) };
    // SAFETY: the C POD structure is valid when zero-initialized.
    let mut entry: THREADENTRY32 = unsafe { zeroed() };
    entry.dwSize = size_of::<THREADENTRY32>() as u32;
    // SAFETY: the owned snapshot and initialized writable entry remain valid.
    let mut found = unsafe { Thread32First(snapshot.as_raw_handle(), &mut entry) } != 0;
    while found {
        if entry.th32OwnerProcessID == process_id {
            // SAFETY: only the initial thread of this suspended child is requested.
            let raw_thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) };
            if raw_thread.is_null() {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: OpenThread returned a new valid handle and this is its sole owner.
            let thread = unsafe { OwnedHandle::from_raw_handle(raw_thread) };
            // SAFETY: the thread handle is live and has THREAD_SUSPEND_RESUME access.
            let previous_count = unsafe { ResumeThread(thread.as_raw_handle()) };
            return match previous_count {
                1 => Ok(()),
                u32::MAX => Err(io::Error::last_os_error()),
                other => Err(io::Error::other(format!(
                    "unexpected initial thread suspend count: {other}"
                ))),
            };
        }
        entry.dwSize = size_of::<THREADENTRY32>() as u32;
        // SAFETY: the owned snapshot and initialized writable entry remain valid.
        found = unsafe { Thread32Next(snapshot.as_raw_handle(), &mut entry) } != 0;
    }
    Err(io::Error::other(
        "suspended command's initial thread was not found",
    ))
}
