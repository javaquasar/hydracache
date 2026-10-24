//! Isolated native experiment. Only test targets import this module.
use super::decoder::{self, Expected, Kind, Record};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub const BOUND: Duration = Duration::from_secs(30);
const CLEANUP: Duration = Duration::from_secs(5);
const INPUT: usize = 256 * 1024;
const STDERR: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Replacements {
    Default,
    Disabled,
}
#[derive(Debug, PartialEq, Eq)]
pub struct Output {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: Option<i32>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    Input,
    Environment,
    Deadline,
    Budget,
    Process,
    FallbackRequired,
    FallbackIdentity,
    FallbackOutput,
    Protocol(decoder::Refusal),
    Io(String),
    Cleanup {
        primary: Option<Box<Refusal>>,
        reason: String,
    },
}
impl From<std::io::Error> for Refusal {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

#[derive(Clone, Debug)]
struct BytePin {
    bytes: usize,
    sha256: [u8; 32],
}
impl BytePin {
    fn from_reference(raw: &[u8]) -> Self {
        Self {
            bytes: raw.len(),
            sha256: Sha256::digest(raw).into(),
        }
    }
    fn matches(&self, raw: &[u8]) -> bool {
        self.bytes == raw.len() && self.sha256 == <[u8; 32]>::from(Sha256::digest(raw))
    }
}

/// Independently prepared output reference, NOT a source authority/enrollment.
/// Bind the raw query, original decoder pin and policy, not a mutable ref digest.
#[derive(Clone, Debug)]
pub struct FallbackPin {
    spec: String,
    original: Expected,
    policy: Replacements,
    stdout: BytePin,
    stderr: BytePin,
    code: i32,
}
impl FallbackPin {
    pub fn from_reference(
        spec: &str,
        original: &Expected,
        policy: Replacements,
        reference: &Output,
    ) -> Result<Self, Refusal> {
        decoder::validate(std::slice::from_ref(original)).map_err(Refusal::Protocol)?;
        if !valid_spec(spec)
            || matches!(
                original,
                Expected::Object {
                    kind: Kind::Blob,
                    ..
                }
            )
            || matches!(original, Expected::Missing { spec: other } | Expected::ShortMissing { spec: other } if other != spec)
            || !matches!(reference.code, Some(0..=255))
            || (matches!(
                original,
                Expected::Missing { .. } | Expected::ShortMissing { .. }
            ) && reference.code == Some(0))
        {
            return Err(Refusal::FallbackIdentity);
        }
        if reference.stdout.len() > decoder::MAX_STREAM || reference.stderr.len() > STDERR {
            return Err(Refusal::Budget);
        }
        Ok(Self {
            spec: spec.to_owned(),
            original: original.clone(),
            policy,
            stdout: BytePin::from_reference(&reference.stdout),
            stderr: BytePin::from_reference(&reference.stderr),
            code: reference.code.expect("validated terminal code"),
        })
    }
    pub fn verify_output(&self, value: &Output, deadline: Instant) -> Result<(), Refusal> {
        timely(deadline)?;
        if value.code != Some(self.code)
            || !self.stdout.matches(&value.stdout)
            || !self.stderr.matches(&value.stderr)
        {
            return Err(Refusal::FallbackOutput);
        }
        timely(deadline)
    }
}

fn validate_fallbacks(
    specs: &[String],
    pins: &[Expected],
    policy: Replacements,
    fallbacks: Option<&[Option<FallbackPin>]>,
) -> Result<(), Refusal> {
    if fallbacks.is_some_and(|v| v.len() != specs.len()) {
        return Err(Refusal::FallbackIdentity);
    }
    let (mut stdout, mut stderr) = (0usize, 0usize);
    for (index, (spec, original)) in specs.iter().zip(pins).enumerate() {
        let fallback = fallbacks.and_then(|v| v[index].as_ref());
        if let Expected::Object {
            kind: Kind::Blob, ..
        } = original
        {
            if fallback.is_some() {
                return Err(Refusal::FallbackIdentity);
            }
            continue;
        }
        let fallback = fallback.ok_or(Refusal::FallbackRequired)?;
        if fallback.spec != *spec || fallback.original != *original || fallback.policy != policy {
            return Err(Refusal::FallbackIdentity);
        }
        stdout = stdout
            .checked_add(fallback.stdout.bytes)
            .ok_or(Refusal::Budget)?;
        stderr = stderr
            .checked_add(fallback.stderr.bytes)
            .ok_or(Refusal::Budget)?;
        if stdout > decoder::MAX_STREAM || stderr > STDERR {
            return Err(Refusal::Budget);
        }
    }
    Ok(())
}

struct Owned {
    child: Child,
    cleanup_attempted: bool,
}
impl Owned {
    fn stop(&mut self) -> std::io::Result<()> {
        self.cleanup_attempted = true;
        if self.child.try_wait()?.is_none() {
            if let Err(error) = self.child.kill() {
                if self.child.try_wait()?.is_none() {
                    return Err(error);
                }
            }
            let until = Instant::now() + CLEANUP;
            while self.child.try_wait()?.is_none() {
                if Instant::now() >= until {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "owned child cleanup",
                    ));
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        Ok(())
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        if !self.cleanup_attempted {
            let _ = self.stop();
        }
    }
}
fn timely(deadline: Instant) -> Result<(), Refusal> {
    if Instant::now() >= deadline {
        Err(Refusal::Deadline)
    } else {
        Ok(())
    }
}
fn bytes(file: &mut File, limit: usize) -> Result<Vec<u8>, Refusal> {
    file.seek(SeekFrom::Start(0))?;
    let mut raw = Vec::new();
    file.take((limit + 1) as u64).read_to_end(&mut raw)?;
    if raw.len() > limit {
        return Err(Refusal::Budget);
    }
    Ok(raw)
}

/// No pipes or image-name cleanup. The command is a private test-owned builder.
/// File-size sampling is a refusal guard, not a hard disk quota or tree fence.
pub fn run_owned(
    command: &mut Command,
    input: &[u8],
    deadline: Instant,
) -> Result<Output, Refusal> {
    timely(deadline)?;
    if input.len() > INPUT {
        return Err(Refusal::Input);
    }
    let mut source = tempfile::tempfile()?;
    source.write_all(input)?;
    source.seek(SeekFrom::Start(0))?;
    let mut stdout = tempfile::tempfile()?;
    let mut stderr = tempfile::tempfile()?;
    timely(deadline)?;
    let child = command
        .stdin(Stdio::from(source))
        .stdout(stdout.try_clone()?)
        .stderr(stderr.try_clone()?)
        .spawn()?;
    let mut owned = Owned {
        child,
        cleanup_attempted: false,
    };
    let result = (|| loop {
        timely(deadline)?;
        if stdout.metadata()?.len() > decoder::MAX_STREAM as u64
            || stderr.metadata()?.len() > STDERR as u64
        {
            return Err(Refusal::Budget);
        }
        if let Some(status) = owned.child.try_wait()? {
            let result = Output {
                stdout: bytes(&mut stdout, decoder::MAX_STREAM)?,
                stderr: bytes(&mut stderr, STDERR)?,
                code: status.code(),
            };
            timely(deadline)?;
            return Ok(result);
        }
        std::thread::sleep(Duration::from_millis(5));
    })();
    if let Err(error) = owned.stop() {
        return Err(Refusal::Cleanup {
            primary: result.err().map(Box::new),
            reason: error.to_string(),
        });
    }
    result
}

pub fn git(
    root: &Path,
    policy: Replacements,
    args: &[&str],
    input: &[u8],
    deadline: Instant,
) -> Result<Output, Refusal> {
    if [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_NAMESPACE",
    ]
    .iter()
    .any(|name| std::env::var_os(name).is_some())
        || std::env::vars_os().any(|(name, _)| {
            name.to_string_lossy()
                .to_ascii_uppercase()
                .starts_with("GIT_CONFIG")
        })
    {
        return Err(Refusal::Environment);
    }
    let mut command = Command::new("git");
    if matches!(policy, Replacements::Disabled) {
        command.arg("--no-replace-objects");
    }
    // Explicit synthetic-fixture boundary; not arbitrary existing-loader config.
    command
        .args(args)
        .current_dir(root)
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_ATTR_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        );
    run_owned(&mut command, input, deadline)
}

fn valid_spec(spec: &str) -> bool {
    spec.split_once(':').is_some_and(|(id, path)| {
        (4..=40).contains(&id.len())
            && id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            && !path.is_empty()
            && path.len() <= 4096
            && !path
                .bytes()
                .any(|b| matches!(b, b'\0' | b'\r' | b'\n' | b'\\'))
            && path.split('/').all(|p| !matches!(p, "" | "." | ".."))
    })
}

/// Pins must come from a separate reference. This function does not admit them.
/// Both policy and raw short/full query syntax remain explicit and unchanged.
pub fn read(
    root: &Path,
    specs: &[String],
    pins: &[Expected],
    policy: Replacements,
    deadline: Instant,
) -> Result<Vec<Output>, Refusal> {
    read_observed(root, specs, pins, policy, deadline, |_| {})
}

/// Logical cuts for deterministic mutation controls in this isolated experiment.
/// Observer work uses the original deadline; it is not a validation authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Point {
    BatchValidated,
    BeforeFallback,
    AfterFallback,
}

pub fn read_observed(
    root: &Path,
    specs: &[String],
    pins: &[Expected],
    policy: Replacements,
    deadline: Instant,
    observe: impl FnMut(Point),
) -> Result<Vec<Output>, Refusal> {
    read_inner(root, specs, pins, policy, deadline, observe, None)
}

pub fn read_pinned(
    root: &Path,
    specs: &[String],
    pins: &[Expected],
    fallbacks: &[Option<FallbackPin>],
    policy: Replacements,
    deadline: Instant,
) -> Result<Vec<Output>, Refusal> {
    read_pinned_observed(root, specs, pins, fallbacks, policy, deadline, |_| {})
}

pub fn read_pinned_observed(
    root: &Path,
    specs: &[String],
    pins: &[Expected],
    fallbacks: &[Option<FallbackPin>],
    policy: Replacements,
    deadline: Instant,
    observe: impl FnMut(Point),
) -> Result<Vec<Output>, Refusal> {
    read_inner(
        root,
        specs,
        pins,
        policy,
        deadline,
        observe,
        Some(fallbacks),
    )
}

fn read_inner(
    root: &Path,
    specs: &[String],
    pins: &[Expected],
    policy: Replacements,
    deadline: Instant,
    mut observe: impl FnMut(Point),
    fallbacks: Option<&[Option<FallbackPin>]>,
) -> Result<Vec<Output>, Refusal> {
    timely(deadline)?;
    if specs.is_empty()
        || specs.len() > 256
        || specs.len() != pins.len()
        || !specs.iter().all(|s| valid_spec(s))
    {
        return Err(Refusal::Input);
    }
    for (spec, pin) in specs.iter().zip(pins) {
        if let Expected::Missing { spec: expected } | Expected::ShortMissing { spec: expected } =
            pin
        {
            if expected != spec {
                return Err(Refusal::Input);
            }
        }
    }
    decoder::validate(pins).map_err(Refusal::Protocol)?;
    validate_fallbacks(specs, pins, policy, fallbacks)?;
    let input = format!("{}\n", specs.join("\n"));
    if input.len() > INPUT {
        return Err(Refusal::Input);
    }
    let batch = git(
        root,
        policy,
        &["cat-file", "--batch"],
        input.as_bytes(),
        deadline,
    )?;
    if batch.code != Some(0) || !batch.stderr.is_empty() {
        return Err(Refusal::Process);
    }
    let records = decoder::decode(&batch.stdout, pins).map_err(Refusal::Protocol)?;
    observe(Point::BatchValidated);
    let mut outputs = Vec::with_capacity(specs.len());
    let (mut out_bytes, mut err_bytes) = (0, 0);
    for (index, (spec, record)) in specs.iter().zip(records).enumerate() {
        timely(deadline)?;
        let output = match record {
            Record::Object {
                kind: Kind::Blob,
                bytes,
            } => Output {
                stdout: bytes.to_vec(),
                stderr: vec![],
                code: Some(0),
            },
            _ => {
                observe(Point::BeforeFallback);
                let result = git(root, policy, &["show", spec], &[], deadline)?;
                observe(Point::AfterFallback);
                fallbacks
                    .and_then(|v| v[index].as_ref())
                    .ok_or(Refusal::FallbackRequired)?
                    .verify_output(&result, deadline)?;
                result
            }
        };
        out_bytes += output.stdout.len();
        err_bytes += output.stderr.len();
        if out_bytes > decoder::MAX_STREAM || err_bytes > STDERR {
            return Err(Refusal::Budget);
        }
        outputs.push(output);
    }
    timely(deadline)?;
    Ok(outputs)
}
