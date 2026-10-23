//! Closed eighteen-role set; no descriptor is adopted by this census.
use super::*;
fn accept(raw: &[u8], fd: i32, seen: &mut u32, entries: &mut usize) -> io::Result<()> {
    let mut at = 0;
    while at < raw.len() {
        need(raw.len() - at >= 20)?;
        let len = u16::from_ne_bytes([raw[at + 16], raw[at + 17]]) as usize;
        need(len >= 20 && len <= raw.len() - at)?;
        let name = &raw[at + 19..at + len];
        let end = name.iter().position(|b| *b == 0).ok_or_else(refused)?;
        let name = &name[..end];
        *entries += 1;
        need(*entries <= 21)?;
        if name != b"." && name != b".." {
            need(!name.is_empty() && name.len() <= 10 && (name.len() == 1 || name[0] != b'0') && name.iter().all(u8::is_ascii_digit))?;
            let value = std::str::from_utf8(name).map_err(|_| refused())?.parse::<i32>().map_err(|_| refused())?;
            let slot = if value == fd { 18 } else { need((0..18).contains(&value))?; value as u32 };
            need(*seen & (1 << slot) == 0)?;
            *seen |= 1 << slot;
        }
        at += len;
    }
    Ok(())
}
pub(super) fn inspect() -> io::Result<()> {
    let directory = OpenOptions::new().read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open("/proc/thread-self/fd")?;
    let fd = directory.as_raw_fd();
    let mut fs: libc::statfs = unsafe { std::mem::zeroed() };
    need(fd >= 18 && unsafe { libc::fstatfs(fd, &mut fs) } == 0 && fs.f_type == libc::PROC_SUPER_MAGIC as libc::c_long)?;
    let (mut seen, mut entries, mut reads) = (0, 0, 0);
    let mut raw = [0; 4096];
    loop {
        reads += 1;
        need(reads <= 22)?;
        let n = unsafe { libc::syscall(libc::SYS_getdents64, fd, raw.as_mut_ptr(), raw.len()) };
        if n < 0 { return Err(io::Error::last_os_error()); }
        if n == 0 { break; }
        need(n as usize <= raw.len())?;
        accept(&raw[..n as usize], fd, &mut seen, &mut entries)?;
    }
    need(seen == 0x7ffff && entries == 21)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn row(name: &[u8]) -> Vec<u8> {
        let mut raw = vec![0; 20+name.len()]; let len = raw.len() as u16;
        raw[16..18].copy_from_slice(&len.to_ne_bytes()); raw[19..19+name.len()].copy_from_slice(name); raw
    }
    #[test]
    fn held_entry_inventory_exact_unknown_duplicate_and_short() {
        let mut raw = row(b"."); raw.extend(row(b".."));
        for fd in 0..18 { raw.extend(row(fd.to_string().as_bytes())); }
        raw.extend(row(b"74"));
        let (mut seen, mut count) = (0,0);
        accept(&raw,74,&mut seen,&mut count).unwrap();
        assert_eq!((seen,count),(0x7ffff,21));
        assert!(accept(&row(b"17"),74,&mut seen,&mut count).is_err());
        for bad in [b"18".as_slice(), b"00", b"-1", b"2147483648", b""] {
            assert!(accept(&row(bad),74,&mut 0,&mut 0).is_err());
        }
        for len in 1..20 { assert!(accept(&vec![0;len],74,&mut 0,&mut 0).is_err()); }
    }
}
