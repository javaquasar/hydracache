//! An unsent FD-bearing message, never a connection or start capability.
#[path = "diagnostic_original_grouped_preparation.rs"]
pub mod grouped_preparation;
#[cfg(test)]
#[path = "diagnostic_original_safety_alarm.rs"]
pub(crate) mod safety_alarm;
use super::*;
use crate::systemd_unit::{TransientUnitSpec, UnitProperty};
use std::fs::File;
use std::os::fd::AsRawFd;
use zbus::message::{Message, Type};
use zbus::zvariant::{Fd, Value};

const DESTINATION: &str = "org.freedesktop.systemd1";
const PATH: &str = "/org/freedesktop/systemd1";
const INTERFACE: &str = "org.freedesktop.systemd1.Manager";
const METHOD: &str = "StartTransientUnit";

#[derive(Debug, thiserror::Error)]
pub enum OriginalPipeTransportError {
    #[error("original material refused: {0}")]
    Material(#[from] OriginalStartMaterialError),
    #[error("original signed collector settings or pipe message differs")]
    Binding,
    #[error("pipe message preparation already refused")]
    Refused,
    #[error("pipe message encoding failed: {0}")]
    Encoding(#[from] zbus::Error),
    #[error("pipe message descriptor inspection failed: {0}")]
    Io(#[from] std::io::Error),
}

fn value(property: &UnitProperty) -> Value<'_> {
    match property {
        UnitProperty::Text(v) => Value::from(v),
        UnitProperty::Boolean(v) => Value::from(*v),
        UnitProperty::Unsigned(v) => Value::from(*v),
        UnitProperty::Bytes(v) => Value::from(v),
        UnitProperty::Strings(v) => Value::from(v),
        UnitProperty::Commands(v) => Value::from(
            v.iter()
                .map(|c| {
                    (
                        c.path.as_str(),
                        c.argv.iter().map(String::as_str).collect::<Vec<_>>(),
                        c.ignore_failure,
                    )
                })
                .collect::<Vec<_>>(),
        ),
    }
}

// This explicit reconstruction is checked against *every* signed typed property.
// No JSON property name becomes an arbitrary systemd property or caller input.
fn worker(
    reserved: &DiagnosticState,
    projection: &[u8],
) -> Result<TransientUnitSpec, OriginalPipeTransportError> {
    if collector_unit_projection(reserved).map_err(|_| OriginalPipeTransportError::Binding)?
        != projection
    {
        return Err(OriginalPipeTransportError::Binding);
    }
    let mut spec = crate::diagnostic_unit::build_diagnostic_unit_spec(reserved)
        .map_err(|_| OriginalPipeTransportError::Binding)?;
    spec.properties.retain(|(name, _)| {
        !matches!(
            *name,
            "StandardOutputFileToAppend" | "StandardErrorFileToAppend"
        )
    });
    for (name, property) in &mut spec.properties {
        match *name {
            "Description" => {
                *property = UnitProperty::Text(
                    "HydraCache collector diagnostic requirements; not qualification".into(),
                )
            }
            "WorkingDirectory" => *property = UnitProperty::Text("/".into()),
            "ReadWritePaths" => *property = UnitProperty::Strings(Vec::new()),
            "LimitFSIZE" => *property = UnitProperty::Unsigned(0),
            _ => (),
        }
    }
    check_settings(&spec, projection)?;
    Ok(spec)
}

fn check_settings(
    spec: &TransientUnitSpec,
    projection: &[u8],
) -> Result<(), OriginalPipeTransportError> {
    let signed: serde_json::Value =
        serde_json::from_slice(projection).map_err(|_| OriginalPipeTransportError::Binding)?;
    let properties = signed["worker"]["properties"]
        .as_array()
        .ok_or(OriginalPipeTransportError::Binding)?;
    if signed["worker"]["unit_name"] != spec.unit_name || properties.len() != spec.properties.len()
    {
        return Err(OriginalPipeTransportError::Binding);
    }
    for ((name, property), signed) in spec.properties.iter().zip(properties) {
        use serde_json::json;
        let (tag, v) = match property {
            UnitProperty::Text(v) => ("text", json!(v)),
            UnitProperty::Boolean(v) => ("boolean", json!(v)),
            UnitProperty::Unsigned(v) => ("unsigned", json!(v)),
            UnitProperty::Bytes(v) => ("bytes", json!(v)),
            UnitProperty::Strings(v) => ("strings", json!(v)),
            UnitProperty::Commands(v) => (
                "commands",
                json!(v
                    .iter()
                    .map(|c| json!({
                        "path": c.path, "argv": c.argv, "ignore_failure": c.ignore_failure,
                    }))
                    .collect::<Vec<_>>()),
            ),
        };
        if *signed != json!([name, {"type": tag, "value": v}]) {
            return Err(OriginalPipeTransportError::Binding);
        }
    }
    Ok(())
}

fn message(
    spec: &TransientUnitSpec,
    stdout: &File,
    stderr: &File,
) -> Result<Message, OriginalPipeTransportError> {
    let mut properties: Vec<_> = spec
        .properties
        .iter()
        .map(|(name, p)| (*name, value(p)))
        .collect();
    properties.push((
        "StandardOutputFileDescriptor",
        Value::from(Fd::from(stdout)),
    ));
    properties.push(("StandardErrorFileDescriptor", Value::from(Fd::from(stderr))));
    let auxiliary: Vec<(&str, Vec<(&str, Value<'_>)>)> = Vec::new();
    Ok(Message::method_call(PATH, METHOD)?
        .destination(DESTINATION)?
        .interface(INTERFACE)?
        .build(&(spec.unit_name.as_str(), "fail", properties, auxiliary))?)
}

fn descriptor(value: &Value<'_>, original: &File) -> Result<(), OriginalPipeTransportError> {
    let Value::Fd(fd) = value else {
        return Err(OriginalPipeTransportError::Binding);
    };
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    // The message owns this FD throughout the bounded fstat/fcntl inspection.
    if unsafe { libc::fstat(fd.as_raw_fd(), &mut stat) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    use std::os::unix::fs::MetadataExt;
    let m = original.metadata()?;
    if stat.st_dev != m.dev()
        || stat.st_ino != m.ino()
        || stat.st_uid != m.uid()
        || stat.st_gid != m.gid()
        || stat.st_mode != m.mode()
        || stat.st_nlink != m.nlink()
        || unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFL) } != libc::O_WRONLY
        || unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFD) } != libc::FD_CLOEXEC
    {
        return Err(OriginalPipeTransportError::Binding);
    }
    Ok(())
}

type Body<'a> = (
    &'a str,
    &'a str,
    Vec<(&'a str, Value<'a>)>,
    Vec<(&'a str, Vec<(&'a str, Value<'a>)>)>,
);
fn inspect_message(
    msg: &Message,
    spec: &TransientUnitSpec,
    stdout: &File,
    stderr: &File,
) -> Result<(), OriginalPipeTransportError> {
    let h = msg.header();
    if msg.message_type() != Type::MethodCall
        || h.path().map(|v| v.as_str()) != Some(PATH)
        || h.destination().map(|v| v.as_str()) != Some(DESTINATION)
        || h.interface().map(|v| v.as_str()) != Some(INTERFACE)
        || h.member().map(|v| v.as_str()) != Some(METHOD)
        || h.sender().is_some()
        || !msg.primary_header().flags().is_empty()
        || msg.data().fds().len() != 2
    {
        return Err(OriginalPipeTransportError::Binding);
    }
    let body = msg.body();
    let (name, mode, properties, auxiliary): Body<'_> = body.deserialize()?;
    if name != spec.unit_name
        || mode != "fail"
        || !auxiliary.is_empty()
        || properties.len() != spec.properties.len() + 2
    {
        return Err(OriginalPipeTransportError::Binding);
    }
    for ((name, expected), (actual_name, actual)) in spec.properties.iter().zip(&properties) {
        if name != actual_name || *actual != value(expected) {
            return Err(OriginalPipeTransportError::Binding);
        }
    }
    let streams = &properties[spec.properties.len()..];
    if streams[0].0 != "StandardOutputFileDescriptor"
        || streams[1].0 != "StandardErrorFileDescriptor"
    {
        return Err(OriginalPipeTransportError::Binding);
    }
    descriptor(&streams[0].1, stdout)?;
    descriptor(&streams[1].1, stderr)
}

/// Owns two message writer copies and the entire original chain. Not executable.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::OriginalCollectorPipeTransport;
/// fn send(p: OriginalCollectorPipeTransport<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.send(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::OriginalCollectorPipeTransport;
/// fn extract(p: OriginalCollectorPipeTransport<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.into_message(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::OriginalCollectorPipeTransport;
/// fn copy(p: OriginalCollectorPipeTransport<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::OriginalCollectorPipeTransport;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<OriginalCollectorPipeTransport<'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static>>();
/// ```
/// The rejected alarm composition is test-only; it grants no production API.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::OriginalCollectorPipeTransport;
/// fn alarm(p: OriginalCollectorPipeTransport<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.prepare_original_safety_alarm(); }
/// ```
pub struct OriginalCollectorPipeTransport<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    // Message-owned duplicates close before the original pipes and fence owner.
    message: Option<Message>,
    material: OriginalCollectorStartMaterial<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
}
impl OriginalCollectorPipeTransport<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.material.is_refused()
    }
    fn validate(
        &mut self,
        allow_fixture: bool,
        construct: bool,
    ) -> Result<(), OriginalPipeTransportError> {
        if !self.material.ready || self.is_refused() {
            self.material.outputs.refuse();
            return Err(OriginalPipeTransportError::Refused);
        }
        let result = (|| {
            self.material
                .outputs
                .material_round()
                .map_err(OriginalStartMaterialError::from)?;
            self.material.bind(allow_fixture)?;
            let r = &self
                .material
                .outputs
                .pipes
                .starting
                .original
                .original
                .prepared
                .original
                .original
                .reservation;
            let spec = worker(&r.state, &self.material.projection)?;
            let pipes = &self.material.outputs.pipes.pipes;
            let [Some(stdout), Some(stderr)] = pipes else {
                return Err(OriginalPipeTransportError::Binding);
            };
            if construct {
                self.message = Some(message(&spec, &stdout.writer, &stderr.writer)?);
            }
            inspect_message(
                self.message
                    .as_ref()
                    .ok_or(OriginalPipeTransportError::Binding)?,
                &spec,
                &stdout.writer,
                &stderr.writer,
            )?;
            self.material
                .outputs
                .material_round()
                .map_err(OriginalStartMaterialError::from)?;
            Ok(())
        })();
        if result.is_err() {
            self.material.outputs.refuse();
        }
        result
    }
    pub fn revalidate(&mut self) -> Result<(), OriginalPipeTransportError> {
        self.validate(false, false)
    }
    #[cfg(test)]
    pub(crate) fn revalidate_fixture_for_test(&mut self) -> Result<(), OriginalPipeTransportError> {
        self.validate(true, false)
    }
    #[cfg(test)]
    pub(crate) fn remove_message_for_test(&mut self) {
        self.message = None;
    }
}
impl fmt::Debug for OriginalCollectorPipeTransport<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OriginalCollectorPipeTransport")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
pub struct OriginalPipeTransportFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: OriginalPipeTransportError,
    held: Box<OriginalCollectorPipeTransport<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl OriginalPipeTransportFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn error(&self) -> &OriginalPipeTransportError {
        &self.error
    }
    pub fn is_refused(&self) -> bool {
        self.held.is_refused()
    }
}
impl fmt::Debug for OriginalPipeTransportFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.error, f)
    }
}
impl fmt::Display for OriginalPipeTransportFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}
impl std::error::Error for OriginalPipeTransportFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalCollectorStartMaterial<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub fn prepare_original_pipe_transport(
        self,
    ) -> Result<
        OriginalCollectorPipeTransport<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalPipeTransportFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        self.prepare_transport_with(false)
    }
    #[cfg(test)]
    pub(crate) fn prepare_original_pipe_transport_for_test(
        self,
    ) -> Result<
        OriginalCollectorPipeTransport<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalPipeTransportFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        self.prepare_transport_with(true)
    }
    fn prepare_transport_with(
        self,
        allow_fixture: bool,
    ) -> Result<
        OriginalCollectorPipeTransport<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalPipeTransportFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut held = OriginalCollectorPipeTransport {
            message: None,
            material: self,
        };
        match held.validate(allow_fixture, true) {
            Ok(()) => Ok(held),
            Err(error) => Err(OriginalPipeTransportFailure {
                error,
                held: Box::new(held),
            }),
        }
    }
}

#[cfg(test)]
#[path = "diagnostic_original_pipe_transport_tests.rs"]
mod tests;
