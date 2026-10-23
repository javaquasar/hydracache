//! Current bytes are verified before an old preservation phase receives its predecessor.
use std::{io, path::Path};
#[path = "../historical-input-view-074/contract.rs"]
pub(crate) mod reviewed_inputs;
pub(crate) fn read_to_string(path: impl AsRef<Path>) -> io::Result<String> {
    let path = path.as_ref();
    let raw = std::fs::read_to_string(path)?.replace("\r\n", "\n");
    let root = super::runtime::root().canonicalize()?;
    let absolute = path.canonicalize()?;
    let relative = absolute
        .strip_prefix(&root)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "outside exact runtime view"))?;
    let relative = relative
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "non-UTF8 runtime path"))?
        .replace('\\', "/");
    if reviewed_inputs::PATHS.contains(&relative.as_str()) {
        return Ok(reviewed_inputs::predecessor(&relative, &raw));
    }
    Ok(super::runtime::view(&relative, raw))
}
