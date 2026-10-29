//! Private fixed process restrictions, shared by terminating diagnostic entries.
use super::FixedProbe;
use std::io;

pub(super) fn install(filters: &mut [libc::sock_filter; 64]) -> io::Result<()> {
    let limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: current diagnostic child only, fixed syscall arguments.
    if unsafe { libc::setrlimit(libc::RLIMIT_CORE, &limit) } != 0
        || unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1usize, 0usize, 0usize, 0usize) } != 0
        || unsafe { libc::prctl(libc::PR_GET_NO_NEW_PRIVS, 0usize, 0usize, 0usize, 0usize) } != 1
    {
        return Err(io::Error::other("fixed restriction install refused"));
    }
    let header = libc::sock_fprog {
        len: 64,
        filter: filters.as_mut_ptr(),
    };
    // SAFETY: fixed complete BPF array/header remain live across synchronous TSYNC.
    if unsafe {
        libc::syscall(
            libc::SYS_seccomp,
            libc::SECCOMP_SET_MODE_FILTER,
            libc::SECCOMP_FILTER_FLAG_TSYNC,
            &header,
        )
    } != 0
        || unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0usize, 0usize, 0usize, 0usize) } != 0
    {
        return Err(io::Error::other("fixed restriction install refused"));
    }
    Ok(())
}

fn native() -> bool {
    let path = c"/".as_ptr() as u64;
    for (nr, args) in [
        (272, [0, 0, 0, 0, 0, 0]),
        (308, [u64::MAX, 0, 0, 0, 0, 0]),
        (161, [0, 0, 0, 0, 0, 0]),
        (80, [path, 0, 0, 0, 0, 0]),
        (81, [u64::MAX, 0, 0, 0, 0, 0]),
        (165, [0, 0, 0, 0, 0, 0]),
        (166, [0, 0, 0, 0, 0, 0]),
        (155, [0, 0, 0, 0, 0, 0]),
        (428, [u64::MAX, 0, 0, 0, 0, 0]),
        (429, [u64::MAX, 0, u64::MAX, 0, 0, 0]),
        (430, [0, 0, 0, 0, 0, 0]),
        (431, [u64::MAX, 0, 0, 0, 0, 0]),
        (432, [u64::MAX, 0, 0, 0, 0, 0]),
        (433, [u64::MAX, 0, 0, 0, 0, 0]),
        (442, [u64::MAX, 0, 0, 0, 0, 0]),
        (59, [0, 0, 0, 0, 0, 0]),
        (322, [u64::MAX, 0, 0, 0, 0, 0]),
        (101, [u64::MAX, 0, 0, 0, 0, 0]),
        (317, [u64::MAX, 0, 0, 0, 0, 0]),
        (157, [4, 1, 0, 0, 0, 0]),
    ] {
        // SAFETY: fixed nonproduct denial probes. Bad pointers/FDs cannot exec/mount;
        // chdir('/') affects only this child if a filter defect is found.
        if unsafe {
            libc::syscall(
                nr as libc::c_long,
                args[0],
                args[1],
                args[2],
                args[3],
                args[4],
                args[5],
            )
        } != -1
            || io::Error::last_os_error().raw_os_error() != Some(libc::EPERM)
        {
            return false;
        }
    }
    true
}
pub(super) fn exercise(probe: FixedProbe) -> bool {
    match probe {
        FixedProbe::Native => native(),
        FixedProbe::X32 => {
            // SAFETY: owned core-disabled child must die at the fixed ABI gate.
            unsafe { libc::syscall(0x4000_0027 as libc::c_long) };
            false
        }
        FixedProbe::I386 => {
            // SAFETY: fixed compat getpid, no pointer arguments or authority.
            unsafe {
                core::arch::asm!("push rbx","xor ebx, ebx","xor ecx, ecx","xor edx, edx","mov eax, 20","int 0x80","pop rbx",lateout("eax") _,lateout("ecx") _,lateout("edx") _)
            };
            false
        }
    }
}
