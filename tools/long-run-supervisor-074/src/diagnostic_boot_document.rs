//! One-read temporary storage only; no origin, clock, or admission state.
pub(super) fn read_once(
    read: impl FnOnce(&mut [u8]) -> std::io::Result<usize>,
) -> std::io::Result<([u8; 38], usize)> {
    // Canonical UUID plus newline and one byte detecting an overlength read.
    // No retry: a short read or EINTR must remain visible to the caller.
    let mut bytes = [0; 38];
    let count = read(&mut bytes)?;
    Ok((bytes, count))
}
