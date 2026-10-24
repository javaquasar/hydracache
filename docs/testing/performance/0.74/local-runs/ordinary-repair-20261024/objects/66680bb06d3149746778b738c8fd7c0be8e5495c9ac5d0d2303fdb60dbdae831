//! Bounded kernel VMA metadata, never loaded-memory or map_files attestation.
use super::super::{Root, FILES};
use super::invalid;
use std::collections::BTreeSet;
use std::io;

fn hex(raw: &str) -> io::Result<u64> {
    if raw.is_empty() || raw.len() > 16 || !raw.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(invalid());
    }
    u64::from_str_radix(raw, 16).map_err(|_| invalid())
}
pub(super) fn validate(raw: &str, root: &Root, loaded: bool) -> io::Result<(usize, usize)> {
    if raw.is_empty() || raw.len() > 65536 || !raw.ends_with('\n') || root.files.len() != 4 {
        return Err(invalid());
    }
    let mut previous = 0;
    let mut seen = BTreeSet::new();
    let mut executable = BTreeSet::new();
    let mut stack = None;
    let mut vdso = false;
    let mut pseudo = BTreeSet::new();
    for (index, line) in raw.lines().enumerate() {
        if index >= 128 {
            return Err(invalid());
        }
        let p: Vec<_> = line.split_whitespace().collect();
        if !(5..=6).contains(&p.len()) {
            return Err(invalid());
        }
        let (lo, hi) = p[0].split_once('-').ok_or_else(invalid)?;
        let (lo, hi) = (hex(lo)?, hex(hi)?);
        let offset = hex(p[2])?;
        let (major, minor) = p[3].split_once(':').ok_or_else(invalid)?;
        let (major, minor) = (hex(major)?, hex(minor)?);
        if p[4].len() > 20 || p[4].is_empty() || !p[4].bytes().all(|b| b.is_ascii_digit()) {
            return Err(invalid());
        }
        let inode = p[4].parse::<u64>().map_err(|_| invalid())?;
        let name = p.get(5).copied().unwrap_or("");
        if lo == 0
            || lo >= hi
            || lo < previous
            || lo % 4096 != 0
            || hi % 4096 != 0
            || offset % 4096 != 0
            || !["r--p", "rw-p", "r-xp", "---p", "--xp"].contains(&p[1])
        {
            return Err(invalid());
        }
        previous = hi;
        if inode != 0 {
            let selected = root
                .files
                .iter()
                .position(|file| {
                    let dev = file.stamp.0[0];
                    libc::major(dev) as u64 == major
                        && libc::minor(dev) as u64 == minor
                        && file.stamp.0[1] == inode
                })
                .ok_or_else(invalid)?;
            if !loaded && selected > 1 {
                return Err(invalid());
            }
            let rounded = root.files[selected].stamp.0[6]
                .checked_add(4095)
                .ok_or_else(invalid)?
                & !4095;
            if offset.checked_add(hi - lo).is_none_or(|end| end > rounded)
                || (name != format!("/{}", FILES[selected])
                    && name
                        != root
                            .path
                            .join(FILES[selected])
                            .to_str()
                            .ok_or_else(invalid)?)
            {
                return Err(invalid());
            }
            seen.insert(selected);
            if p[1] == "r-xp" {
                executable.insert(selected);
            }
        } else {
            if major != 0 || minor != 0 || offset != 0 {
                return Err(invalid());
            }
            match name {
                "" if !p[1].contains('x') => {}
                "[heap]" if p[1] == "rw-p" => {}
                "[stack]" if p[1] == "rw-p" && stack.is_none() => {
                    stack = Some((lo as usize, hi as usize))
                }
                "[vvar]" | "[vvar_vclock]" if p[1] == "r--p" && pseudo.insert(name) => {}
                "[vdso]" if p[1] == "r-xp" && !vdso => vdso = true,
                "[vsyscall]"
                    if p[1] == "--xp"
                        && lo == 0xffff_ffff_ff60_0000
                        && hi == 0xffff_ffff_ff60_1000
                        && pseudo.insert(name) => {}
                _ => return Err(invalid()),
            }
        }
    }
    let expected: BTreeSet<_> = (0..if loaded { 4 } else { 2 }).collect();
    if seen != expected || executable != expected || !vdso {
        return Err(invalid());
    }
    stack.ok_or_else(invalid)
}
