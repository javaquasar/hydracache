//! Isolated census compatibility candidate, NOT a batched or installed caller.
//! Each reached read inherits Git's environment/configuration at spawn, exactly
//! like the old caller. No env filtering, replacement-policy switch or retry.
//! The shared diagnostic deadline/budgets are additional refusal boundaries;
//! this is not equivalence to the old unbounded loader for arbitrary execution.
use super::{decoder, transport};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

#[derive(Debug, PartialEq, Eq)]
pub enum Reason {
    Reference,
    Transport(transport::Refusal),
    Process,
    Utf8,
    Content,
}

/// Keep successful diagnostics and failed command output, not only a boolean.
#[derive(Debug)]
pub struct Failure {
    pub reason: Reason,
    pub output: Option<transport::Output>,
    pub original_io: Option<std::io::Error>,
}
impl Failure {
    fn before(reason: Reason) -> Self {
        Self {
            reason,
            output: None,
            original_io: None,
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
pub struct Loaded {
    pub text: String,
    pub output: transport::Output,
}

/// Bound an independent raw blob reference to the literal query before effects.
/// No observed body can update this reference. Construction grants no authority.
#[derive(Debug)]
pub struct Reference {
    spec: String,
    bytes: usize,
    sha256: [u8; 32],
}
impl Reference {
    pub fn new(spec: &str, pin: &decoder::Expected) -> Result<Self, Failure> {
        let valid = spec.split_once(':').is_some_and(|(revision, path)| {
            revision.len() == 40
                && revision
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                && !path.is_empty()
                && path.len() <= 4096
                && !path
                    .bytes()
                    .any(|b| matches!(b, b'\0' | b'\r' | b'\n' | b'\\'))
                && path.split('/').all(|part| !matches!(part, "" | "." | ".."))
        });
        if !valid || decoder::validate(std::slice::from_ref(pin)).is_err() {
            return Err(Failure::before(Reason::Reference));
        }
        let decoder::Expected::Object {
            kind: decoder::Kind::Blob,
            bytes,
            sha256,
            ..
        } = pin
        else {
            return Err(Failure::before(Reason::Reference));
        };
        Ok(Self {
            spec: spec.to_owned(),
            bytes: *bytes,
            sha256: *sha256,
        })
    }
}

/// Old interpretation order: status, UTF8, CRLF normalization. Stderr is retained
/// but is not a census failure when status succeeds. No lossy UTF8 conversion.
pub fn interpret(output: transport::Output) -> Result<Loaded, Failure> {
    let reason = if output.code != Some(0) {
        Some(Reason::Process)
    } else if std::str::from_utf8(&output.stdout).is_err() {
        Some(Reason::Utf8)
    } else {
        None
    };
    if let Some(reason) = reason {
        return Err(Failure {
            reason,
            output: Some(output),
            original_io: None,
        });
    }
    let text = std::str::from_utf8(&output.stdout)
        .expect("checked UTF8")
        .replace("\r\n", "\n");
    Ok(Loaded { text, output })
}

/// One exact original `show` at its conditional observation point. Does not use
/// batch transport, sanitize stderr, prefetch a second read, or fall back on error.
pub fn read(
    root: &Path,
    spec: &str,
    reference: &Reference,
    deadline: Instant,
) -> Result<Loaded, Failure> {
    if Instant::now() >= deadline {
        return Err(Failure::before(Reason::Transport(
            transport::Refusal::Deadline,
        )));
    }
    if spec != reference.spec {
        return Err(Failure::before(Reason::Reference));
    }
    let mut command = Command::new("git");
    command
        .arg("--no-replace-objects")
        .args(["show", spec])
        .current_dir(root);
    let output =
        transport::run_owned_with_io(&mut command, &[], deadline).map_err(|failure| Failure {
            reason: Reason::Transport(failure.refusal),
            output: None,
            original_io: failure.original_io,
        })?;
    let loaded = interpret(output)?;
    let reason = if loaded.output.stdout.len() != reference.bytes
        || <[u8; 32]>::from(Sha256::digest(&loaded.output.stdout)) != reference.sha256
    {
        Some(Reason::Content)
    } else if Instant::now() >= deadline {
        Some(Reason::Transport(transport::Refusal::Deadline))
    } else {
        None
    };
    if let Some(reason) = reason {
        return Err(Failure {
            reason,
            output: Some(loaded.output),
            original_io: None,
        });
    }
    Ok(loaded)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Revision {
    Base,
    Reviewed,
}

/// Inputs stay lazy. Neither pins nor this trait admit current helper/body bytes.
pub trait Inputs {
    type Error;
    fn helper(&mut self) -> Result<String, Self::Error>;
    fn helper_matches(&mut self, helper: &str) -> bool;
    fn frozen(&mut self, revision: Revision) -> Result<String, Self::Error>;
    fn body_matches(&mut self, original: &str, raw: &str) -> bool;
}
#[derive(Debug, PartialEq, Eq)]
pub enum ViewError<E> {
    Input(E),
    Helper,
    Provider,
}

pub fn accepts<I: Inputs>(inputs: &mut I, original: &str, raw: &str) -> Result<bool, I::Error> {
    Ok(
        raw.replace("\r\n", "\n") == inputs.frozen(Revision::Reviewed)?
            && inputs.body_matches(original, raw),
    )
}

/// Mirror only the unchanged census caller's schedule, not its panic API.
pub fn view<I: Inputs>(
    inputs: &mut I,
    name: &str,
    raw: &str,
) -> Result<String, ViewError<I::Error>> {
    if name != "diagnostic_artifacts_linux.rs" {
        return Ok(raw.to_owned());
    }
    let helper = inputs.helper().map_err(ViewError::Input)?;
    if !inputs.helper_matches(&helper) {
        return Err(ViewError::Helper);
    }
    let original = inputs.frozen(Revision::Base).map_err(ViewError::Input)?;
    if !accepts(inputs, &original, raw).map_err(ViewError::Input)? {
        return Err(ViewError::Provider);
    }
    Ok(original)
}

/// Errors retain their original phase until the old panic boundary is reached.
#[derive(Debug)]
pub enum InputError {
    Helper(std::io::Error),
    Frozen(Failure),
}

/// Compatibility facade only. New pin/budget/deadline/cleanup refusals must stay
/// explicit, never be mislabeled as a legacy process failure or retried.
pub mod panic_api {
    use super::transport::Refusal;
    use super::{Failure, InputError, Inputs, Loaded, Reason, Revision, ViewError};

    // Keep std's actual unwrap payload instead of rebuilding its text. The same
    // boundary handles both original IO errors and FromUtf8Error values.
    fn legacy_unwrap<T, E: std::fmt::Debug>(value: Result<T, E>) -> T {
        value.unwrap()
    }

    fn unwrap_input<T>(value: Result<T, InputError>) -> T {
        match value {
            Ok(value) => value,
            Err(InputError::Helper(error)) => legacy_unwrap(Err(error)),
            Err(InputError::Frozen(failure)) => frozen_failure(failure),
        }
    }

    fn frozen_failure(failure: Failure) -> ! {
        match failure {
            Failure {
                reason: Reason::Process,
                ..
            } => panic!("missing exact material fixture source"),
            Failure {
                reason: Reason::Utf8,
                output: Some(output),
                ..
            } => {
                // Use the same FromUtf8Error Debug representation as the old unwrap.
                legacy_unwrap(String::from_utf8(output.stdout));
                unreachable!("Utf8 failure must contain invalid bytes")
            }
            Failure {
                reason: Reason::Transport(Refusal::Io(_)),
                original_io: Some(error),
                ..
            } => {
                legacy_unwrap::<(), _>(Err(error));
                unreachable!("IO failure cannot succeed")
            }
            failure => panic!("census adapter refusal: {failure:?}"),
        }
    }

    pub fn frozen(value: Result<Loaded, Failure>) -> String {
        match value {
            Ok(value) => value.text,
            Err(failure) => frozen_failure(failure),
        }
    }
    pub fn original<I: Inputs<Error = InputError>>(inputs: &mut I) -> String {
        unwrap_input(inputs.frozen(Revision::Base))
    }
    pub fn accepts<I: Inputs<Error = InputError>>(
        inputs: &mut I,
        original: &str,
        raw: &str,
    ) -> bool {
        unwrap_input(super::accepts(inputs, original, raw))
    }
    pub fn view<I: Inputs<Error = InputError>>(inputs: &mut I, name: &str, raw: &str) -> String {
        match super::view(inputs, name, raw) {
            Ok(value) => value,
            Err(ViewError::Input(error)) => unwrap_input(Err(error)),
            Err(ViewError::Helper) => panic!("unreviewed fixture control"),
            Err(ViewError::Provider) => panic!("unreviewed provider body"),
        }
    }
}
