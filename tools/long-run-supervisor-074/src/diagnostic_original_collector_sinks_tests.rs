use super::*;
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::PipePreparationStage;
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::{OriginalSinkError, SinkPreparationStage, OutputJoinStage};
use std::path::{Path, PathBuf};

#[path = "diagnostic_original_output_join_crash_tests.rs"]
mod crash_tests;
#[path = "diagnostic_original_output_join_fault_tests.rs"]
mod join_fault_tests;

#[test]
#[ignore = "manual nonroot J1 output cost and complete original control; no execution"]
fn original_output_join_cost_and_healthy_control() {
    use crate::diagnostic_ipc::validation_trace::{self as trace, Stage};
    assert_ne!(unsafe { libc::geteuid() }, 0);
    for sample in 0..3 {
        with_original_at(false, None, |original, _, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let framed = accepted(original, fence, leaf, receiver, sender, model);
                let expected = joined_cuts();
                let mut visited = Vec::new();
                let (mut owner, report) = trace::capture(|| {
                    trace::stage(Stage::Publish);
                    framed
                        .prepare_original_outputs_for_test(|at| {
                            visited.push(at);
                            assert_eq!(visited, expected[..visited.len()]);
                            busy(root);
                            Ok(())
                        })
                        .unwrap()
                });
                assert_eq!(visited, expected);
                assert_eq!(
                    report.stages[Stage::Publish as usize].authentication_calls,
                    46 * 17
                );
                assert_eq!(
                    report.stages[Stage::Publish as usize].signature_calls,
                    46 * 17
                );
                assert!(report.first_time.is_none());
                println!("HC074-OUTPUT-JOIN-COST sample={sample} rounds=46 ipc={report:?}");
                owner.revalidate().unwrap();
                inventory_at_cut(leaf, 9);
                owner.drop_eof_for_test();
                retained_context_refuses_for_test(root);
                println!("HC074-OUTPUT-JOIN-CONTROL sample={sample} ordered=25 empty=true drop_eof=true live_closing=true start=false");
            });
        });
    }
}

fn joined_cuts() -> Vec<OutputJoinStage> {
    super::STAGES
        .into_iter()
        .map(OutputJoinStage::Starting)
        .chain(
            [
                PipePreparationStage::BeforeCreate,
                PipePreparationStage::StdoutCreated,
                PipePreparationStage::StdoutReady,
                PipePreparationStage::StderrCreated,
                PipePreparationStage::StderrReady,
                PipePreparationStage::Acknowledging,
            ]
            .into_iter()
            .map(OutputJoinStage::Pipes),
        )
        .chain(CUTS.into_iter().map(OutputJoinStage::Sinks))
        .collect()
}

const CUTS: [SinkPreparationStage; 10] = [
    SinkPreparationStage::BeforeCreate,
    SinkPreparationStage::RootCreated,
    SinkPreparationStage::RootSynced,
    SinkPreparationStage::DirectoryCreated,
    SinkPreparationStage::DirectorySynced,
    SinkPreparationStage::StdoutCreated,
    SinkPreparationStage::StdoutSynced,
    SinkPreparationStage::StderrCreated,
    SinkPreparationStage::StderrSynced,
    SinkPreparationStage::Acknowledging,
];
fn output(leaf: &Path) -> PathBuf {
    leaf.parent().unwrap().join(format!(
        "diagnostic-collector-output-074-{}",
        crate::sha256_hex(&fs::read(leaf.join("diagnostic-collector-reserved-074.json")).unwrap())
    ))
}
fn inventory_at_cut(leaf: &Path, index: usize) {
    assert_eq!(fs::read_dir(leaf).unwrap().count(), 3);
    let root = output(leaf);
    if index == 0 {
        assert!(!root.exists());
        return;
    }
    assert_eq!(
        fs::read_dir(&root).unwrap().count(),
        usize::from(index >= 3)
    );
    if index < 3 {
        return;
    }
    let directory = root.join("collector-attempt-v1");
    let count = if index >= 7 {
        2
    } else {
        usize::from(index >= 5)
    };
    assert_eq!(fs::read_dir(&directory).unwrap().count(), count);
    for name in ["collector-stdout.bin", "collector-stderr.bin"]
        .into_iter()
        .take(count)
    {
        assert!(fs::read(directory.join(name)).unwrap().is_empty());
    }
}
#[test]
#[ignore = "manual nonroot original sink fault/live ownership; no worker or production enrollment"]
fn original_collector_sinks_joined_fault_and_live_refusal() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    for (index, cut) in CUTS.into_iter().enumerate() {
        with_original_at(true, None, |original, _, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let framed = accepted(original, fence, leaf, receiver, sender, model);
                let mut raw = None;
                let mut visited = Vec::new();
                let failure = framed
                    .prepare_original_outputs_for_test(|stage| {
                        let OutputJoinStage::Sinks(at) = stage else {
                            return Ok(());
                        };
                        if raw.is_none() {
                            raw = Some(fs::read(leaf.join(COLLECTOR_STARTING_NAME)).unwrap());
                        }
                        visited.push(at);
                        if at == cut {
                            return Err(OriginalSinkError::Interrupted(at));
                        }
                        Ok(())
                    })
                    .unwrap_err();
                assert!(
                    matches!(failure.error(), OriginalSinkError::Interrupted(at) if *at == cut),
                    "{failure:?}"
                );
                assert_eq!(visited, CUTS[..=index]);
                assert!(failure.is_refused());
                inventory_at_cut(leaf, index);
                assert_eq!(
                    fs::read(leaf.join(COLLECTOR_STARTING_NAME)).unwrap(),
                    raw.unwrap()
                );
                busy(root);
                drop(failure);
                retained_context_refuses_for_test(root);
                println!("HC074-SINK-INTERRUPT cut={cut:?} held=true refused=true unchanged=true");
            });
        });
    }
    for kind in 0..12 {
        with_original_at(true, None, |original, paths, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let framed = accepted(original, fence, leaf, receiver, sender, model);
                let mut owner = framed
                    .prepare_original_outputs_for_test(|_| Ok(()))
                    .unwrap();
                let directory = output(leaf).join("collector-attempt-v1");
                let sink = directory.join("collector-stdout.bin");
                match kind {
                    0 => fs::write(&sink, b"unowned bytes").unwrap(),
                    1 => fs::set_permissions(&sink, fs::Permissions::from_mode(0o400)).unwrap(),
                    2 => fs::hard_link(&sink, directory.join("alias")).unwrap(),
                    3 => {
                        fs::rename(&sink, directory.join("old")).unwrap();
                        fs::write(&sink, []).unwrap();
                    }
                    4 => {
                        fs::rename(&sink, directory.join("old")).unwrap();
                        symlink("old", &sink).unwrap();
                    }
                    5 => fs::write(directory.join("unknown"), b"retained").unwrap(),
                    6..=9 => owner.corrupt_descriptor_for_test(kind - 6),
                    10 => fs::write(&paths.worker, b"sink trust drift").unwrap(),
                    11 => {
                        fs::set_permissions(&directory, fs::Permissions::from_mode(0o500)).unwrap()
                    }
                    _ => unreachable!(),
                }
                let result = owner.revalidate();
                if kind == 9 {
                    assert!(matches!(result, Err(OriginalSinkError::Original(crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::OriginalPipeError::Invalid))), "kind={kind} {result:?}");
                } else if kind == 10 {
                    assert!(
                        matches!(result, Err(OriginalSinkError::Original(_))),
                        "kind={kind} {result:?}"
                    );
                    assert!(
                        format!("{result:?}").contains("Input(Invalid)"),
                        "{result:?}"
                    );
                } else {
                    assert!(
                        matches!(result, Err(OriginalSinkError::Invalid)),
                        "kind={kind} {result:?}"
                    );
                }
                assert!(owner.is_refused());
                if kind == 11 {
                    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
                }
                assert!(matches!(
                    owner.revalidate(),
                    Err(OriginalSinkError::Refused)
                ));
                busy(root);
                drop(owner);
                retained_context_refuses_for_test(root);
                println!("HC074-SINK-DRIFT kind={kind} held=true refused=true");
            });
        });
    }
    for kind in 0..4 {
        with_original_at(true, None, |original, paths, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let mut framed = accepted(original, fence, leaf, receiver, sender, model);
                if kind == 0 {
                    fs::create_dir(output(leaf)).unwrap();
                }
                if kind == 1 {
                    fs::write(&paths.worker, b"already refused sinks").unwrap();
                    assert!(framed.revalidate().is_err());
                }
                let result = if kind == 2 {
                    framed.prepare_original_outputs()
                } else {
                    framed.prepare_original_outputs_for_test(|stage| {
                        let OutputJoinStage::Sinks(at) = stage else {
                            return Ok(());
                        };
                        if kind == 3 && at == SinkPreparationStage::Acknowledging {
                            fs::write(&paths.worker, b"sink ACK trust drift").unwrap();
                        }
                        Ok(())
                    })
                };
                let failure = result.unwrap_err();
                match kind {
                    0 => assert!(matches!(failure.error(), OriginalSinkError::Conflict)),
                    1 => assert!(
                        matches!(failure.error(), OriginalSinkError::Storage(_)),
                        "{failure:?}"
                    ),
                    3 => assert!(matches!(failure.error(), OriginalSinkError::Original(_))),
                    2 => assert!(matches!(failure.error(), OriginalSinkError::Privilege)),
                    _ => unreachable!(),
                }
                assert!(failure.is_refused());
                if kind == 1 || kind == 2 {
                    assert!(!output(leaf).exists());
                }
                busy(root);
                drop(failure);
                retained_context_refuses_for_test(root);
                println!("HC074-SINK-ENTRY kind={kind} held=true refused=true");
            });
        });
    }
    with_original_at(false, None, |original, _, receiver, sender, model| {
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let framed = accepted(original, fence, leaf, receiver, sender, model);
            let mut owner = framed
                .prepare_original_outputs_for_test(|_| Ok(()))
                .unwrap();
            owner.revalidate().unwrap();
            inventory_at_cut(leaf, 9);
            busy(root);
            owner.drop_eof_for_test();
            retained_context_refuses_for_test(root);
            inventory_at_cut(leaf, 9);
            println!("HC074-SINK-CONTROL original=true empty=true drop_eof=true execution=false");
        });
    });
}
