//! Value-only extension under the same original material owner; no FD export.
use super::*;

pub(crate) struct OriginalFdSet([i32; 20]);
impl OriginalFdSet {
    // Numeric values alone are not authority. The owner frames authenticate the
    // source inventory; only its private kernel provider supplies this value.
    pub(crate) fn from_material(values: [i32; 20]) -> io::Result<Self> {
        need(distinct(&values))?;
        Ok(Self(values))
    }
}
#[derive(PartialEq, Eq)]
pub(crate) struct BootstrapImageBindings {
    roles: [i32; 14],
    fds: [i32; 26],
    images: [Stamp; 3],
}
fn distinct(values: &[i32]) -> bool {
    values
        .iter()
        .enumerate()
        .all(|(i, v)| *v >= 0 && !values[..i].contains(v))
}
fn prefix(prior: &CompleteBindings, original: &OriginalFdSet) -> io::Result<[i32; 23]> {
    let mut fds = [-1; 23];
    fds[..20].copy_from_slice(&original.0);
    for (at, value) in prior.extra.iter().enumerate() {
        fds[20 + at] = value.fd;
    }
    need(distinct(&fds))?;
    let expected = [
        (0, 14),
        (1, 6),
        (2, 7),
        (3, 16),
        (5, 17),
        (6, 19),
        (9, 1),
        (10, 18),
    ];
    for (slot, at) in expected {
        need(prior.roles[slot] == fds[at])?;
    }
    for (slot, at) in [(4, 20), (7, 21), (8, 22)] {
        need(prior.roles[slot] == fds[at])?;
    }
    Ok(fds)
}
fn image(file: &File, expected: [u64; 3]) -> io::Result<Stamp> {
    let before = stamp(file)?;
    let all = libc::F_SEAL_SEAL | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_WRITE;
    need(expected.iter().all(|v| *v > 0) && expected[2] <= super::super::MAX_BINARY_BYTES)?;
    need(
        before.stat[4] as u32 & libc::S_IFMT == libc::S_IFREG
            && before.flags[1] & libc::O_ACCMODE == libc::O_RDONLY
            && [before.stat[0], before.stat[1], before.stat[6]] == expected
            && unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GET_SEALS) } == all,
    )?;
    need(stamp(file)? == before)?;
    Ok(before)
}
pub(crate) fn collect(
    prior: &CompleteBindings,
    original: OriginalFdSet,
    files: [&File; 3],
    expected: [[u64; 3]; 3],
) -> io::Result<BootstrapImageBindings> {
    let old = prefix(prior, &original)?;
    let images = [
        image(files[0], expected[0])?,
        image(files[1], expected[1])?,
        image(files[2], expected[2])?,
    ];
    let mut fds = [-1; 26];
    fds[..23].copy_from_slice(&old);
    for (at, value) in images.iter().enumerate() {
        fds[23 + at] = value.fd;
    }
    need(distinct(&fds))?;
    for (at, current) in images.iter().enumerate() {
        need(
            !images[..at]
                .iter()
                .any(|old| old.stat[..2] == current.stat[..2]),
        )?;
        need(current.stat[..2] != prior.extra[0].stat[..2])?;
    }
    let mut roles = [-1; 14];
    roles[..11].copy_from_slice(&prior.roles);
    roles[11..].copy_from_slice(&fds[23..]);
    // The old complete frames still authenticate the entire old 23. New image
    // reads are query-only, then checked again before the old closing frame.
    for (at, file) in files.iter().enumerate() {
        need(image(file, expected[at])? == images[at])?;
    }
    Ok(BootstrapImageBindings { roles, fds, images })
}

#[cfg(test)]
#[path = "diagnostic_bootstrap_image_census_tests.rs"]
pub(crate) mod tests;

#[path = "diagnostic_bootstrap_entry_census.rs"]
pub(crate) mod entry;
