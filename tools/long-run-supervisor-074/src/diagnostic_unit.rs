//! Construction only. Never starts a transient unit; no live backend enrolled.
use crate::diagnostic_lease::{
    cell_intent, CellIntent, DiagnosticState, CELL_SECONDS, MAX_RECEIPT_BYTES,
};
use crate::systemd_unit::{ExecCommand, TransientUnitSpec, UnitProperty};

/// Build only from validated persisted state, not caller-supplied argv or paths.
pub fn build_diagnostic_unit_spec(state: &DiagnosticState) -> Result<TransientUnitSpec, String> {
    let intent = cell_intent(state).map_err(|error| error.to_string())?;
    if !matches!(
        state.stage,
        crate::diagnostic_lease::DiagnosticStage::Reserved
            | crate::diagnostic_lease::DiagnosticStage::Starting
    ) || intent.maximum_runtime_seconds == 0
        || intent.maximum_runtime_seconds > CELL_SECONDS
    {
        return Err("diagnostic unit cannot start in this state or past its budget".into());
    }
    Ok(spec(&intent))
}

fn spec(intent: &CellIntent) -> TransientUnitSpec {
    let receipts = format!(
        "/var/lib/hydracache-performance/diagnostics/{}/{}",
        intent.lease_id, intent.surface
    );
    TransientUnitSpec {
        unit_name: intent.unit_name.clone(),
        properties: vec![
            (
                "Description",
                UnitProperty::Text(
                    "HydraCache local-only diagnostic P0 preset; not qualification".into(),
                ),
            ),
            ("User", UnitProperty::Text("hydracache-perf".into())),
            ("Group", UnitProperty::Text("hydracache-perf".into())),
            ("Type", UnitProperty::Text("exec".into())),
            ("RemainAfterExit", UnitProperty::Boolean(true)),
            ("Restart", UnitProperty::Text("no".into())),
            ("KillMode", UnitProperty::Text("control-group".into())),
            ("SendSIGKILL", UnitProperty::Boolean(true)),
            ("Delegate", UnitProperty::Boolean(false)),
            ("Slice", UnitProperty::Text("system.slice".into())),
            ("CPUAffinity", UnitProperty::Bytes(vec![0x02])),
            (
                "RuntimeMaxUSec",
                UnitProperty::Unsigned(intent.maximum_runtime_seconds * 1_000_000),
            ),
            ("TimeoutStopUSec", UnitProperty::Unsigned(1_000_000)),
            ("MemoryMax", UnitProperty::Unsigned(8 * 1024 * 1024 * 1024)),
            ("TasksMax", UnitProperty::Unsigned(256)),
            ("LimitNOFILE", UnitProperty::Unsigned(4096)),
            // Each of two spool files has half the combined receipt ceiling.
            ("LimitFSIZE", UnitProperty::Unsigned(MAX_RECEIPT_BYTES / 2)),
            ("NoNewPrivileges", UnitProperty::Boolean(true)),
            ("PrivateTmp", UnitProperty::Boolean(true)),
            ("PrivateDevices", UnitProperty::Boolean(true)),
            ("ProtectSystem", UnitProperty::Text("strict".into())),
            ("ProtectHome", UnitProperty::Text("yes".into())),
            ("ProtectControlGroups", UnitProperty::Boolean(true)),
            ("ProtectKernelTunables", UnitProperty::Boolean(true)),
            ("ProtectKernelModules", UnitProperty::Boolean(true)),
            ("ProtectKernelLogs", UnitProperty::Boolean(true)),
            ("RestrictSUIDSGID", UnitProperty::Boolean(true)),
            ("RestrictRealtime", UnitProperty::Boolean(true)),
            ("LockPersonality", UnitProperty::Boolean(true)),
            ("IOAccounting", UnitProperty::Boolean(true)),
            (
                "Environment",
                UnitProperty::Strings(vec!["PATH=/usr/bin:/bin".into(), "LC_ALL=C".into()]),
            ),
            ("WorkingDirectory", UnitProperty::Text(receipts.clone())),
            (
                "ReadWritePaths",
                UnitProperty::Strings(vec![receipts.clone()]),
            ),
            ("StandardInput", UnitProperty::Text("null".into())),
            (
                "StandardOutputFileToAppend",
                UnitProperty::Text(format!("{receipts}/stdout.json")),
            ),
            (
                "StandardErrorFileToAppend",
                UnitProperty::Text(format!("{receipts}/stderr.log")),
            ),
            (
                "ExecStart",
                UnitProperty::Commands(vec![ExecCommand {
                    path: intent.binary_path.clone(),
                    argv: vec![
                        intent.binary_path.clone(),
                        "--run".into(),
                        intent.config_path.clone(),
                        intent.binary_sha256.clone(),
                        intent.source_commit.clone(),
                    ],
                    ignore_failure: false,
                }]),
            ),
        ],
    }
}
