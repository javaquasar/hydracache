//! Bounded kernel VMA metadata only, never a loaded-memory attestation.
use super::{refused, require, Root};
use std::collections::BTreeSet;
use std::io;
fn hex(raw: &str) -> io::Result<u64> {
    require(!raw.is_empty() && raw.len() <= 16 && raw.bytes().all(|b| b.is_ascii_hexdigit()))?;
    u64::from_str_radix(raw, 16).map_err(|_| refused())
}
pub(super) fn validate(raw: &str, root: &Root, loaded: bool) -> io::Result<()> {
    require(!raw.is_empty() && raw.len() <= 65536 && raw.ends_with('\n') && root.files.len() == 4)?;
    let path = std::fs::read_link(format!(
        "/proc/self/fd/{}",
        std::os::fd::AsRawFd::as_raw_fd(&root.dirs[0].file)
    ))?;
    let relative = [
        "signer",
        "lib64/ld-linux-x86-64.so.2",
        "lib/x86_64-linux-gnu/libc.so.6",
        "lib/x86_64-linux-gnu/libgcc_s.so.1",
    ];
    let mut previous = 0;
    let mut seen = BTreeSet::new();
    let mut executable = BTreeSet::new();
    let mut pseudo = BTreeSet::new();
    for (index, line) in raw.lines().enumerate() {
        require(index < 128)?;
        let p: Vec<_> = line.split_whitespace().collect();
        require((5..=6).contains(&p.len()))?;
        let (lo, hi) = p[0].split_once('-').ok_or_else(refused)?;
        let (lo, hi) = (hex(lo)?, hex(hi)?);
        let offset = hex(p[2])?;
        let (major, minor) = p[3].split_once(':').ok_or_else(refused)?;
        let (major, minor) = (hex(major)?, hex(minor)?);
        require(!p[4].is_empty() && p[4].len() <= 20 && p[4].bytes().all(|b| b.is_ascii_digit()))?;
        let inode: u64 = p[4].parse().map_err(|_| refused())?;
        let name = p.get(5).copied().unwrap_or("");
        require(
            lo != 0
                && lo < hi
                && lo >= previous
                && lo % 4096 == 0
                && hi % 4096 == 0
                && offset % 4096 == 0
                && ["r--p", "rw-p", "r-xp", "---p", "--xp"].contains(&p[1]),
        )?;
        previous = hi;
        if inode != 0 {
            let role = root
                .files
                .iter()
                .position(|file| {
                    libc::major(file.stamp.dev) as u64 == major
                        && libc::minor(file.stamp.dev) as u64 == minor
                        && file.stamp.ino == inode
                })
                .ok_or_else(refused)?;
            require(loaded || role < 2)?;
            let rounded = root.files[role]
                .stamp
                .len
                .checked_add(4095)
                .ok_or_else(refused)?
                & !4095;
            require(
                offset
                    .checked_add(hi - lo)
                    .is_some_and(|end| end <= rounded)
                    && (name == format!("/{}", relative[role])
                        || Some(name) == path.join(relative[role]).to_str()),
            )?;
            seen.insert(role);
            if p[1] == "r-xp" {
                executable.insert(role);
            }
        } else {
            require(major == 0 && minor == 0 && offset == 0)?;
            match name {
                "" if !p[1].contains('x') => (),
                "[stack]" | "[heap]" if p[1] == "rw-p" && pseudo.insert(name) => (),
                "[vvar]" | "[vvar_vclock]" if p[1] == "r--p" && pseudo.insert(name) => (),
                "[vdso]" if p[1] == "r-xp" && pseudo.insert(name) => (),
                "[vsyscall]"
                    if p[1] == "--xp"
                        && lo == 0xffff_ffff_ff60_0000
                        && hi == 0xffff_ffff_ff60_1000
                        && pseudo.insert(name) => {}
                _ => return Err(refused()),
            }
        }
    }
    let expected: BTreeSet<_> = (0..if loaded { 4 } else { 2 }).collect();
    require(
        seen == expected
            && executable == expected
            && pseudo.contains("[stack]")
            && pseudo.contains("[vdso]"),
    )
}
