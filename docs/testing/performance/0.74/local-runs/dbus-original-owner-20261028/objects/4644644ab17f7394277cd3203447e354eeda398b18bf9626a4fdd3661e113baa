//! Separate root-local signed context; never changes the nonroot fixture.
use super::*;
pub(crate) fn new() -> (
    TempDir,
    TempDir,
    WorkerTrustInput,
    FixedAuthorityRegistry,
    Vec<u8>,
) {
    let mut model = kernel_model(kernel_local_host());
    model.root = crate::diagnostic_two_phase_launch::root_enrollment::privileged_fixture::new();
    model.authority_root =
        crate::diagnostic_two_phase_launch::root_enrollment::privileged_fixture::new();
    fs::create_dir(model.root.path().join("etc")).unwrap();
    fs::create_dir(model.root.path().join("etc/hydracache-perf")).unwrap();
    fs::write(model.config(), model.document()).unwrap();
    fs::set_permissions(model.config(), fs::Permissions::from_mode(0o600)).unwrap();
    let input = model.input();
    let checked =
        super::super::super::verify_worker_policy(&model.envelope, &model.trust, &model.host)
            .unwrap();
    let authority =
        fixture::fixed::privileged_fixture::initialize(model.authority_root.path(), &checked);
    (
        model.root,
        model.authority_root,
        input,
        authority,
        model.envelope,
    )
}
