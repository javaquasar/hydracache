use super::*;
use imap_hc2_campaign::{exercise_control, exercise_live_drain, owner_checkpoint};
use std::ffi::OsStr;
use std::ops::Range;

#[path = "imap_packaged_workload.rs"]
mod packaged_workload;
use packaged_workload::{PackageCommands, PackagedWorkload, WatchCut};

const PROFILE: &str = "HYDRACACHE_RUN_W10I_SDK_COHORT";
const MAP_NAME: &[u8] = b"w10i-sdk-cohort";

#[derive(PartialEq, Eq)]
enum Scenario {
    Correct,
    InterruptedCopy,
    WrongRust,
    WrongJava,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn w10i_packaged_rust_and_java_reconcile_retained_cohort() {
    run_cohort(Scenario::Correct).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn w10i_packaged_sdks_read_during_interrupted_copy_and_after_cutover() {
    run_cohort(Scenario::InterruptedCopy).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn w10i_packaged_java_rejects_a_divergent_rust_value() {
    run_cohort(Scenario::WrongRust).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn w10i_packaged_rust_rejects_a_divergent_java_value() {
    run_cohort(Scenario::WrongJava).await;
}

async fn run_cohort(scenario: Scenario) {
    if std::env::var(PROFILE).as_deref() != Ok("1") {
        return;
    }
    let java = std::env::var_os("HYDRACACHE_W10G_JAVA").expect("SDK cohort requires java");
    let classpath = std::env::var_os("HYDRACACHE_W10G_JAVA_CLASSPATH")
        .expect("SDK cohort requires a packaged Java classpath");
    let executable = std::env::var_os("HYDRACACHE_W10I_RUST_CONSUMER")
        .expect("SDK cohort requires an external packaged Rust consumer");
    let package = std::env::var("HYDRACACHE_W10I_RUST_PACKAGE_SHA256")
        .expect("SDK cohort requires the Rust package digest");
    assert!(package.len() == 64 && package.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let _guard = async_daemon_process_guard().await;
    let credentials = pki();
    let interrupt_copy = scenario == Scenario::InterruptedCopy;
    let duration = Duration::from_secs(if interrupt_copy { 60 } else { 30 });
    let mut cluster = MemberCluster::start_fixture(
        "w10i-sdk-cohort",
        &credentials,
        MemberOptions {
            interrupt_copy,
            ..Default::default()
        },
    )
    .await;
    let root = cluster.spec.root.clone();
    let root = &root;
    let rust = RustProbe {
        executable: &executable,
        package: &package,
        root,
    };
    std::fs::write(root.join("java-client.pem"), credentials.client_cert).unwrap();
    std::fs::write(root.join("java-client.key"), credentials.client_key).unwrap();
    let mut prefix = b"HC\x02\x01".to_vec();
    prefix.extend(u16::try_from(MAP_NAME.len()).unwrap().to_be_bytes());
    prefix.extend(MAP_NAME);
    let control = cluster.key_for(1, Some(2), "w10i-sdk-control/");
    let scope = fixture_imap_scope();
    let control_partition = scope.locate(control.to_vec()).unwrap().0;
    let pick = |label: &str, primary, backup| {
        cluster.pick_key(
            primary,
            Some(backup),
            |index| {
                let mut physical = prefix.clone();
                physical.extend(format!("{label}/{index}").into_bytes());
                physical
            },
            |partition| partition != control_partition,
        )
    };
    let hot = pick("faulted", 0, 1);
    let keys: [[Bytes; 4]; 3] = std::array::from_fn(|phase| {
        std::array::from_fn(|index| pick(&format!("phase-{phase}-{index}"), 1, 2))
    });
    let logical = |key: &[u8]| {
        std::str::from_utf8(&key[prefix.len()..])
            .unwrap()
            .to_owned()
    };
    let expected_value = |sdk, key: &[u8]| Bytes::from(format!("{sdk}:{}", logical(key)));
    let log_path = root.join("java-probe.log");
    let log = std::fs::File::create(&log_path).unwrap();
    let mut probe = ChildGuard(
        Command::new(&java)
            .args(["-XX:ActiveProcessorCount=2", "-cp"])
            .arg(&classpath)
            .arg("io.hydracache.hazelcast.W10iPackagedCohortProbe")
            .arg(root)
            .args(cluster.spec.addresses[9..].iter().map(ToString::to_string))
            .arg(logical(&hot))
            .arg(if scenario == Scenario::WrongJava {
                "wrong-java"
            } else {
                "correct"
            })
            .args(keys.iter().flatten().map(|key| logical(key)))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap(),
    );
    await_java_marker(&mut probe.0, root, "cohort-ready").await;

    let started = tokio::time::Instant::now();
    let ledger = root.join("control-ledger.jsonl");
    let load = matches!(scenario, Scenario::Correct | Scenario::InterruptedCopy).then(|| {
        tokio::spawn(exercise_control(
            cluster.clients[2].clone(),
            cluster.clients[1].clone(),
            control,
            started,
            duration,
            Some(std::io::BufWriter::new(
                std::fs::File::create(&ledger).unwrap(),
            )),
            true,
        ))
    });
    let hot_partition = scope.locate(hot.to_vec()).unwrap().0;
    let partition = hot_partition.value();
    let http = w10h_http();
    let mut expected = Vec::new();
    let mut promoted_epoch = None;
    let mut outage = 0..0;
    let killed = cluster.daemons[0].0.id();
    let commands = PackageCommands {
        root,
        rust: &executable,
        java: &java,
        classpath: &classpath,
    };
    let witness = cluster.pick_key(
        1,
        Some(2),
        |n| format!("w10i-watch/{n}").into_bytes(),
        |p| p != control_partition,
    );
    let mut package_load = if interrupt_copy {
        let mut reserved = vec![
            control_partition,
            hot_partition,
            scope.locate(witness.to_vec()).unwrap().0,
        ];
        reserved.extend(
            keys.iter()
                .flatten()
                .map(|key| scope.locate(key.to_vec()).unwrap().0),
        );
        Some(PackagedWorkload::start(&commands, &cluster, &reserved, started).await)
    } else {
        None
    };
    let mut watermarks = Vec::new();
    let mut watcher = if interrupt_copy {
        Some(
            commands
                .watch(
                    WatchCut {
                        label: "before-kill",
                        address: cluster.spec.addresses[9],
                        resume: [0; 2],
                        previous: "-",
                        hold: true,
                    },
                    &witness,
                    &cluster.clients[2],
                )
                .await,
        )
    } else {
        None
    };
    for (phase, [rust_first, rust_second, java_first, java_second]) in keys.into_iter().enumerate()
    {
        let seed = [rust_first, rust_second].map(|key| {
            let value = if scenario == Scenario::WrongRust {
                Bytes::from_static(b"intentional-wrong-value")
            } else {
                expected_value("rust", &key)
            };
            (key, value)
        });
        let put = rust
            .run(phase, "put", cluster.spec.addresses[11], &seed)
            .await;
        rust.require(put, "put", seed.len());
        std::fs::File::create(root.join(format!("cohort-start-{phase}"))).unwrap();
        let complete = format!("cohort-done-{phase}");
        if scenario == Scenario::WrongRust {
            assert!(!child_exit(&mut probe.0).await.success());
            assert!(!root.join(&complete).exists());
            let log = std::fs::read_to_string(&log_path).unwrap();
            assert!(log.contains("wrong value for phase-0-"), "{log}");
            println!("W10i Java cohort rejected an incorrect Rust value");
            return;
        }
        let java_seeded = format!("cohort-java-seeded-{phase}");
        await_java_marker(&mut probe.0, root, &java_seeded).await;
        let java_seed = [java_first, java_second].map(|key| {
            let value = expected_value("java", &key);
            (key, value)
        });
        let hot_seed = (phase == 0).then(|| (hot.clone(), expected_value("java", &hot)));
        let seeded: Vec<_> = seed
            .iter()
            .chain(&java_seed)
            .chain(&hot_seed)
            .cloned()
            .collect();
        let result = rust
            .run(phase, "check", cluster.spec.addresses[10], &seeded)
            .await;
        if scenario == Scenario::WrongJava {
            let (status, log) = result;
            assert!(!status.success());
            assert!(
                log.contains("wrong value for Rust cohort item 2")
                    && !log.contains("action=check confirmed="),
                "{log}"
            );
            println!("W10i Rust cohort rejected an incorrect Java value");
            return;
        }
        rust.require(result, "check", seeded.len());
        let mixed = rust
            .run(phase, "mixed", cluster.spec.addresses[11], &java_seed)
            .await;
        rust.require(mixed, "mixed", 5);
        std::fs::File::create(root.join(format!("cohort-rust-mixed-{phase}"))).unwrap();
        for [(update, original), (deleted, _)] in [seed, java_seed] {
            let mut replacement = b"updated:".to_vec();
            replacement.extend_from_slice(&original);
            expected.push((update, Some(Bytes::from(replacement))));
            expected.push((deleted, None));
        }
        expected.extend(hot_seed.map(|(key, value)| (key, Some(value))));
        await_java_marker(&mut probe.0, root, &complete).await;
        let label = phase.to_string();
        let manifest = readback_manifest(root, &label, &expected);
        let reader = if phase == 2 { 9 } else { 10 };
        let reconciled = rust
            .invoke(
                &label,
                "check-file",
                cluster.spec.addresses[reader],
                [manifest],
            )
            .await;
        rust.require(reconciled, "check-file", expected.len());
        if phase == 0 {
            let original = owner_checkpoint(&http, &cluster.spec, [0, 1], partition, 1, 1).await;
            outage.start = started.elapsed().as_millis() as u64;
            cluster.daemons[0].0.kill().unwrap();
            assert!(!cluster.daemons[0].0.wait().unwrap().success());
            if let Some(before) = watcher.take() {
                watermarks.push(before.finish(&commands).await);
            }
            let promoted = owner_checkpoint(
                &http,
                &cluster.spec,
                [1, 2],
                partition,
                1,
                original.epoch() + 1,
            )
            .await;
            original.assert_same_image(&promoted);
            println!(
                "W10i SDK cohort owner promotion partition={partition} epoch={}->{} version=1",
                original.epoch(),
                promoted.epoch()
            );
            promoted_epoch = Some(promoted.epoch());
            if let Some(workload) = &mut package_load {
                workload.checkpoint("promoted", &cluster.clients[1]).await;
                let survivor = commands
                    .watch(
                        WatchCut {
                            label: "after-kill",
                            address: cluster.spec.addresses[10],
                            resume: *watermarks.last().unwrap(),
                            previous: "before-kill",
                            hold: false,
                        },
                        &witness,
                        &cluster.clients[2],
                    )
                    .await;
                watermarks.push(survivor.finish(&commands).await);
            }
        } else if phase == 1 {
            assert!(cluster.daemons[0].0.try_wait().unwrap().is_some());
            outage.end = started.elapsed().as_millis() as u64;
            cluster.daemons[0] = cluster.spec.spawn(0);
            cluster.clients[0] = connect_w10i(&cluster.adapters[0], "w10i").await;
            assert_ne!(killed, cluster.daemons[0].0.id());
            // Transport readiness can precede agreement on the returning process's generation.
            let prior_epoch = promoted_epoch.unwrap();
            let rejoined = wait_cluster_imap(&http, cluster.spec.addresses[7], |body| {
                body["ready"] == true
                    && cluster_imap_members(body).len() == 3
                    && body["epoch"]
                        .as_u64()
                        .is_some_and(|epoch| epoch > prior_epoch)
            })
            .await;
            println!(
                "W10i SDK cohort authority restored epoch={}",
                rejoined["epoch"]
            );
            if interrupt_copy {
                watcher = Some(
                    commands
                        .watch(
                            WatchCut {
                                label: "before-drain",
                                address: cluster.spec.addresses[9],
                                resume: *watermarks.last().unwrap(),
                                previous: "after-kill",
                                hold: true,
                            },
                            &witness,
                            &cluster.clients[2],
                        )
                        .await,
                );
            }
        }
    }
    let status = child_exit(&mut probe.0).await;
    let log = std::fs::read_to_string(&log_path).unwrap();
    assert!(status.success(), "Java cohort failed: {log}");
    print!("{log}");
    let rejoined = cluster.daemons[0].0.id();
    let drain_checks = if interrupt_copy {
        let addresses = cluster.spec.addresses;
        let mut held_reads = None;
        let reader_during_hold = cluster.clients[1].clone();
        let check_while_held = async {
            package_load
                .as_mut()
                .unwrap()
                .checkpoint("copy-held", &reader_during_hold)
                .await;
            held_reads = Some(
                packaged_readback(
                    &rust,
                    (&java, &classpath),
                    "held",
                    [addresses[10], addresses[11]],
                    &expected,
                    started,
                )
                .await,
            );
        };
        let drain = exercise_live_drain(
            &mut cluster,
            started,
            duration,
            true,
            hot_partition,
            check_while_held,
        )
        .await;
        let held = held_reads.expect("the SDKs did not read while the copy was held");
        let hold = drain.interrupted_window().unwrap();
        assert!(hold.start <= held.start && held.end <= hold.end);
        watermarks.push(watcher.take().unwrap().finish(&commands).await);
        package_load
            .as_mut()
            .unwrap()
            .checkpoint("after-drain", &cluster.clients[1])
            .await;
        let survivor = commands
            .watch(
                WatchCut {
                    label: "after-drain",
                    address: addresses[10],
                    resume: *watermarks.last().unwrap(),
                    previous: "before-drain",
                    hold: false,
                },
                &witness,
                &cluster.clients[2],
            )
            .await;
        watermarks.push(survivor.finish(&commands).await);
        let mut final_values = expected.clone();
        final_values.push((drain.probe.0.clone(), Some(drain.probe.1.clone())));
        let returned = packaged_readback(
            &rust,
            (&java, &classpath),
            "returned",
            [addresses[9]; 2],
            &final_values,
            started,
        )
        .await;
        assert!(drain.returned_at_ms <= returned.start);
        Some((drain, held, returned))
    } else {
        None
    };
    let load = load.unwrap();
    assert!(
        !load.is_finished(),
        "control load ended before cohort verification"
    );
    let counts = load.await.unwrap();
    let rows: Vec<_> = std::io::BufReader::new(std::fs::File::open(ledger).unwrap())
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(&line.unwrap()).unwrap())
        .collect();
    let verified_while_offline = verified_control(&rows, &outage);
    assert!(
        verified_while_offline > 0,
        "no verified control round during the owner outage"
    );
    println!("W10i SDK cohort retained={} killed_pid={killed} rejoined_pid={rejoined} control_verified={} control_verified_offline={verified_while_offline} offline_from_ms={} offline_to_ms={}", expected.len(), counts.verified, outage.start, outage.end);
    if let Some((drain, held, returned)) = drain_checks {
        package_load
            .take()
            .unwrap()
            .finish(
                &cluster.clients[1],
                &outage,
                &drain.interrupted_window().unwrap(),
                &(drain.returned_at_ms..duration.as_millis() as u64),
                &watermarks,
            )
            .await;
        let held_progress = verified_control(&rows, &drain.interrupted_window().unwrap());
        let returned_progress =
            verified_control(&rows, &(drain.returned_at_ms..duration.as_millis() as u64));
        assert!(
            held_progress > 0 && returned_progress > 0,
            "missing control progress during the hold or after rejoin"
        );
        let report = serde_json::json!({
            "drain": drain, "held_readback": held, "returned_readback": returned,
            "held_verified": held_progress, "returned_verified": returned_progress,
            "counts": counts,
        });
        serde_json::to_writer_pretty(
            std::fs::File::create(root.join("sdk-drain.json")).unwrap(),
            &report,
        )
        .unwrap();
        println!("W10i packaged SDK drain retained=13 migrated=1 held_verified={held_progress} returned_verified={returned_progress}");
    }
}

fn verified_control(rows: &[serde_json::Value], window: &Range<u64>) -> usize {
    rows.iter()
        .filter(|row| {
            window.contains(&row["elapsed_ms"].as_u64().unwrap())
                && window.contains(&row["verified_at_ms"].as_u64().unwrap())
                && row["put_ack"] == true
                && row["cas_ack"] == true
                && row["delete_ack"] == true
        })
        .count()
        * 3
}

async fn packaged_readback(
    rust: &RustProbe<'_>,
    java: (&OsStr, &OsStr),
    label: &str,
    [rust_address, java_address]: [SocketAddr; 2],
    rows: &[(Bytes, Option<Bytes>)],
    started: tokio::time::Instant,
) -> Range<u64> {
    let start = started.elapsed().as_millis() as u64;
    let file = readback_manifest(rust.root, label, rows);
    let (exit, java_log) = tokio::join!(
        rust.invoke(label, "check-file", rust_address, [file.clone()]),
        async {
            let path = rust.root.join(format!("java-check-{label}.log"));
            let log = std::fs::File::create(&path).unwrap();
            let mut child = ChildGuard(
                Command::new(java.0)
                    .args(["-XX:ActiveProcessorCount=2", "-cp"])
                    .arg(java.1)
                    .arg("io.hydracache.hazelcast.W10iPackagedCohortProbe")
                    .arg("check-file")
                    .arg(rust.root)
                    .arg(java_address.to_string())
                    .arg(&file)
                    .stdout(Stdio::from(log.try_clone().unwrap()))
                    .stderr(Stdio::from(log))
                    .spawn()
                    .unwrap(),
            );
            let status = child_exit(&mut child.0).await;
            let log = std::fs::read_to_string(path).unwrap();
            assert!(
                status.success(),
                "invalid packaged Java readback: {status}; {log}"
            );
            log
        }
    );
    rust.require(exit, "check-file", rows.len());
    let receipt = format!(
        "W10i Java cohort action=check-file confirmed={}",
        rows.len()
    );
    assert!(java_log.lines().any(|line| line == receipt), "{java_log}");
    print!("{java_log}");
    start..started.elapsed().as_millis() as u64
}

fn readback_manifest(root: &Path, label: &str, rows: &[(Bytes, Option<Bytes>)]) -> String {
    use std::io::Write;

    let file = format!("readback-{label}.tsv");
    let mut manifest = std::io::BufWriter::new(std::fs::File::create(root.join(&file)).unwrap());
    for (key, value) in rows {
        let key = hex_digest(key);
        if let Some(value) = value {
            writeln!(
                manifest,
                "{key}\tpresent\t{}",
                std::str::from_utf8(value).unwrap()
            )
            .unwrap();
        } else {
            writeln!(manifest, "{key}\tabsent").unwrap();
        }
    }
    manifest.flush().unwrap();
    file
}

struct RustProbe<'a> {
    executable: &'a OsStr,
    package: &'a str,
    root: &'a Path,
}

impl RustProbe<'_> {
    async fn run(
        &self,
        phase: usize,
        action: &str,
        address: SocketAddr,
        rows: &[(Bytes, Bytes)],
    ) -> (std::process::ExitStatus, String) {
        let args = rows.iter().flat_map(|(key, value)| {
            [
                hex_digest(key),
                std::str::from_utf8(value).unwrap().to_owned(),
            ]
        });
        self.invoke(&phase.to_string(), action, address, args).await
    }

    async fn invoke(
        &self,
        label: &str,
        action: &str,
        address: SocketAddr,
        args: impl IntoIterator<Item = String>,
    ) -> (std::process::ExitStatus, String) {
        let path = self.root.join(format!("rust-{label}-{action}.log"));
        let log = std::fs::File::create(&path).unwrap();
        let mut child = ChildGuard(
            Command::new(self.executable)
                .arg(action)
                .arg(self.root)
                .arg(address.to_string())
                .args(args)
                .stdout(Stdio::from(log.try_clone().unwrap()))
                .stderr(Stdio::from(log))
                .spawn()
                .expect("start packaged Rust probe"),
        );
        (
            child_exit(&mut child.0).await,
            std::fs::read_to_string(path).unwrap(),
        )
    }

    fn require(
        &self,
        (status, log): (std::process::ExitStatus, String),
        action: &str,
        count: usize,
    ) {
        let receipt = format!(
            "W10i Rust package sha256={} action={action} confirmed={count}",
            self.package
        );
        assert!(
            status.success() && log.lines().any(|line| line == receipt),
            "invalid packaged Rust receipt: {status}; {log}"
        );
        print!("{log}");
    }
}

async fn child_exit(probe: &mut Child) -> std::process::ExitStatus {
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if let Some(status) = probe.try_wait().unwrap() {
                return status;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("SDK cohort process did not exit")
}
