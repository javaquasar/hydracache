//! One observation shared by standalone and already-imported projector callers.
use std::{
    io,
    path::{Component, Path, PathBuf},
};
fn lexical(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => (),
            Component::ParentDir => {
                if !result.pop() {
                    return None;
                }
            }
            other => result.push(other.as_os_str()),
        }
    }
    Some(result)
}
pub fn selected(root: &Path, path: &Path) -> Option<&'static str> {
    let root = lexical(root)?;
    let path = lexical(path)?;
    let relative = path.strip_prefix(root).ok()?;
    if relative == Path::new("docs/COMPAT.md") {
        Some("docs/COMPAT.md")
    } else if relative == Path::new("docs/testing/gated-test-registry.toml") {
        Some("docs/testing/gated-test-registry.toml")
    } else {
        None
    }
}
pub fn read_with(
    root: &Path,
    path: &Path,
    observe: impl FnOnce(&Path) -> io::Result<String>,
    project: impl FnOnce(&Path, &Path, String) -> String,
) -> io::Result<String> {
    let raw = observe(path)?;
    Ok(project(root, path, raw))
}
