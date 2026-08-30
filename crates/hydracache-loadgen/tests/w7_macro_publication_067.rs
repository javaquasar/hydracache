#[test]
fn w7_publication_waits_for_the_final_reference_suite() {
    let source = include_str!("../src/main.rs");
    let resp = source
        .split_once("LoadgenCommand::SuiteResp { .. } => {")
        .expect("RESP suite arm")
        .1
        .split_once("LoadgenCommand::TierGridModel { .. } => {")
        .expect("end of RESP suite arm")
        .0;
    let control_plane = source
        .split_once("LoadgenCommand::SuiteControlPlane { .. } => {")
        .expect("control-plane suite arm")
        .1
        .split_once("LoadgenCommand::Brownout { .. } => {")
        .expect("end of control-plane suite arm")
        .0;

    assert_eq!(resp.matches("publish_w7_macro_tail").count(), 1);
    assert_eq!(control_plane.matches("publish_w7_macro_tail").count(), 0);
    assert!(control_plane.contains("absolute_output_path(&repo_root, predecessor)"));
}

#[test]
fn full_dress_produces_every_w7_family_before_resp_seals_the_batch() {
    let workflow = include_str!("../../../.github/workflows/ci.yml");
    let full_dress = workflow
        .split_once("name: Run full-dress control-plane reference evidence")
        .expect("full-dress control-plane step")
        .1;
    let core = full_dress
        .find("name: Run full-dress core reference evidence")
        .expect("full-dress core step");
    let resp = full_dress
        .find("name: Run full-dress RESP reference evidence")
        .expect("full-dress RESP step");

    assert!(core < resp, "core reports must exist before RESP seals W7");
}
