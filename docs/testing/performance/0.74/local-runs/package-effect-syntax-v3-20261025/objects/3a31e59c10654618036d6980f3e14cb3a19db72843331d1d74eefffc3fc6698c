//! Explicit historical test IO. Never a current-source admission API.
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Component, Path, PathBuf};
use std::process::Output;

pub const SOURCE: &str = "33c54b0cda8c330b115009cdf092ad002077cedf";

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "unsupported historical guard IO",
    )
}
pub fn root() -> PathBuf {
    lexical(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}
fn lexical(path: &Path) -> io::Result<PathBuf> {
    if !path.is_absolute() {
        return Err(invalid());
    }
    let mut result = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir => {
                if !result.pop() {
                    return Err(invalid());
                }
            }
            Component::CurDir => (),
            other => result.push(other.as_os_str()),
        }
    }
    Ok(result)
}
pub fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 1024
        && path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_./-".contains(&byte))
        && path.split('/').all(|part| {
            let upper = part
                .split('.')
                .next()
                .unwrap_or_default()
                .to_ascii_uppercase();
            !part.is_empty()
                && part != "."
                && part != ".."
                && !part.ends_with('.')
                && !matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                && !(upper.len() == 4
                    && (upper.starts_with("COM") || upper.starts_with("LPT"))
                    && upper.as_bytes()[3].is_ascii_digit())
        })
}
fn revision(value: &str) -> bool {
    (4..=40).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
pub fn translated(args: &[String]) -> io::Result<Vec<String>> {
    let words: Vec<_> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["show", object] => {
            let (rev, path) = object.split_once(':').ok_or_else(invalid)?;
            if !revision(rev) || !safe_path(path) {
                return Err(invalid());
            }
        }
        ["ls-tree", "-r", "--name-only", rev, "--", paths @ ..] => {
            if !revision(rev) || paths.is_empty() || !paths.iter().all(|path| safe_path(path)) {
                return Err(invalid());
            }
        }
        ["diff", flag, rev, "--", paths @ ..] => {
            if !matches!(*flag, "--exit-code" | "--name-only")
                || !revision(rev)
                || paths.is_empty()
                || !paths.iter().all(|path| safe_path(path))
            {
                return Err(invalid());
            }
            let mut output = args.to_vec();
            output.insert(3, SOURCE.into());
            return Ok(output);
        }
        _ => return Err(invalid()),
    }
    Ok(args.to_vec())
}
pub fn frozen_bytes(path: &str) -> io::Result<Vec<u8>> {
    if !safe_path(path) {
        return Err(invalid());
    }
    let output = std::process::Command::new("git")
        .arg("--no-replace-objects")
        .args(["show", &format!("{SOURCE}:{path}")])
        .current_dir(root())
        .output()?;
    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "missing frozen guard input",
        ));
    }
    if output.stdout.len() > 16 * 1024 * 1024 {
        return Err(invalid());
    }
    Ok(output.stdout)
}
pub fn read_to_string(path: impl AsRef<Path>) -> io::Result<String> {
    let absolute = lexical(path.as_ref())?;
    let root = root();
    let relative = absolute.strip_prefix(&root).map_err(|_| invalid())?;
    let relative = relative.to_str().ok_or_else(invalid)?.replace('\\', "/");
    String::from_utf8(frozen_bytes(&relative)?)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "frozen source is not UTF-8"))
}

pub struct HistoricalCommand {
    args: Vec<OsString>,
    cwd: Option<PathBuf>,
}
pub fn command() -> HistoricalCommand {
    HistoricalCommand {
        args: Vec::new(),
        cwd: None,
    }
}
impl HistoricalCommand {
    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args
            .extend(args.into_iter().map(|value| value.as_ref().to_owned()));
        self
    }
    // Some historical binaries relied on the package cwd instead of supplying it.
    #[allow(dead_code)]
    pub fn current_dir(&mut self, path: impl AsRef<Path>) -> &mut Self {
        self.cwd = Some(path.as_ref().to_owned());
        self
    }
    pub fn output(&mut self) -> io::Result<Output> {
        let root = root();
        if let Some(cwd) = &self.cwd {
            if lexical(cwd)? != root {
                return Err(invalid());
            }
        }
        let args = self
            .args
            .iter()
            .map(|value| value.to_str().map(str::to_owned).ok_or_else(invalid))
            .collect::<io::Result<Vec<_>>>()?;
        let args = translated(&args)?;
        std::process::Command::new("git")
            .arg("--no-replace-objects")
            .args(args)
            .current_dir(root)
            .output()
    }
}
