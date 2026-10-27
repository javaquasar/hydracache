//! Reuse the installed caller's existing projector, not a second module instance.
use std::{io, path::Path};
#[path = "read.rs"]
mod read;
pub fn read_to_string(path: impl AsRef<Path>) -> io::Result<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    read::read_with(
        &root,
        path.as_ref(),
        |path| std::fs::read_to_string(path),
        |root, path, raw| match read::security_selected(root, path)
            .or_else(|| read::selected(root, path))
        {
            Some(relative) => crate::runtime::successor::project(relative, raw),
            None => raw,
        },
    )
}
