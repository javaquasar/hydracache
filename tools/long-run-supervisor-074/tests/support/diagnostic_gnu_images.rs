pub(crate) fn p16(b: &mut [u8], at: usize, n: u16) {
    b[at..at + 2].copy_from_slice(&n.to_le_bytes());
}
pub(crate) fn p32(b: &mut [u8], at: usize, n: u32) {
    b[at..at + 4].copy_from_slice(&n.to_le_bytes());
}
pub(crate) fn p64(b: &mut [u8], at: usize, n: u64) {
    b[at..at + 8].copy_from_slice(&n.to_le_bytes());
}

// Independent synthetic byte vectors, not compiled helpers or executable inputs.
pub(crate) fn image(interpreter: bool, needed: &[&str], soname: Option<&str>) -> Vec<u8> {
    let mut b = vec![0u8; 4096];
    b[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    p16(&mut b, 16, 3);
    p16(&mut b, 18, 62);
    p32(&mut b, 20, 1);
    p64(&mut b, 24, 0x10200);
    p64(&mut b, 32, 64);
    p16(&mut b, 52, 64);
    p16(&mut b, 54, 56);
    let loader = b"/lib64/ld-linux-x86-64.so.2\0";
    let mut table = vec![0u8];
    let mut tags = Vec::new();
    for name in needed {
        tags.push((1u64, table.len() as u64));
        table.extend_from_slice(name.as_bytes());
        table.push(0);
    }
    if let Some(name) = soname {
        tags.push((14, table.len() as u64));
        table.extend_from_slice(name.as_bytes());
        table.push(0);
    }
    tags.extend([(5, 0x10800), (10, table.len() as u64), (0, 0)]);
    let mut headers = vec![
        (1u32, 5u32, 0u64, 4096u64, 4096u64),
        (2, 4, 1024, (tags.len() * 16) as u64, 8),
        (0x6474_e551, 6, 0, 0, 16),
    ];
    if interpreter {
        headers.push((3, 4, 768, loader.len() as u64, 1));
    }
    p16(&mut b, 56, headers.len() as u16);
    for (i, (tag, flags, offset, bytes, align)) in headers.into_iter().enumerate() {
        let at = 64 + i * 56;
        p32(&mut b, at, tag);
        p32(&mut b, at + 4, flags);
        p64(&mut b, at + 8, offset);
        p64(
            &mut b,
            at + 16,
            if bytes == 0 { 0 } else { 0x10000 + offset },
        );
        p64(&mut b, at + 32, bytes);
        p64(&mut b, at + 40, bytes);
        p64(&mut b, at + 48, align);
    }
    if interpreter {
        b[768..768 + loader.len()].copy_from_slice(loader);
    }
    for (i, (tag, value)) in tags.into_iter().enumerate() {
        p64(&mut b, 1024 + i * 16, tag);
        p64(&mut b, 1032 + i * 16, value);
    }
    b[2048..2048 + table.len()].copy_from_slice(&table);
    b
}
pub(crate) fn images() -> [Vec<u8>; 4] {
    [
        image(true, &["libc.so.6", "libgcc_s.so.1"], None),
        image(false, &[], Some("ld-linux-x86-64.so.2")),
        image(false, &["ld-linux-x86-64.so.2"], Some("libc.so.6")),
        image(false, &["libc.so.6"], Some("libgcc_s.so.1")),
    ]
}
