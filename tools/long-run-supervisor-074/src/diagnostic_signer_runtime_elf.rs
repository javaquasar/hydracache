//! Bounded declaration inventory, not a complete ELF/linker or runtime verifier.
use super::invalid;
use std::collections::BTreeSet;
use std::io;

pub(super) const MAX_IMAGE: usize = 8 * 1024 * 1024;
pub(super) const LOADER: &str = "/lib64/ld-linux-x86-64.so.2";
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Info {
    pub(super) interpreter: Option<String>,
    pub(super) needed: Vec<String>,
    pub(super) soname: Option<String>,
    pub(super) kind: u16,
    pub(super) program_headers: usize,
}
#[derive(Clone)]
struct Segment {
    flags: u32,
    offset: u64,
    va: u64,
    file: u64,
    memory: u64,
}
fn range(bytes: &[u8], offset: u64, count: u64) -> io::Result<&[u8]> {
    let end = offset.checked_add(count).ok_or_else(invalid)?;
    bytes
        .get(
            usize::try_from(offset).map_err(|_| invalid())?
                ..usize::try_from(end).map_err(|_| invalid())?,
        )
        .ok_or_else(invalid)
}
fn u16_at(bytes: &[u8], offset: u64) -> io::Result<u16> {
    Ok(u16::from_le_bytes(
        range(bytes, offset, 2)?.try_into().unwrap(),
    ))
}
fn u32_at(bytes: &[u8], offset: u64) -> io::Result<u32> {
    Ok(u32::from_le_bytes(
        range(bytes, offset, 4)?.try_into().unwrap(),
    ))
}
fn u64_at(bytes: &[u8], offset: u64) -> io::Result<u64> {
    Ok(u64::from_le_bytes(
        range(bytes, offset, 8)?.try_into().unwrap(),
    ))
}
fn intersects(a: u64, an: u64, b: u64, bn: u64) -> bool {
    an > 0 && bn > 0 && a < b + bn && b < a + an
}
fn mapped(loads: &[Segment], address: u64, count: u64) -> io::Result<u64> {
    let end = address.checked_add(count).ok_or_else(invalid)?;
    if count == 0 {
        return Err(invalid());
    }
    let mut found = None;
    for load in loads {
        if load.flags & 4 != 0 && address >= load.va && end <= load.va + load.file {
            if found.is_some() {
                return Err(invalid());
            }
            found = Some(load.offset + (address - load.va));
        }
    }
    found.ok_or_else(invalid)
}
fn name(table: &[u8], offset: u64) -> io::Result<String> {
    let tail = table
        .get(usize::try_from(offset).map_err(|_| invalid())?..)
        .ok_or_else(invalid)?;
    let count = tail
        .iter()
        .take(129)
        .position(|b| *b == 0)
        .ok_or_else(invalid)?;
    if count == 0 || count > 128 {
        return Err(invalid());
    }
    let raw = &tail[..count];
    if raw.windows(2).any(|s| s == b"..")
        || !raw
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(b))
    {
        return Err(invalid());
    }
    String::from_utf8(raw.to_vec()).map_err(|_| invalid())
}
pub(super) fn parse(bytes: &[u8], executable: bool) -> io::Result<Info> {
    if bytes.len() < 64
        || bytes.len() > MAX_IMAGE
        || &bytes[..7] != b"\x7fELF\x02\x01\x01"
        || ![0, 3].contains(&bytes[7])
        || bytes[8..16].iter().any(|b| *b != 0)
        || u16_at(bytes, 18)? != 62
        || u32_at(bytes, 20)? != 1
        || u32_at(bytes, 48)? != 0
        || u16_at(bytes, 52)? != 64
        || u16_at(bytes, 54)? != 56
    {
        return Err(invalid());
    }
    let kind = u16_at(bytes, 16)?;
    if ![2, 3].contains(&kind) || (!executable && kind != 3) {
        return Err(invalid());
    }
    let start = u64_at(bytes, 32)?;
    let count = u16_at(bytes, 56)? as usize;
    if start < 64 || start % 8 != 0 || count == 0 || count > 32 {
        return Err(invalid());
    }
    range(bytes, start, (count * 56) as u64)?;
    let mut loads: Vec<Segment> = Vec::new();
    let mut dynamic = None;
    let mut interpreter = None;
    let mut stack = false;
    let mut singleton = BTreeSet::new();
    for index in 0..count {
        let p = range(bytes, start + (index * 56) as u64, 56)?;
        let tag = u32_at(p, 0)?;
        if !matches!(tag, 1 | 4) && !singleton.insert(tag) {
            return Err(invalid());
        }
        let segment = Segment {
            flags: u32_at(p, 4)?,
            offset: u64_at(p, 8)?,
            va: u64_at(p, 16)?,
            file: u64_at(p, 32)?,
            memory: u64_at(p, 40)?,
        };
        let align = u64_at(p, 48)?;
        range(bytes, segment.offset, segment.file)?;
        if segment.file > segment.memory
            || segment.flags & !7 != 0
            || segment.flags & 3 == 3
            || segment.va.checked_add(segment.memory).is_none()
            || (align > 1
                && (!align.is_power_of_two()
                    || align > 2 * 1024 * 1024
                    || segment.va % align != segment.offset % align))
        {
            return Err(invalid());
        }
        match tag {
            1 => {
                if segment.memory == 0
                    || loads.len() >= 16
                    || loads.iter().any(|old| {
                        intersects(old.va, old.memory, segment.va, segment.memory)
                            || intersects(old.offset, old.file, segment.offset, segment.file)
                    })
                {
                    return Err(invalid());
                }
                loads.push(segment);
            }
            2 => {
                if dynamic.replace(segment).is_some() {
                    return Err(invalid());
                }
            }
            3 => {
                if interpreter.replace(segment).is_some() {
                    return Err(invalid());
                }
            }
            0x6474_e551 => {
                if stack || segment.flags & 1 != 0 {
                    return Err(invalid());
                }
                stack = true;
            }
            4 | 6 | 7 | 0x6474_e550 | 0x6474_e552 | 0x6474_e553 => {}
            _ => return Err(invalid()),
        }
    }
    if loads.is_empty() || !stack {
        return Err(invalid());
    }
    let entry = u64_at(bytes, 24)?;
    if (executable || entry != 0)
        && !loads
            .iter()
            .any(|l| l.flags & 1 != 0 && entry >= l.va && entry < l.va + l.memory)
    {
        return Err(invalid());
    }
    let interpreter = match interpreter {
        Some(s) => {
            if s.file == 0 || s.file > 128 || mapped(&loads, s.va, s.file)? != s.offset {
                return Err(invalid());
            }
            let raw = range(bytes, s.offset, s.file)?;
            if raw.last() != Some(&0) || raw[..raw.len() - 1] != *LOADER.as_bytes() {
                return Err(invalid());
            }
            Some(LOADER.to_owned())
        }
        None if executable => return Err(invalid()),
        None => None,
    };
    let d = dynamic.ok_or_else(invalid)?;
    if d.file == 0
        || d.file % 16 != 0
        || d.file / 16 > 512
        || mapped(&loads, d.va, d.file)? != d.offset
    {
        return Err(invalid());
    }
    let raw = range(bytes, d.offset, d.file)?;
    let mut tags = BTreeSet::new();
    let mut needed = Vec::new();
    let mut soname = None;
    let mut strings = None;
    let mut string_size = None;
    let mut terminated = false;
    for (index, pair) in raw.chunks_exact(16).enumerate() {
        let tag = u64_at(pair, 0)?;
        let value = u64_at(pair, 8)?;
        if tag == 0 {
            if value != 0 || raw[(index + 1) * 16..].iter().any(|b| *b != 0) {
                return Err(invalid());
            }
            terminated = true;
            break;
        }
        if tag != 1 && !tags.insert(tag) {
            return Err(invalid());
        }
        match tag {
            1 => {
                if needed.len() >= 8 {
                    return Err(invalid());
                }
                needed.push(value);
            }
            5 => strings = Some(value),
            10 => string_size = Some(value),
            14 => soname = Some(value),
            30 if value & !0x18 == 0 => {}
            0x6fff_fffb if value & !0x0800_0001 == 0 => {}
            21 if value == 0 => {}
            // Known metadata is not relocation/symbol/version validation.
            2 | 3 | 4 | 6 | 7 | 8 | 9 | 11 | 12 | 13 | 20 | 23 | 25 | 26 | 27 | 28 | 35 | 36
            | 37 | 0x6fff_fef5 | 0x6fff_fff0 | 0x6fff_fff9 | 0x6fff_fffc | 0x6fff_fffd
            | 0x6fff_fffe | 0x6fff_ffff | 0x7000_0000 | 0x7000_0001 | 0x7000_0003 => {}
            _ => return Err(invalid()),
        }
    }
    if !terminated {
        return Err(invalid());
    }
    let string_size = string_size.ok_or_else(invalid)?;
    if string_size == 0 || string_size > 65536 {
        return Err(invalid());
    }
    let table = range(
        bytes,
        mapped(&loads, strings.ok_or_else(invalid)?, string_size)?,
        string_size,
    )?;
    if table.first() != Some(&0) || table.last() != Some(&0) {
        return Err(invalid());
    }
    let needed = needed
        .into_iter()
        .map(|index| name(table, index))
        .collect::<io::Result<Vec<_>>>()?;
    if needed.len() != needed.iter().collect::<BTreeSet<_>>().len() {
        return Err(invalid());
    }
    let soname = soname.map(|index| name(table, index)).transpose()?;
    Ok(Info {
        interpreter,
        needed,
        soname,
        kind,
        program_headers: count,
    })
}
