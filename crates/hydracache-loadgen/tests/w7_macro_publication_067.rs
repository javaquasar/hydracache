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

#[test]
fn exploratory_reference_job_produces_every_w7_family_before_resp_seals_the_batch() {
    let workflow = include_str!("../../../.github/workflows/ci.yml");
    let job = workflow
        .split_once("release-067-performance:")
        .expect("0.67 reference job")
        .1
        .split_once("release-0671-performance-qualification:")
        .expect("end of 0.67 reference job")
        .0;
    let control_plane = job
        .find("name: Run 0.67 control-plane performance evidence")
        .expect("control-plane producer");
    let core = job
        .find("name: Run 0.67 core performance evidence")
        .expect("core producer");
    let resp = job
        .find("name: Run 0.67 RESP performance evidence")
        .expect("RESP finalizer");

    assert!(
        control_plane < core && core < resp,
        "all macro producers must run before RESP seals W7"
    );
}

#[test]
fn serialized_candidate_jobs_keep_resp_as_the_w7_finalizer() {
    let workflow = include_str!("../../../.github/workflows/ci.yml");
    for (start, end, control_plane, core, resp) in [
        (
            "release-0671-performance-bootstrap:",
            "release-0671-frozen-candidate:",
            "name: Run bootstrap control-plane reference evidence",
            "name: Run bootstrap core reference evidence",
            "name: Run bootstrap RESP reference evidence",
        ),
        (
            "release-0671-frozen-candidate:",
            "raft-loom:",
            "name: Run frozen-candidate real 3/5/7 daemon control-plane evidence",
            "name: Run frozen-candidate core reference evidence",
            "name: Run frozen-candidate RESP and Redis reference evidence",
        ),
    ] {
        let job = workflow
            .split_once(start)
            .unwrap_or_else(|| panic!("missing job {start}"))
            .1
            .split_once(end)
            .unwrap_or_else(|| panic!("missing job boundary {end}"))
            .0;
        let control_plane = job
            .find(control_plane)
            .unwrap_or_else(|| panic!("missing control-plane step in {start}"));
        let core = job
            .find(core)
            .unwrap_or_else(|| panic!("missing core step in {start}"));
        let resp = job
            .find(resp)
            .unwrap_or_else(|| panic!("missing RESP step in {start}"));
        assert!(
            control_plane < core && core < resp,
            "{start} must produce every macro family before RESP seals W7"
        );
    }
}
