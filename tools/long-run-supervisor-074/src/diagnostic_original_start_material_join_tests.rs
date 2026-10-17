//! Real signed bytes are copied as nonexecuted fixture data; no installed-origin proof.
use super::*;
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::OriginalStartMaterialError;

struct Bundle {
    temp: tempfile::TempDir,
    path: PathBuf,
}
impl Bundle {
    fn new() -> Self {
        assert_ne!(unsafe { libc::geteuid() }, 0);
        let source = PathBuf::from(
            std::env::var_os("HC074_START_MATERIAL_BUNDLE")
                .expect("manual gate requires retained hosted bundle data"),
        );
        assert!(source.is_absolute());
        assert_ne!(source, Path::new(crate::diagnostic_artifacts::INSTALL_ROOT));
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("bundle");
        fs::create_dir(&path).unwrap();
        for name in [
            "timing-controls-074",
            "build-receipt-v1.json",
            "build-log.jsonl",
            "Cargo.lock.root",
            "Cargo.lock.observer",
            "embedded.json",
            "direct.json",
            "resp2.json",
            "resp3.json",
        ] {
            fs::copy(source.join(name), path.join(name)).unwrap();
            fs::set_permissions(
                path.join(name),
                fs::Permissions::from_mode(if name == "timing-controls-074" {
                    0o555
                } else {
                    0o444
                }),
            )
            .unwrap();
        }
        fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
        Self { temp, path }
    }
    fn change(&self) {
        let file = self.path.join("embedded.json");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
        fs::write(&file, b"changed original config\n").unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o444)).unwrap();
    }
    fn replace(&self) {
        fs::set_permissions(&self.path, fs::Permissions::from_mode(0o755)).unwrap();
        let file = self.path.join("embedded.json");
        let old = fs::read(&file).unwrap();
        fs::remove_file(&file).unwrap();
        fs::write(&file, old).unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o444)).unwrap();
        fs::set_permissions(&self.path, fs::Permissions::from_mode(0o555)).unwrap();
    }
}
impl Drop for Bundle {
    fn drop(&mut self) {
        // Exact owned temporary fixture only; restore directory access for TempDir.
        assert!(self.path.starts_with(self.temp.path()));
        fs::set_permissions(&self.path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}
fn records(leaf: &Path) -> Vec<Vec<u8>> {
    [NAME, RESERVED_NAME, COLLECTOR_STARTING_NAME]
        .into_iter()
        .map(|n| fs::read(leaf.join(n)).unwrap())
        .collect()
}
#[test]
#[ignore = "manual nonroot signed bundle and original output material; never executes observer"]
fn original_start_material_joined_healthy() {
    use crate::diagnostic_ipc::validation_trace::{self as trace, Stage};
    for sample in 0..3 {
        let bundle = Bundle::new();
        with_original_at(false, None, |original, _, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let owner = accepted(original, fence, leaf, receiver, sender, model)
                    .prepare_original_outputs_for_test(|_| Ok(()))
                    .unwrap();
                let before = records(leaf);
                let (mut material, preparation) = trace::capture(|| {
                    trace::stage(Stage::Bind);
                    owner
                        .prepare_original_start_material_for_test(&bundle.path, || {})
                        .unwrap()
                });
                let (_, validation) = trace::capture(|| {
                    trace::stage(Stage::Checks);
                    material.revalidate_fixture_for_test().unwrap()
                });
                assert_eq!(
                    preparation.stages[Stage::Bind as usize].authentication_calls,
                    2 * 17
                );
                assert_eq!(
                    validation.stages[Stage::Checks as usize].authentication_calls,
                    2 * 17
                );
                assert!(preparation.first_time.is_none() && validation.first_time.is_none());
                println!("HC074-START-MATERIAL-V2-COST sample={sample} phase=prepare report={preparation:?}");
                println!("HC074-START-MATERIAL-V2-COST sample={sample} phase=validate report={validation:?}");
                assert!(!material.is_refused());
                busy(root);
                assert_eq!(records(leaf), before);
                assert_eq!(fs::read_dir(leaf).unwrap().count(), 3);
                drop(material);
                retained_context_refuses_for_test(root);
                println!("HC074-START-MATERIAL-CONTROL sample={sample} original=true bundle=true fixture=true retained=true start=false");
            });
        });
    }
}
#[test]
#[ignore = "manual nonroot original material mutation/ownership gate; no execution"]
fn original_start_material_joined_refusals() {
    for case in [
        "load-content",
        "load-missing",
        "closing-trust",
        "closing-output",
        "later-content",
        "later-replace",
        "later-trust",
        "later-output",
        "projection",
        "starting",
        "descriptor",
        "fixture-entry",
        "expiry",
    ] {
        let bundle = Bundle::new();
        if case == "load-content" {
            bundle.change();
        }
        if case == "load-missing" {
            fs::set_permissions(&bundle.path, fs::Permissions::from_mode(0o755)).unwrap();
            fs::remove_file(bundle.path.join("embedded.json")).unwrap();
            fs::set_permissions(&bundle.path, fs::Permissions::from_mode(0o555)).unwrap();
        }
        with_original_at(true, None, |original, paths, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let owner = accepted(original, fence, leaf, receiver, sender, model)
                    .prepare_original_outputs_for_test(|_| Ok(()))
                    .unwrap();
                let before = records(leaf);
                let result = owner.prepare_original_start_material_for_test(&bundle.path, || {
                    if case == "closing-trust" {
                        fs::write(&paths.worker, b"original trust drift").unwrap();
                    }
                    if case == "closing-output" {
                        fs::write(
                            output(leaf).join("collector-attempt-v1/collector-stderr.bin"),
                            b"unowned bytes",
                        )
                        .unwrap();
                    }
                });
                match case {
                    "load-content" | "load-missing" | "closing-trust" | "closing-output" => {
                        let failure = result.unwrap_err();
                        assert!(failure.is_refused());
                        match case {
                            "load-content" | "load-missing" => assert!(matches!(
                                failure.error(),
                                OriginalStartMaterialError::Build(_)
                            )),
                            "closing-trust" => {
                                let reason = format!("{:?}", failure.error());
                                assert!(
                                    reason.contains("Input(Invalid)")
                                        && !reason.contains("Kernel(")
                                        && !reason.contains("Time"),
                                    "{reason}"
                                );
                            }
                            "closing-output" => assert!(matches!(
                                failure.error(),
                                OriginalStartMaterialError::Outputs(OriginalSinkError::Invalid)
                            )),
                            _ => unreachable!(),
                        }
                        busy(root);
                        drop(failure);
                    }
                    _ => {
                        let mut material = result.unwrap();
                        match case {
                            "later-content" => bundle.change(),
                            "later-replace" => bundle.replace(),
                            "later-trust" => {
                                fs::write(&paths.worker, b"later original trust drift").unwrap()
                            }
                            "later-output" => fs::write(
                                output(leaf).join("collector-attempt-v1/collector-stderr.bin"),
                                b"late unowned bytes",
                            )
                            .unwrap(),
                            "projection" => material.corrupt_for_test(0),
                            "starting" => material.corrupt_for_test(1),
                            "descriptor" => material.corrupt_for_test(2),
                            "fixture-entry" => (),
                            "expiry" => std::thread::sleep(std::time::Duration::from_secs(10)),
                            _ => unreachable!(),
                        }
                        let (error, event) =
                            crate::diagnostic_kernel_admission::refusal_trace::capture(|| {
                                if case == "fixture-entry" {
                                    material.revalidate().unwrap_err()
                                } else {
                                    material.revalidate_fixture_for_test().unwrap_err()
                                }
                            });
                        match case {
                            "later-content" | "later-replace" => assert!(
                                matches!(error, OriginalStartMaterialError::Build(_)),
                                "{case}: {error:?}"
                            ),
                            "projection" | "starting" => assert!(
                                matches!(error, OriginalStartMaterialError::Binding),
                                "{case}: {error:?}"
                            ),
                            "fixture-entry" => assert!(
                                matches!(error, OriginalStartMaterialError::Fixture),
                                "{error:?}"
                            ),
                            "later-trust" => {
                                let reason = format!("{error:?}");
                                assert!(
                                    reason.contains("Input(Invalid)")
                                        && !reason.contains("Kernel(")
                                        && !reason.contains("Time"),
                                    "{reason}"
                                );
                            }
                            "later-output" => assert!(
                                matches!(
                                    error,
                                    OriginalStartMaterialError::Outputs(OriginalSinkError::Invalid)
                                ),
                                "{error:?}"
                            ),
                            "descriptor" => assert!(
                                matches!(
                                    error,
                                    OriginalStartMaterialError::Outputs(OriginalSinkError::Invalid)
                                ),
                                "{error:?}"
                            ),
                            "expiry" => {
                                let event = event.expect("exact original progress age refusal");
                                assert_eq!(event.site, "progress-age");
                                assert_eq!(event.clock[5], 10_000_000_000);
                                assert!(event.clock[0] > event.clock[1]);
                                assert!(
                                    event.clock[2] >= event.clock[0]
                                        && event.clock[2] - event.clock[0] >= event.clock[5]
                                );
                                assert!(
                                    format!("{error:?}")
                                        .contains("Causal(Progress(Input(Kernel(Invalid))))"),
                                    "{error:?}"
                                );
                                println!("HC074-START-MATERIAL-EXPIRY event={event:?} limit_ns=10000000000 renewed=false");
                            }
                            _ => unreachable!(),
                        }
                        assert!(material.is_refused());
                        assert!(matches!(
                            material.revalidate_fixture_for_test(),
                            Err(OriginalStartMaterialError::Refused)
                        ));
                        busy(root);
                        drop(material);
                    }
                }
                assert_eq!(records(leaf), before);
                assert_eq!(fs::read_dir(leaf).unwrap().count(), 3);
                retained_context_refuses_for_test(root);
                println!("HC074-START-MATERIAL-REFUSAL case={case} exact=true sticky=true held=true unchanged=true start=false");
            });
        });
    }
}
