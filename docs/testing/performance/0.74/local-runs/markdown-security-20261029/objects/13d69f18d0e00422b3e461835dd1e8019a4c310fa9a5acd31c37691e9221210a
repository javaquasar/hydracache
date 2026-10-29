//! Separate GNU helper for the local post-install mutation policy.
use std::ffi::{c_int, c_long};
#[link(name = "c")]
extern "C" {
    fn syscall(number: c_long, ...) -> c_long;
    fn prctl(option: c_int, ...) -> c_int;
    fn __errno_location() -> *mut c_int;
    fn _exit(status: c_int) -> !;
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Filter {
    code: u16,
    jt: u8,
    jf: u8,
    k: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Header {
    len: u16,
    filter: *mut Filter,
}
const fn ins(code: u16, jt: u8, jf: u8, k: u32) -> Filter {
    Filter { code, jt, jf, k }
}
fn program(control: bool) -> ([Filter; 64], usize) {
    let mut out = [ins(0, 0, 0, 0); 64];
    out[..6].copy_from_slice(&[
        ins(0x20, 0, 0, 4),
        ins(0x15, 1, 0, 0xc000_003e),
        ins(0x06, 0, 0, 0x8000_0000),
        ins(0x20, 0, 0, 0),
        ins(0x45, 0, 1, 0x4000_0000),
        ins(0x06, 0, 0, 0x8000_0000),
    ]);
    for (index, nr) in [59, 322, 57, 58, 56, 435, 101, 317].into_iter().enumerate() {
        out[6 + index * 2] = ins(0x15, 0, 1, nr);
        out[7 + index * 2] = ins(0x06, 0, 0, 0x0005_0001);
    }
    let tail = if control {
        22
    } else {
        for (index, nr) in [
            272, 308, 161, 80, 81, 165, 166, 155, 428, 429, 430, 431, 432, 433, 442,
        ]
        .into_iter()
        .enumerate()
        {
            out[22 + index * 2] = ins(0x15, 0, 1, nr);
            out[23 + index * 2] = ins(0x06, 0, 0, 0x0005_0001);
        }
        52
    };
    out[tail..tail + 12].copy_from_slice(&[
        ins(0x15, 0, 10, 157),
        ins(0x20, 0, 0, 16),
        ins(0x15, 0, 7, 4),
        ins(0x20, 0, 0, 20),
        ins(0x15, 0, 5, 0),
        ins(0x20, 0, 0, 24),
        ins(0x15, 0, 3, 0),
        ins(0x20, 0, 0, 28),
        ins(0x15, 0, 1, 0),
        ins(0x06, 0, 0, 0x7fff_0000),
        ins(0x06, 0, 0, 0x0005_0001),
        ins(0x06, 0, 0, 0x7fff_0000),
    ]);
    (out, tail + 12)
}
fn main() {
    let (case, control) = match std::env::args().nth(1).as_deref() {
        Some("unshare") => (0, false),
        Some("setns") => (1, false),
        Some("chroot") => (2, false),
        Some("chdir") => (3, false),
        Some("fchdir") => (4, false),
        Some("mount") => (5, false),
        Some("umount") => (6, false),
        Some("pivot") => (7, false),
        Some("open-tree") => (8, false),
        Some("move-mount") => (9, false),
        Some("fsopen") => (10, false),
        Some("fsconfig") => (11, false),
        Some("fsmount") => (12, false),
        Some("fspick") => (13, false),
        Some("mount-setattr") => (14, false),
        Some("control-unshare") => (0, true),
        Some("control-setns") => (1, true),
        Some("control-chdir") => (3, true),
        Some("control-fchdir") => (4, true),
        Some("self") => (16, false),
        Some("other") => (17, false),
        Some("x32") => (18, false),
        Some("i386") => (19, false),
        Some("flags") => (21, false),
        Some("policy") => (22, false),
        Some("length") => (23, false),
        Some("header") => (24, false),
        Some("pointer") => (25, false),
        Some("early-exec") => (26, false),
        Some("early-mount") => (27, false),
        _ => unsafe { _exit(68) },
    };
    if std::env::vars_os().next().is_some() {
        unsafe { _exit(69) }
    }
    let (mut program, count) = program(control);
    if case == 22 {
        program[count - 1].k = 0x8000_0000;
    }
    let mut heap_program = (case == 25).then(|| Box::new(program));
    let mut header = Header {
        len: if case == 23 { 1 } else { count as u16 },
        filter: match &mut heap_program {
            Some(heap) => heap.as_mut().as_mut_ptr(),
            None => program.as_mut_ptr(),
        },
    };
    let heap_header = (case == 24).then(|| Box::new(header));
    let supplied = match &heap_header {
        Some(heap) => heap.as_ref() as *const Header,
        None => &mut header as *mut Header as *const Header,
    };
    let path = c"/".as_ptr() as usize as u64;
    let none = c"none".as_ptr() as usize as u64;
    let (nr, args): (c_long, [u64; 6]) = match case {
        0 => (272, [0x1000_0000, 0, 0, 0, 0, 0]),
        1 => (308, [u64::MAX, 0, 0, 0, 0, 0]),
        2 => (161, [path, 0, 0, 0, 0, 0]),
        3 => (80, [path, 0, 0, 0, 0, 0]),
        4 => (81, [0, 0, 0, 0, 0, 0]),
        5 => (165, [none, path, none, 0, 0, 0]),
        6 => (166, [path, 0, 0, 0, 0, 0]),
        7 => (155, [path, path, 0, 0, 0, 0]),
        8 => (428, [u64::MAX, path, 0, 0, 0, 0]),
        9 => (429, [u64::MAX, path, u64::MAX, path, 0, 0]),
        10 => (430, [none, 0, 0, 0, 0, 0]),
        11 => (431, [u64::MAX, 0, 0, 0, 0, 0]),
        12 => (432, [u64::MAX, 0, 0, 0, 0, 0]),
        13 => (433, [u64::MAX, path, 0, 0, 0, 0]),
        14 => (442, [u64::MAX, path, 0, 0, 0, 0]),
        _ => (39, [0; 6]),
    };
    let argv = [c"owned-namespace-helper".as_ptr(), std::ptr::null()];
    let env = [std::ptr::null::<std::ffi::c_char>()];
    // SAFETY: prepared fixed ABI/scalars; after install only libc/asm/_exit, no handoff.
    unsafe {
        if case == 26 {
            syscall(59, c"/signer".as_ptr(), argv.as_ptr(), env.as_ptr());
            _exit(73);
        }
        if case == 27 {
            syscall(430, none, 0u64);
            _exit(73);
        }
        if syscall(317, 1u64, if case == 21 { 0u64 } else { 1u64 }, supplied) != 0 {
            _exit(70);
        }
        if prctl(4, 0usize, 0usize, 0usize, 0usize) < 0 {
            _exit(71);
        }
        if case == 19 {
            core::arch::asm!("push rbx","xor ebx, ebx","xor ecx, ecx","xor edx, edx","mov eax, 20","int 0x80","pop rbx",lateout("eax") _,lateout("ecx") _,lateout("edx") _);
            _exit(72);
        }
        let result = if case == 18 {
            syscall(0x4000_0027u64 as c_long, 0usize, 0usize, 0usize)
        } else if case == 16 || case == 17 {
            syscall(
                59,
                if case == 16 {
                    c"/signer".as_ptr()
                } else {
                    c"/bin/true".as_ptr()
                },
                argv.as_ptr(),
                env.as_ptr(),
            )
        } else {
            syscall(nr, args[0], args[1], args[2], args[3], args[4], args[5])
        };
        let okay = if control && (case == 0 || case == 3) {
            result == 0
        } else {
            result == -1
                && *__errno_location()
                    == if control {
                        if case == 1 {
                            9
                        } else {
                            20
                        }
                    } else {
                        1
                    }
        };
        _exit(if okay { 0 } else { 72 });
    }
}
