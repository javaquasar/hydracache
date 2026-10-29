//! Separate GNU helper for one-shot synthetic original-child handoff.
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
    let case = match std::env::args().nth(1).as_deref() {
        Some("good") => 0,
        Some("wrong-ready") => 1,
        Some("duplicate-ready") => 2,
        Some("short") => 3,
        Some("oversized") => 4,
        Some("wrong-fd") => 5,
        Some("early") => 6,
        Some("leak") => 7,
        Some("no-ack") => 8,
        Some("wrong-ack") => 9,
        Some("duplicate-ack") => 10,
        Some("closed") => 11,
        Some("exit") => 12,
        Some("ancillary") => 13,
        Some("replace") => 14,
        Some("rodata") => 15,
        _ => unsafe { _exit(68) },
    };
    if std::env::vars_os().next().is_some() {
        unsafe { _exit(69) }
    }
    let (mut program, count) = program(false);
    let header = Header {
        len: count as u16,
        filter: program.as_mut_ptr(),
    };
    let mut ready = [0u8; 17];
    ready[..16].copy_from_slice(b"HC074-READY-v001");
    if case == 1 {
        ready[0] ^= 1;
    }
    let mut received = [0u8; 16];
    let mut ack = *b"HC074-ACK---v001";
    if case == 9 {
        ack[0] ^= 1;
    }
    let length = if case == 3 {
        15usize
    } else if case == 4 {
        17
    } else {
        16
    };
    let fd = if case == 5 { 1i32 } else { 3 };
    // SAFETY: fixed local owned protocol; after hardening only scalar libc, no allocation.
    unsafe {
        if case == 6 {
            syscall(44, 3i32, ready.as_ptr(), 16usize, 0x4000i32, 0usize, 0usize);
        }
        if case == 7 {
            syscall(33, 0i32, 5i32);
        }
        if syscall(317, 1u64, 1u64, &header) != 0 {
            _exit(70);
        }
        if prctl(4, 0usize, 0usize, 0usize, 0usize) < 0 {
            _exit(71);
        }
        if case == 11 {
            syscall(3, 3i32);
        }
        if case == 12 {
            _exit(72);
        }
        if case == 13 {
            syscall(46, 3i32, 0usize, 0i32);
        }
        if case == 14 {
            syscall(33, 0i32, 3i32);
        }
        let pointer = if case == 15 {
            b"HC074-READY-v001".as_ptr()
        } else {
            ready.as_ptr()
        };
        if syscall(44, fd, pointer, length, 0x4000i32, 0usize, 0usize) != length as c_long {
            _exit(73);
        }
        if case == 2 {
            syscall(44, 3i32, ready.as_ptr(), 16usize, 0x4000i32, 0usize, 0usize);
        }
        if syscall(
            45,
            3i32,
            received.as_mut_ptr(),
            16usize,
            0x20i32,
            0usize,
            0usize,
        ) != 16
            || received != *b"HC074-DUMMY-v001"
        {
            _exit(74);
        }
        if case == 8 {
            _exit(75);
        }
        if syscall(44, 3i32, ack.as_ptr(), 16usize, 0x4000i32, 0usize, 0usize) != 16 {
            _exit(76);
        }
        if case == 10 {
            syscall(44, 3i32, ack.as_ptr(), 16usize, 0x4000i32, 0usize, 0usize);
        }
        if syscall(3, 3i32) != 0 {
            _exit(77);
        }
        _exit(0);
    }
}
