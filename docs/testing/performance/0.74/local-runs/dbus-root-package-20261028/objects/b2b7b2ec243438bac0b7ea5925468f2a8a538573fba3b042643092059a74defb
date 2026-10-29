//! Separate bounded seventeen-role closed set. The old checker is unchanged.
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
        need(*entries <= 20)?;
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
                17
            } else {
                need((0..17).contains(&value))?;
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
        fd >= 17
            && unsafe { libc::fstatfs(fd, &mut fs) } == 0
            && fs.f_type == libc::PROC_SUPER_MAGIC as libc::c_long,
    )?;
    let (mut seen, mut entries, mut reads) = (0, 0, 0);
    let mut raw = [0; 4096];
    loop {
        reads += 1;
        need(reads <= 21)?;
        let n = unsafe { libc::syscall(libc::SYS_getdents64, fd, raw.as_mut_ptr(), raw.len()) };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        if n == 0 {
            break;
        }
        need(n as usize <= raw.len())?;
        accept(&raw[..n as usize], fd, &mut seen, &mut entries)?;
    }
    need(seen == 0x3ffff && entries == 20)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn row(name: &[u8]) -> Vec<u8> {
        let mut v = vec![0; 20 + name.len()];
        let n = v.len() as u16;
        v[16..18].copy_from_slice(&n.to_ne_bytes());
        v[19..19 + name.len()].copy_from_slice(name);
        v
    }
    #[test]
    fn bootstrap_installed_inventory_exact_set_is_bounded_and_not_old_set() {
        let mut v = row(b".");
        v.extend(row(b".."));
        for fd in 0..17 {
            v.extend(row(fd.to_string().as_bytes()));
        }
        v.extend(row(b"74"));
        let (mut seen, mut n) = (0, 0);
        accept(&v, 74, &mut seen, &mut n).unwrap();
        assert_eq!((seen, n), (0x3ffff, 20));
        assert!(accept(&row(b"1"), 74, &mut seen, &mut n).is_err());
    }
    #[test]
    fn bootstrap_installed_inventory_unknown_short_duplicate_and_noncanonical_refuse() {
        for name in [
            b"".as_slice(),
            b"00",
            b"-1",
            b"+1",
            b"17",
            b"2147483648",
            b"99999999999",
            b"x",
        ] {
            assert!(accept(&row(name), 74, &mut 0, &mut 0).is_err());
        }
        for n in 1..20 {
            assert!(accept(&vec![0; n], 74, &mut 0, &mut 0).is_err());
        }
        let mut v = row(b"1");
        v.extend(row(b"1"));
        assert!(accept(&v, 74, &mut 0, &mut 0).is_err());
        let mut v = row(b"1");
        *v.last_mut().unwrap() = b'1';
        assert!(accept(&v, 74, &mut 0, &mut 0).is_err());
        for size in [0u16, 19, 65535] {
            let mut v = row(b"1");
            v[16..18].copy_from_slice(&size.to_ne_bytes());
            assert!(accept(&v, 74, &mut 0, &mut 0).is_err());
        }
    }
}
