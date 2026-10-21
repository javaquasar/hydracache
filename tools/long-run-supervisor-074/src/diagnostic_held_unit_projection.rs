//! Exact semantic projection under the consumed owner, NOT a runnable spec.
//! Unmaterialized FD roles are explicit: no socket/bootstrap creation, FD
//! duplication, Message, send, release, caller-selected command or extraction.
use super::*;
use crate::diagnostic_lease::{cell_intent, DiagnosticState, BINARY_PATH, SOURCE_COMMIT};
use crate::diagnostic_two_phase_launch::held_placement::SignedPlacement;
use crate::systemd_unit::UnitProperty;
use std::os::unix::fs::MetadataExt;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Image {
    identity: [u64; 3],
    sha256: String,
}

// These are semantic slots, NOT raw descriptors. New endpoints/copies must be
// materialized and joined to a future expanded inventory before ANY transport.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Slot {
    DedicatedEmptyInputPipe,
    OriginalStdout([u64; 2]),
    OriginalStderr([u64; 2]),
    AuthenticatedReleaseEndpoint,
    OriginalWorker(Image),
    SealedCopyOfOriginalConfig(Image),
    OriginalCreatorPidfd(i64),
    ArmedRootGuardian(i32, [u64; 2]),
    OriginalAbsoluteAlarm(u64),
    SealedBootstrap256,
}

#[derive(Clone, PartialEq, Eq)]
struct Context {
    state: DiagnosticState,
    owner: [i64; 6],
    origins: [u64; 4],
    bounds: [u64; 4],
    inventory: [i32; 10],
    custody: [i32; 2],
    addressed: [i32; 2],
    names: (String, String),
    serial: NonZeroU32,
    root: RootBindingClaims,
    documents_sha256: [String; 5],
    host: Image,
    launcher: Image,
    worker: Image,
    config: Image,
    streams: [[u64; 2]; 2],
    guardians: [(i32, [u64; 2]); 2],
}

#[derive(PartialEq, Eq)]
struct Projection {
    context: Context,
    unit: String,
    slice: String,
    root_path: String,
    held_path: String,
    properties: Vec<(&'static str, UnitProperty)>,
    launcher_argv: [String; 1],
    worker_argv: [String; 5],
    slots: [Slot; 11],
}

fn project(context: Context) -> Result<Projection, HeldJoinError> {
    let intent = cell_intent(&context.state).map_err(|_| HeldJoinError::Binding)?;
    let min = context.bounds[..3].iter().copied().min();
    if context.bounds[3] == 0
        || min != Some(context.bounds[3])
        || context.root.deadline_boottime_ns != context.bounds[3]
        || context.worker.sha256 != intent.binary_sha256
        || context.config.sha256 != intent.config_sha256
        || context.host.identity == context.launcher.identity
        || context.streams[0] == context.streams[1]
        || context.guardians[0].0 <= 1
        || context.guardians[1].0 <= 1
        || context.guardians[0].0 == context.guardians[1].0
        || context
            .guardians
            .iter()
            .any(|g| i64::from(g.0) == context.owner[0])
    {
        return Err(HeldJoinError::Binding);
    }
    let attempt: String = context
        .root
        .attempt_sha256
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let slice = format!("system-hc074{attempt}.slice");
    let root_path = format!("/system.slice/{slice}");
    // Preserve all hardening/limits, but do not smuggle the legacy direct-worker
    // ExecStart or pathname/stdio/environment into the held semantics.
    let mut properties = crate::diagnostic_unit::build_diagnostic_unit_spec(&context.state)
        .map_err(|_| HeldJoinError::Binding)?
        .properties;
    properties.retain(|(name, _)| {
        !matches!(
            *name,
            "ExecStart"
                | "StandardInput"
                | "StandardOutputFileToAppend"
                | "StandardErrorFileToAppend"
        )
    });
    for (name, value) in &mut properties {
        match *name {
            "Description" => {
                *value =
                    UnitProperty::Text("HydraCache held diagnostic projection; no transport".into())
            }
            "Slice" => *value = UnitProperty::Text(slice.clone()),
            "Environment" | "ReadWritePaths" => *value = UnitProperty::Strings(vec![]),
            "WorkingDirectory" => *value = UnitProperty::Text("/".into()),
            "LimitFSIZE" => *value = UnitProperty::Unsigned(0),
            _ => {}
        }
    }
    let slots = [
        Slot::DedicatedEmptyInputPipe,
        Slot::OriginalStdout(context.streams[0]),
        Slot::OriginalStderr(context.streams[1]),
        Slot::AuthenticatedReleaseEndpoint,
        Slot::OriginalWorker(context.worker.clone()),
        Slot::SealedCopyOfOriginalConfig(context.config.clone()),
        Slot::OriginalCreatorPidfd(context.owner[0]),
        Slot::ArmedRootGuardian(context.guardians[0].0, context.guardians[0].1),
        Slot::ArmedRootGuardian(context.guardians[1].0, context.guardians[1].1),
        Slot::OriginalAbsoluteAlarm(context.bounds[3]),
        Slot::SealedBootstrap256,
    ];
    Ok(Projection {
        held_path: format!("{root_path}/{}", intent.unit_name),
        unit: intent.unit_name,
        slice,
        root_path,
        properties,
        launcher_argv: ["diagnostic-held-launcher-074".into()],
        worker_argv: [
            BINARY_PATH.into(),
            "--run".into(),
            "/proc/self/fd/5".into(),
            context.worker.sha256.clone(),
            SOURCE_COMMIT.into(),
        ],
        slots,
        context,
    })
}

trait Owner: Checks {
    fn context(&self) -> Result<Context, HeldJoinError>;
}
// Fresh unchanged full validations bracket the pure projection; comparison is
// not a cached success. Any first error retires the SAME retained owner.
fn round(
    owner: &mut impl Owner,
    retained: Option<&Projection>,
) -> Result<Projection, HeldJoinError> {
    let result = (|| {
        validate(owner)?;
        let opening = project(owner.context()?)?;
        if retained.is_some_and(|old| old != &opening) {
            return Err(HeldJoinError::Binding);
        }
        validate(owner)?;
        if project(owner.context()?)? != opening {
            return Err(HeldJoinError::Binding);
        }
        Ok(opening)
    })();
    if result.is_err() {
        owner.refuse();
    }
    result
}

impl Owner for JoinedHeldPlacement<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn context(&self) -> Result<Context, HeldJoinError> {
        let owned = &self.placement.held;
        let owner = &owned.held.owner;
        let binding = owner.binding.as_ref().ok_or(HeldJoinError::Binding)?;
        let source = &binding.source;
        let doc: SignedPlacement =
            serde_json::from_slice(&owned.raw).map_err(|_| HeldJoinError::Binding)?;
        let launch = &doc.placement.launch_claims;
        if launch.consent_claims.starting_state != source.state {
            return Err(HeldJoinError::Binding);
        }
        // Fresh authentication is done by validate(), not by this deserialization.
        let identities = self.roles.projection_identities()?;
        let material = &owner
            .original
            .original
            .original
            .original
            .original
            .transport
            .material;
        let intent = cell_intent(&source.state).map_err(|_| HeldJoinError::Binding)?;
        let [(worker_id, worker), (config_id, config)] = material
            .bundle
            .as_ref()
            .ok_or(HeldJoinError::Binding)?
            .projection_contents(&intent.surface)?;
        let [Some(stdout), Some(stderr)] = &material.outputs.pipes.pipes else {
            return Err(HeldJoinError::Binding);
        };
        let id = |file: &std::fs::File| -> Result<_, HeldJoinError> {
            let metadata = file.metadata().map_err(ArtifactError::from)?;
            Ok([metadata.dev(), metadata.ino()])
        };
        let docs = self
            .documents
            .as_ref()
            .ok_or(HeldJoinError::Binding)?
            .slices();
        Ok(Context {
            state: source.state.clone(),
            owner: source.owner,
            origins: source.origins,
            bounds: source.bounds,
            inventory: source.descriptors,
            custody: source.guardians,
            addressed: binding.addressed,
            names: source.names.clone(),
            serial: source.serial,
            root: owned.snapshot.root,
            documents_sha256: [
                crate::sha256_hex(docs[0]),
                crate::sha256_hex(docs[1]),
                crate::sha256_hex(docs[2]),
                crate::sha256_hex(docs[3]),
                crate::sha256_hex(&owned.raw),
            ],
            host: Image {
                identity: identities[0],
                sha256: launch.artifacts.watchdog_sha256.clone(),
            },
            launcher: Image {
                identity: identities[1],
                sha256: launch.artifacts.launcher_sha256.clone(),
            },
            worker: Image {
                identity: worker_id,
                sha256: worker.sha256,
            },
            config: Image {
                identity: config_id,
                sha256: config.sha256,
            },
            streams: [id(&stdout.writer)?, id(&stderr.writer)?],
            guardians: owned
                .held
                .controls
                .projection_guardians()
                .map_err(ArtifactError::from)?,
        })
    }
}

pub(crate) struct ProjectedHeldUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    owner: JoinedHeldPlacement<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    projection: Projection,
}
impl ProjectedHeldUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn revalidate(&mut self) -> Result<(), HeldJoinError> {
        round(&mut self.owner, Some(&self.projection)).map(|_| ())
    }
}
impl<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    JoinedHeldPlacement<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn project_held_unit(
        mut self,
    ) -> Result<
        ProjectedHeldUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        HeldJoinFailure<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        match round(&mut self, None) {
            Ok(projection) => Ok(ProjectedHeldUnit {
                owner: self,
                projection,
            }),
            Err(error) => Err(HeldJoinFailure {
                error: Box::new(error),
                held: Box::new(self),
            }),
        }
    }
}

#[cfg(test)]
#[path = "diagnostic_held_unit_projection_tests.rs"]
mod tests;
