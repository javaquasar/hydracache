//! Two document source views only. Lexical selection is not path authorization.
use std::{io, path::Path};
#[path = "read.rs"]
mod read;
pub use read::security_selected;
pub use read::selected;
#[path = "../source-successor-view-074/contract.rs"]
pub mod successor;
pub fn project(root: &Path, path: &Path, raw: String) -> String {
    match selected(root, path) {
        Some(relative) => successor::project(relative, raw),
        None => raw,
    }
}
pub fn read_to_string(path: impl AsRef<Path>) -> io::Result<String> {
    let path = path.as_ref();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    read_with(&root, path, |path| std::fs::read_to_string(path))
}
pub fn read_with(
    root: &Path,
    path: &Path,
    observe: impl FnOnce(&Path) -> io::Result<String>,
) -> io::Result<String> {
    read::read_with(
        root,
        path,
        observe,
        |root, path, raw| match security_selected(root, path) {
            Some(relative) => successor::project(relative, raw),
            None => project(root, path, raw),
        },
    )
}
