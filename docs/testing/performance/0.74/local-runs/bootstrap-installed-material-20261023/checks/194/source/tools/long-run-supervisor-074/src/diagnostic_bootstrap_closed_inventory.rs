//! Bounded procfs enumeration; no descriptor closing or repair.
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
        need(*entries <= 18)?;
        if name != b"." && name != b".." {
            need(
                !name.is_empty()
                    && name.len() <= 10
                    && (name.len() == 1 || name[0] != b'0')
                    && name.iter().all(u8::is_ascii_digit),
            )?;
            let value = std::str::from_utf8(name)
                .map_err(|_| refused())?
                .parse::<i32>()
                .map_err(|_| refused())?;
            let slot = if value == fd {
                15
            } else {
                need((0..15).contains(&value))?;
                value as u32
            };
            need(*seen & (1 << slot) == 0)?;
            *seen |= 1 << slot;
        }
        at += len;
    }
    Ok(())
}
pub(super) fn inspect() -> io::Result<()> {
    let directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open("/proc/thread-self/fd")?;
    let fd = directory.as_raw_fd();
    let mut fs: libc::statfs = unsafe { std::mem::zeroed() };
    need(
        fd >= 15
            && unsafe { libc::fstatfs(fd, &mut fs) } == 0
            && fs.f_type == libc::PROC_SUPER_MAGIC as libc::c_long,
    )?;
    let mut seen = 0u32;
    let mut entries = 0usize;
    let mut reads = 0usize;
    let mut buffer = [0u8; 4096];
    loop {
        reads += 1;
        need(reads <= 19)?;
        let n =
            unsafe { libc::syscall(libc::SYS_getdents64, fd, buffer.as_mut_ptr(), buffer.len()) };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        if n == 0 {
            break;
        }
        need(n as usize <= buffer.len())?;
        accept(&buffer[..n as usize], fd, &mut seen, &mut entries)?;
    }
    need(seen == 0xffff && entries == 18)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record(name: &[u8]) -> Vec<u8> {
        let mut raw = vec![0; 20 + name.len()];
        let len = raw.len() as u16;
        raw[16..18].copy_from_slice(&len.to_ne_bytes());
        raw[19..19 + name.len()].copy_from_slice(name);
        raw
    }
    #[test]
    fn bootstrap_inventory_parser_exact_bounded_set_and_own_descriptor() {
        let mut raw = record(b".");
        raw.extend(record(b".."));
        for i in 0..15 {
            raw.extend(record(i.to_string().as_bytes()));
        }
        raw.extend(record(b"74"));
        let (mut seen, mut count) = (0, 0);
        accept(&raw, 74, &mut seen, &mut count).unwrap();
        assert_eq!((seen, count), (0xffff, 18));
        assert!(accept(&record(b"0"), 74, &mut seen, &mut count).is_err());
    }
    #[test]
    fn bootstrap_inventory_parser_short_duplicate_oversize_and_unknown_names_refuse() {
        for name in [
            b"".as_slice(),
            b"00",
            b"-1",
            b"+1",
            b"15",
            b"2147483648",
            b"99999999999",
            b"x",
        ] {
            assert!(accept(&record(name), 74, &mut 0, &mut 0).is_err());
        }
        for n in 1..20 {
            assert!(accept(&vec![0; n], 74, &mut 0, &mut 0).is_err());
        }
        for len in [0, 19, 65535] {
            let mut bad = record(b"1");
            bad[16..18].copy_from_slice(&(len as u16).to_ne_bytes());
            assert!(accept(&bad, 74, &mut 0, &mut 0).is_err());
        }
        let mut no_nul = record(b"1");
        *no_nul.last_mut().unwrap() = b'1';
        assert!(accept(&no_nul, 74, &mut 0, &mut 0).is_err());
        let mut duplicate = record(b"1");
        duplicate.extend(record(b"1"));
        assert!(accept(&duplicate, 74, &mut 0, &mut 0).is_err());
    }
}
