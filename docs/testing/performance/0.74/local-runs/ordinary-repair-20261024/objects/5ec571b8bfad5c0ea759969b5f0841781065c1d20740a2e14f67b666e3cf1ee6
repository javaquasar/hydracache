//! Test-only pure adapter to the private production storage and complete body.
use super::{document_buffer::Scratch, WorkerContextError};

pub(crate) fn with_buffer<T>(length: usize, call: impl FnOnce(&mut [u8]) -> T) -> T {
    let mut buffer = Scratch::new(length);
    call(&mut buffer)
}

pub(crate) fn read_model(
    expected: &str,
    length: usize,
    read: impl FnOnce(&mut [u8]) -> std::io::Result<usize>,
    mut metadata: impl FnMut() -> Result<bool, WorkerContextError>,
) -> Result<(), WorkerContextError> {
    if !metadata()? {
        return Err(WorkerContextError::Drift);
    }
    // One bounded positional read. Short/EINTR reads refuse; no retry or IO deadline claim.
    let mut buffer = Scratch::new(length);
    let count = read(&mut buffer)?;
    if !metadata()? {
        return Err(WorkerContextError::Drift);
    }
    if count != length
        || expected.len() + 1 != length
        || buffer[count - 1] != b'\n'
        || &buffer[..count - 1] != expected.as_bytes()
    {
        return Err(WorkerContextError::Drift);
    }
    Ok(())
}
