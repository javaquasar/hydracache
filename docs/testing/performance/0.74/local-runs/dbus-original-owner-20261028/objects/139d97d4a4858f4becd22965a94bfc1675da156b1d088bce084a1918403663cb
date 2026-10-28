//! Standalone local GNU fixture, embedded only by the library test module.
use std::ffi::{c_int, c_long};
use std::os::fd::AsRawFd;

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
fn policy() -> [Filter; 34] {
    // Fixed Linux x86_64 fixture ABI; parent independently checks every byte.
    let mut out = [ins(0, 0, 0, 0); 34];
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
    out[22..].copy_from_slice(&[
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
    out
}
fn main() {
    // A fresh exec permits Rust startup here; no credentials or handoff exists.
    let case = match std::env::args().nth(1).as_deref() {
        Some("self") => 0,
        Some("other") => 1,
        Some("fd") => 2,
        Some("flags") => 11,
        Some("policy") => 12,
        Some("length") => 13,
        Some("header") => 14,
        Some("pointer") => 15,
        Some("early-exec") => 16,
        _ => unsafe { _exit(68) },
    };
    if std::env::vars_os().next().is_some() {
        // SAFETY: immediate termination of this fixture only.
        unsafe { _exit(69) }
    }
    let executable = (case == 2).then(|| std::fs::File::open("/proc/self/exe").unwrap());
    let mut program = policy();
    if case == 12 {
        program[33].k = 0x8000_0000;
    }
    let mut heap_program = (case == 15).then(|| Box::new(program));
    let mut header = Header {
        len: if case == 13 { 1 } else { 34 },
        filter: match &mut heap_program {
            Some(heap) => heap.as_mut().as_mut_ptr(),
            None => program.as_mut_ptr(),
        },
    };
    let heap_header = (case == 14).then(|| Box::new(header));
    let supplied = match &heap_header {
        Some(heap) => heap.as_ref() as *const Header,
        None => &mut header as *mut Header as *const Header,
    };
    let argv = [c"owned-joined-helper".as_ptr(), std::ptr::null()];
    let env = [std::ptr::null::<std::ffi::c_char>()];
    // SAFETY: fixed native ABI, prepared buffers/C strings, no credential data.
    // After filter installation only fixed libc operations and _exit occur.
    unsafe {
        if case == 16 {
            syscall(59, c"/proc/self/exe".as_ptr(), argv.as_ptr(), env.as_ptr());
            _exit(73);
        }
        if syscall(317, 1u64, if case == 11 { 0u64 } else { 1u64 }, supplied) != 0 {
            _exit(70);
        }
        if prctl(4, 0usize, 0usize, 0usize, 0usize) < 0 {
            _exit(71);
        }
        let result = if let Some(file) = &executable {
            syscall(
                322,
                file.as_raw_fd(),
                c"".as_ptr(),
                argv.as_ptr(),
                env.as_ptr(),
                0x1000usize,
            )
        } else {
            let path = if case == 1 {
                c"/bin/true"
            } else {
                c"/proc/self/exe"
            };
            syscall(59, path.as_ptr(), argv.as_ptr(), env.as_ptr())
        };
        _exit(if result == -1 && *__errno_location() == 1 {
            0
        } else {
            72
        });
    }
}
