//! Root receiver, real separate nonroot sender; synthetic issuer stays test-only.
use super::*;
pub(in super::super) fn new() -> Model {
    let mut model = Model::new();
    model.root = crate::diagnostic_two_phase_launch::root_enrollment::privileged_fixture::new();
    fs::create_dir(model.root.path().join("etc")).unwrap();
    fs::create_dir(model.root.path().join("etc/hydracache-perf")).unwrap();
    model.document.client_uids = vec![65534];
    model.document.client_gid = 65534;
    model.write();
    model
}
