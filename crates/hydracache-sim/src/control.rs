use std::fmt;

use serde::{Deserialize, Serialize};

/// Stable replay-script format version.
pub const REPLAY_SCRIPT_VERSION: u16 = 1;

/// Maximum number of replay actions accepted from a share/replay artifact.
pub const MAX_REPLAY_ACTIONS: usize = 256;

/// Simulator mode recorded in replay scripts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimMode {
    /// User actions only.
    Manual,
    /// Curated script loop only.
    Scripted,
    /// Scripted loop plus user interventions.
    Mixed,
}

/// Versioned replay artifact for interactive simulator controls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayScriptV1 {
    /// Replay artifact version.
    pub version: u16,
    /// Seed used to construct the simulator.
    pub seed: u64,
    /// Mode visible to the lab UI.
    pub mode: SimMode,
    /// Optional curated scenario name.
    pub scenario: Option<String>,
    /// Ordered replay-visible control actions.
    pub actions: Vec<ControlActionV1>,
}

impl ReplayScriptV1 {
    /// Create a replay script with the current schema version.
    pub fn new(seed: u64, mode: SimMode, actions: Vec<ControlActionV1>) -> Self {
        Self {
            version: REPLAY_SCRIPT_VERSION,
            seed,
            mode,
            scenario: None,
            actions,
        }
    }

    /// Serialize this script as stable JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("replay script serialization is infallible")
    }

    /// Decode a replay script and reject future versions or oversized action logs.
    pub fn from_json(input: &str) -> Result<Self, ReplayScriptError> {
        let header: ReplayScriptHeader =
            serde_json::from_str(input).map_err(|error| ReplayScriptError::InvalidJson {
                message: error.to_string(),
            })?;
        if header.version != REPLAY_SCRIPT_VERSION {
            return Err(ReplayScriptError::UnsupportedVersion {
                found: header.version,
                max_supported: REPLAY_SCRIPT_VERSION,
            });
        }
        let script: Self =
            serde_json::from_str(input).map_err(|error| ReplayScriptError::InvalidJson {
                message: error.to_string(),
            })?;
        script.validate()?;
        Ok(script)
    }

    /// Validate compatibility bounds after ordinary serde decoding.
    pub fn validate(&self) -> Result<(), ReplayScriptError> {
        if self.version != REPLAY_SCRIPT_VERSION {
            return Err(ReplayScriptError::UnsupportedVersion {
                found: self.version,
                max_supported: REPLAY_SCRIPT_VERSION,
            });
        }
        if self.actions.len() > MAX_REPLAY_ACTIONS {
            return Err(ReplayScriptError::TooManyActions {
                found: self.actions.len(),
                max_supported: MAX_REPLAY_ACTIONS,
            });
        }
        Ok(())
    }
}

/// Closed control surface shared by native, WASM, and sandbox simulator paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlActionV1 {
    /// Advance the simulator by `n` scheduler steps.
    Step { at_step: u64, n: u64 },
    /// Isolate a node from all peers.
    Isolate { at_step: u64, node: String },
    /// Rejoin a previously isolated node.
    Rejoin { at_step: u64, node: String },
    /// Disable a node.
    Disable { at_step: u64, node: String },
    /// Enable a disabled node.
    Enable { at_step: u64, node: String },
    /// Add a deterministic node.
    AddNode { at_step: u64 },
    /// Push a client-visible cache event into the simulator.
    PushEvent {
        at_step: u64,
        client: String,
        ns: String,
        key: String,
        value: String,
    },
    /// Subscribe a client to namespace cache events.
    Subscribe {
        at_step: u64,
        client: String,
        ns: String,
    },
    /// Change the UI/lab mode.
    ModeChange { at_step: u64, mode: SimMode },
}

impl ControlActionV1 {
    /// Return the logical scheduler step at which the action applies.
    pub fn at_step(&self) -> u64 {
        match self {
            Self::Step { at_step, .. }
            | Self::Isolate { at_step, .. }
            | Self::Rejoin { at_step, .. }
            | Self::Disable { at_step, .. }
            | Self::Enable { at_step, .. }
            | Self::AddNode { at_step }
            | Self::PushEvent { at_step, .. }
            | Self::Subscribe { at_step, .. }
            | Self::ModeChange { at_step, .. } => *at_step,
        }
    }
}

/// Replay/control decode errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayScriptError {
    /// JSON could not be parsed.
    InvalidJson { message: String },
    /// Replay script version is not supported.
    UnsupportedVersion { found: u16, max_supported: u16 },
    /// The script carries too many actions for a bounded replay/share URL.
    TooManyActions { found: usize, max_supported: usize },
}

impl fmt::Display for ReplayScriptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson { message } => {
                write!(formatter, "invalid replay script JSON: {message}")
            }
            Self::UnsupportedVersion {
                found,
                max_supported,
            } => write!(
                formatter,
                "unsupported replay script version {found}; max supported is {max_supported}"
            ),
            Self::TooManyActions {
                found,
                max_supported,
            } => write!(
                formatter,
                "replay script has {found} actions; max supported is {max_supported}"
            ),
        }
    }
}

impl std::error::Error for ReplayScriptError {}

/// Error returned while applying a shared simulator control action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlApplyError {
    /// The action references an unknown or currently invalid target.
    InvalidAction(String),
    /// The action is part of the closed contract but is implemented by a later work item.
    UnsupportedAction(&'static str),
    /// The replay script itself failed compatibility checks.
    ReplayScript(ReplayScriptError),
}

impl fmt::Display for ControlApplyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAction(message) => formatter.write_str(message),
            Self::UnsupportedAction(action) => {
                write!(
                    formatter,
                    "control action '{action}' is not implemented in this simulator"
                )
            }
            Self::ReplayScript(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for ControlApplyError {}

impl From<ReplayScriptError> for ControlApplyError {
    fn from(error: ReplayScriptError) -> Self {
        Self::ReplayScript(error)
    }
}

#[derive(Debug, Deserialize)]
struct ReplayScriptHeader {
    version: u16,
}
