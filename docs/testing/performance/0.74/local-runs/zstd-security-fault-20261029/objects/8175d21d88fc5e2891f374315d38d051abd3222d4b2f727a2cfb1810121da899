/// Register the actual native Unix backend for bounded transport fixtures.
///
/// Uses the same runtime selection, registration, peer credentials and socket
/// operations as the Unix-stream connection builder. Enabled only explicitly.
#[cfg(all(unix, feature = "hc074-unix-fixture"))]
pub fn hc074_unix_fixture(stream: std::os::unix::net::UnixStream) -> crate::Result<BoxedSplit> {
    let runtime = crate::runtime::Runtime::default_for_build()?;
    Ok(crate::runtime::io::registered(&runtime, stream, crate::runtime::io::UnixOps)?.into())
}
