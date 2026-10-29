//! Fixed child restriction primitive and terminating nonproduct probe, never launch authority.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct Instruction {
    pub(crate) code: u16,
    pub(crate) jt: u8,
    pub(crate) jf: u8,
    pub(crate) k: u32,
}
const fn ins(code: u16, jt: u8, jf: u8, k: u32) -> Instruction {
    Instruction { code, jt, jf, k }
}
pub(crate) fn program() -> [Instruction; 64] {
    let mut out = [ins(0, 0, 0, 0); 64];
    out[..6].copy_from_slice(&[
        ins(0x20, 0, 0, 4),
        ins(0x15, 1, 0, 0xc000_003e),
        ins(0x06, 0, 0, 0x8000_0000),
        ins(0x20, 0, 0, 0),
        ins(0x45, 0, 1, 0x4000_0000),
        ins(0x06, 0, 0, 0x8000_0000),
    ]);
    for (i, nr) in [
        59, 322, 57, 58, 56, 435, 101, 317, 272, 308, 161, 80, 81, 165, 166, 155, 428, 429, 430,
        431, 432, 433, 442,
    ]
    .into_iter()
    .enumerate()
    {
        out[6 + i * 2] = ins(0x15, 0, 1, nr);
        out[7 + i * 2] = ins(0x06, 0, 0, 0x0005_0001);
    }
    out[52..].copy_from_slice(&[
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

/// Exact existing 64-instruction policy bytes; data only, not an installed guard.
pub fn policy_bytes() -> [u8; 512] {
    let mut out = [0; 512];
    for (i, item) in program().iter().enumerate() {
        let code = item.code.to_le_bytes();
        let k = item.k.to_le_bytes();
        out[i * 8..i * 8 + 8]
            .copy_from_slice(&[code[0], code[1], item.jt, item.jf, k[0], k[1], k[2], k[3]]);
    }
    out
}

/// Closed nonproduct cases; no command/path/PID/FD/key/filter override.
#[derive(Clone, Copy)]
pub enum FixedProbe {
    Native,
    X32,
    I386,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "diagnostic_signer_isolation_linux.rs"]
mod linux;

/// Terminates the CURRENT process. Use only an owned, fresh, secret-free probe child.
/// Refuses root, nonempty environment, multiple threads/capabilities and extra FDs.
/// Never returns a bootstrap/signing/start capability or executes another image.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub fn run_fixed_probe(probe: FixedProbe) -> ! {
    linux::run(probe)
}
