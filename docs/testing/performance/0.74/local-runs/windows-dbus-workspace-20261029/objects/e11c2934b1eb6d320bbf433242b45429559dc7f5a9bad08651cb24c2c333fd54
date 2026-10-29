use super::*;

#[test]
fn signer_seccomp_original_install_and_kernel_execve_denial() {
    probe(Case::ExecSelf, Fault::None).unwrap();
    println!("kernel_install=exact_entry_memory_exit_zero original_fork_helper=true handoff_or_runtime_admission=false");
}

#[test]
fn signer_seccomp_execve_other_and_execveat_are_kernel_denied() {
    for case in [Case::ExecOther, Case::ExecFd] {
        probe(case, Fault::None).unwrap();
    }
}
#[test]
fn signer_seccomp_task_creation_and_policy_changes_are_kernel_denied() {
    for nr in DENIED.into_iter().skip(2) {
        probe(Case::Denied(nr), Fault::None).unwrap();
    }
    probe(Case::RestoreDumpable, Fault::None).unwrap();
    assert!(probe(Case::Denied(libc::SYS_getpid as u32), Fault::None).is_err());
}
#[test]
fn signer_seccomp_x32_and_i386_abi_are_killed_before_dispatch() {
    for case in [Case::X32, Case::I386] {
        probe(case, Fault::None).unwrap();
    }
    println!("actual_x32_number_bit_and_i386_int80=SIGSYS no_exec_or_handoff=true");
}
#[test]
fn signer_seccomp_wrong_flags_bytes_length_or_equal_header_pointer_refuse() {
    for fault in [
        Fault::Flags,
        Fault::Header,
        Fault::Policy,
        Fault::Length,
        Fault::Pointer,
    ] {
        assert!(probe(Case::ExecSelf, fault).is_err());
    }
}
fn evaluate_args(program: &[libc::sock_filter], arch: u32, nr: u32, args: [u64; 2]) -> Option<u32> {
    let mut a = 0;
    let mut pc = 0usize;
    for _ in 0..COUNT + 1 {
        let i = program.get(pc)?;
        match i.code {
            0x20 => {
                a = match i.k {
                    0 => nr,
                    4 => arch,
                    16 => args[0] as u32,
                    20 => (args[0] >> 32) as u32,
                    24 => args[1] as u32,
                    28 => (args[1] >> 32) as u32,
                    _ => return None,
                }
            }
            0x15 => pc += if a == i.k { i.jt } else { i.jf } as usize,
            0x45 => pc += if a & i.k != 0 { i.jt } else { i.jf } as usize,
            0x06 => return Some(i.k),
            _ => return None,
        }
        pc += 1;
    }
    None
}
fn evaluate(program: &[libc::sock_filter], arch: u32, nr: u32) -> Option<u32> {
    evaluate_args(program, arch, nr, [0; 2])
}
#[test]
fn signer_seccomp_policy_exact_bytes_bounded_syscalls_and_bad_arch() {
    let p = policy();
    assert_eq!(encoded(&p).len(), 272);
    assert_eq!(std::mem::size_of::<libc::sock_filter>(), 8);
    assert_eq!(std::mem::size_of::<libc::sock_fprog>(), 16);
    for nr in 0..1024 {
        let want = if DENIED.contains(&nr) {
            libc::SECCOMP_RET_ERRNO | libc::EPERM as u32
        } else {
            libc::SECCOMP_RET_ALLOW
        };
        assert_eq!(evaluate(&p, ARCH, nr), Some(want));
        for arch in [I386, 0, ARCH ^ 1] {
            assert_eq!(evaluate(&p, arch, nr), Some(libc::SECCOMP_RET_KILL_PROCESS));
        }
        assert_eq!(
            evaluate(&p, ARCH, nr | X32),
            Some(libc::SECCOMP_RET_KILL_PROCESS)
        );
    }
    for index in 0..COUNT {
        let mut changed = p;
        changed[index].k ^= 1;
        assert_ne!(encoded(&changed), encoded(&p));
    }
    assert_eq!(
        evaluate_args(
            &p,
            ARCH,
            libc::SYS_prctl as u32,
            [libc::PR_SET_DUMPABLE as u64, 0]
        ),
        Some(libc::SECCOMP_RET_ALLOW)
    );
    for args in [
        [libc::PR_SET_DUMPABLE as u64, 1],
        [libc::PR_SET_DUMPABLE as u64, 1 << 32],
        [libc::PR_SET_DUMPABLE as u64 | 1 << 32, 0],
        [libc::PR_SET_NO_NEW_PRIVS as u64, 0],
    ] {
        assert_eq!(
            evaluate_args(&p, ARCH, libc::SYS_prctl as u32, args),
            Some(libc::SECCOMP_RET_ERRNO | libc::EPERM as u32)
        );
    }
    println!("policy_instructions=34 serialized_bytes=272 bounded_syscall_numbers=1024 single_thread_fixture_only=true");
}
#[test]
fn signer_seccomp_kernel_info_unknown_partial_or_invalid_flags_refuses() {
    // SAFETY: integral/union C value, zero valid; fields written before active reads.
    let mut info: libc::ptrace_syscall_info = unsafe { std::mem::zeroed() };
    for op in [0, 3, 255] {
        info.op = op;
        assert!(decode(&info, 88).is_err());
    }
    info.op = 1;
    info.arch = ARCH;
    info.u.entry = libc::__c_anonymous_ptrace_syscall_info_entry {
        nr: libc::SYS_seccomp as u64,
        args: [0; 6],
    };
    for bytes in [0, 24, 79, 89, usize::MAX] {
        assert!(decode(&info, bytes).is_err());
    }
    assert!(decode(&info, 80).is_ok());
    info.flags = 1;
    assert!(decode(&info, 80).is_err());
    info.flags = 0;
    info.op = 2;
    info.u.exit = libc::__c_anonymous_ptrace_syscall_info_exit {
        sval: 0,
        is_error: 2,
    };
    assert!(decode(&info, 33).is_err());
    info.u.exit = libc::__c_anonymous_ptrace_syscall_info_exit {
        sval: 0,
        is_error: 0,
    };
    assert!(decode(&info, 32).is_err());
    assert_eq!(
        decode(&info, 33).unwrap(),
        Event::Exit {
            arch: ARCH,
            value: 0,
            error: false
        }
    );
    info.u.exit = libc::__c_anonymous_ptrace_syscall_info_exit {
        sval: 123,
        is_error: 0,
    };
    assert_ne!(
        decode(&info, 33).unwrap(),
        Event::Exit {
            arch: ARCH,
            value: 0,
            error: false
        }
    );
    assert_eq!(peek_word(-1, 0).unwrap(), -1);
    assert!(peek_word(-1, libc::EIO).is_err());
    assert_eq!(peek_word(42, libc::EIO).unwrap(), 42);
}
#[test]
fn signer_seccomp_seeded_probes_and_independent_tracer_threads() {
    let mut seed = 0x0740_3b26_u64;
    for _ in 0..16 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        probe(
            Case::Denied(DENIED[seed as usize % DENIED.len()]),
            Fault::None,
        )
        .unwrap();
    }
    let threads: Vec<_> = (0..4)
        .map(|_| std::thread::spawn(|| probe(Case::ExecFd, Fault::None).unwrap()))
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    println!("seed=0x07403b26 fresh_kernel_probes=16 independent_tracer_threads=4 multithreaded_signer_supported=false");
}
