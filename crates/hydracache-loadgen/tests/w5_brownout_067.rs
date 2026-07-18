#![allow(dead_code)]

use std::collections::BTreeSet;

pub use hydracache_loadgen::{histogram, rate, report};
pub mod targets {
    pub use hydracache_loadgen::targets::{control_plane, grid_model};
}
pub mod tiers {
    pub use hydracache_loadgen::tiers::resp_reference;
}

#[path = "../src/targets/brownout.rs"]
mod brownout;

use brownout::{
    BrownoutRunMode, ControlPlaneActionReceipt, ControlPlaneBrownoutAction,
    ControlPlaneBrownoutScenario, GridModelBrownoutScenario, ModelReplicaFault,
    RespBrownoutScenario, GRID_MODEL_REFERENCE_ENV, RESP_REFERENCE_ENV, W5_CANARY_MARKER,
};

const CONTROL_SCENARIO: &str =
    include_str!("../../../docs/testing/perf-scenarios/0.67/brownout-control-plane-v1.toml");
const RESP_SCENARIO: &str =
    include_str!("../../../docs/testing/perf-scenarios/0.67/brownout-resp-endpoint-v1.toml");
const MODEL_SCENARIO: &str =
    include_str!("../../../docs/testing/perf-scenarios/0.67/brownout-grid-model-v1.toml");

fn scenarios() -> (
    ControlPlaneBrownoutScenario,
    RespBrownoutScenario,
    GridModelBrownoutScenario,
) {
    (
        ControlPlaneBrownoutScenario::parse_toml(CONTROL_SCENARIO).unwrap(),
        RespBrownoutScenario::parse_toml(RESP_SCENARIO).unwrap(),
        GridModelBrownoutScenario::parse_toml(MODEL_SCENARIO).unwrap(),
    )
}

#[test]
fn w5_committed_scenarios_have_exact_digests_and_non_combinable_authorities() {
    assert_eq!(RESP_REFERENCE_ENV, "HYDRACACHE_RUN_PERF_RESP");
    assert_eq!(GRID_MODEL_REFERENCE_ENV, "HYDRACACHE_RUN_PERF_CORE");
    let (control, resp, model) = scenarios();
    assert_eq!(control.reference.predecessor_node_count, 3);
    assert_eq!(
        control.reference.committed_scenario_sha256,
        control.contract_sha256()
    );
    assert_eq!(
        resp.reference.committed_scenario_sha256,
        resp.contract_sha256()
    );
    assert_eq!(
        model.reference.committed_scenario_sha256,
        model.contract_sha256()
    );
    control.validate_exact_reference_shape().unwrap();
    resp.validate_exact_reference_shape().unwrap();
    model.validate_exact_reference_shape().unwrap();

    let evidence_classes = BTreeSet::from([
        control.identity.evidence_class.as_str(),
        resp.identity.evidence_class.as_str(),
        model.identity.evidence_class.as_str(),
    ]);
    let headlines = BTreeSet::from([
        control.identity.headline_metric.as_str(),
        resp.identity.headline_metric.as_str(),
        model.identity.headline_metric.as_str(),
    ]);
    assert_eq!(evidence_classes.len(), 3);
    assert_eq!(headlines.len(), 3);
    assert!(!control.identity.aggregate_goodput);
    assert!(!resp.identity.aggregate_goodput);
    assert!(!model.identity.aggregate_goodput);
}

#[test]
fn w5_reference_shape_rejects_every_material_contract_drift() {
    let (control, resp, model) = scenarios();

    let mut changed = control.clone();
    changed.events.max_leader_unavailable_millis += 1;
    assert!(changed.validate_exact_reference_shape().is_err());
    let mut changed = control.clone();
    changed.reference.predecessor_node_count = 5;
    assert!(changed.validate_exact_reference_shape().is_err());
    let mut changed = control;
    changed.identity.generic_client_write_invariant = true;
    assert!(changed.validate_exact_reference_shape().is_err());

    let mut changed = resp.clone();
    changed.event.independent_control_endpoints += 1;
    assert!(changed.validate_exact_reference_shape().is_err());
    let mut changed = resp;
    changed.identity.data_recovery_claim = true;
    assert!(changed.validate_exact_reference_shape().is_err());

    let mut changed = model.clone();
    changed.work.raw_repeats = 4;
    assert!(changed.validate_exact_reference_shape().is_err());
    let mut changed = model;
    changed.work.fresh_model_per_repeat = false;
    assert!(changed.validate_exact_reference_shape().is_err());
}

#[test]
fn w5_smoke_provenance_cannot_be_promoted_by_changing_mode() {
    let (control_scenario, resp_scenario, model_scenario) = scenarios();

    let mut control = brownout::run_control_plane_smoke(&control_scenario).unwrap();
    control.run_mode = BrownoutRunMode::Reference;
    assert!(control.validate(&control_scenario).is_err());

    let mut resp = brownout::run_resp_smoke(&resp_scenario).unwrap();
    resp.run_mode = BrownoutRunMode::Reference;
    assert!(resp.validate(&resp_scenario).is_err());

    let mut model = brownout::run_grid_model_smoke(&model_scenario).unwrap();
    model.run_mode = BrownoutRunMode::Reference;
    assert!(model.validate(&model_scenario).is_err());
}

#[test]
fn w5a_validates_leader_term_exact_membership_diffs_and_pid_lifecycle() {
    let (scenario, _, _) = scenarios();
    let baseline = brownout::run_control_plane_smoke(&scenario).unwrap();
    baseline.validate(&scenario).unwrap();
    assert_eq!(baseline.predecessor_node_count, 3);

    let mut bad_node_count = baseline.clone();
    bad_node_count.predecessor_node_count = 5;
    assert!(bad_node_count.validate(&scenario).is_err());

    let mut bad_term = baseline.clone();
    let leader = bad_term
        .events
        .iter_mut()
        .find(|event| event.action == ControlPlaneBrownoutAction::LeaderFailover)
        .unwrap();
    for snapshot in &mut leader.raw.after {
        snapshot.admin_status.term = 1;
        snapshot.cluster_overview.leader.as_mut().unwrap().term = 1;
    }
    assert!(bad_term.validate(&scenario).is_err());

    let mut bad_add = baseline.clone();
    let add = bad_add
        .events
        .iter_mut()
        .find(|event| event.action == ControlPlaneBrownoutAction::MemberAdd)
        .unwrap();
    add.raw.after.pop();
    assert!(bad_add.validate(&scenario).is_err());

    let mut bad_drain = baseline.clone();
    let drain = bad_drain
        .events
        .iter_mut()
        .find(|event| event.action == ControlPlaneBrownoutAction::MemberDrain)
        .unwrap();
    if let ControlPlaneActionReceipt::MemberDrain { action, .. } = &mut drain.raw.receipt {
        action.target_node_id = "node-b".to_owned();
    }
    assert!(bad_drain.validate(&scenario).is_err());

    let mut value = serde_json::to_value(&baseline).unwrap();
    let events = value["events"].as_array_mut().unwrap();
    let kill = events
        .iter_mut()
        .find(|event| event["action"] == "node_kill_rejoin")
        .unwrap();
    kill["raw"]["receipt"]["receipt"]["restarted"]["pid"] = 103.into();
    let bad_pid = serde_json::from_value(value).unwrap();
    assert!(brownout::ControlPlaneBrownoutReport::validate(&bad_pid, &scenario).is_err());
}

#[test]
fn w5b_binds_exact_socket_process_and_independent_control_without_data_recovery() {
    let (_, scenario, _) = scenarios();
    let baseline = brownout::run_resp_smoke(&scenario).unwrap();
    baseline.validate(&scenario).unwrap();

    let json = serde_json::to_string(&baseline).unwrap();
    assert!(!json.contains("data_recovery"));
    assert!(!json.contains("state_digest"));

    let mut selected_as_control = baseline.clone();
    selected_as_control.event.independent_controls[0].endpoint =
        selected_as_control.event.selected_endpoint.clone();
    assert!(selected_as_control.validate(&scenario).is_err());

    let mut value = serde_json::to_value(&baseline).unwrap();
    value["event"]["restarted"]["pid"] = 201.into();
    let reused_pid = serde_json::from_value(value).unwrap();
    assert!(brownout::RespBrownoutReport::validate(&reused_pid, &scenario).is_err());
}

#[test]
fn w5c_uses_real_primitive_with_five_fresh_repeats_and_independent_checksums() {
    let (_, _, scenario) = scenarios();
    let baseline = brownout::run_grid_model_smoke(&scenario).unwrap();
    baseline.validate(&scenario).unwrap();

    for fault in &baseline.faults {
        assert_eq!(fault.raw_repeats.len(), 5);
        assert_eq!(
            fault
                .raw_repeats
                .iter()
                .map(|repeat| repeat.fresh_model_identity_sha256.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            5
        );
        assert_eq!(fault.primitive, "LiveReplicationPeer::send_record");
        match fault.fault {
            ModelReplicaFault::SlowReplica => {
                assert!(fault.raw_repeats.iter().all(|repeat| {
                    repeat.slow_primitive_calls == repeat.steady_iterations
                        && repeat.unavailable_decisions == 0
                }));
            }
            ModelReplicaFault::UnavailableReplica => {
                assert!(fault.raw_repeats.iter().all(|repeat| {
                    repeat.unavailable_decisions == repeat.steady_iterations
                        && repeat.slow_primitive_calls == 0
                }));
            }
        }
    }

    let mut forged = baseline.clone();
    forged.faults[0].raw_repeats[0].fault_result_checksum ^= 1;
    assert!(forged.validate(&scenario).is_err());

    let mut synthetic_knee = baseline;
    synthetic_knee.predecessor.synthetic_knee_rate_per_second = Some(1_000);
    assert!(synthetic_knee.validate(&scenario).is_err());
}

#[test]
fn canary_extended_leader_downtime_breaches_the_control_plane_budget() {
    let (control, resp, model) = scenarios();
    let message =
        brownout::canary_extended_leader_downtime_breaches_the_control_plane_brownout_budget(
            &control, &resp, &model,
        )
        .unwrap_err();
    assert!(message.contains(W5_CANARY_MARKER));
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W5") {
        panic!("{message}");
    }
}
