use super::*;
use imap_hc2_campaign::{exercise_control, owner_checkpoint};
use std::ffi::OsStr;

const PROFILE: &str = "HYDRACACHE_RUN_W10I_SDK_COHORT";
const MAP_NAME: &[u8] = b"w10i-sdk-cohort";

#[derive(PartialEq, Eq)]
enum Scenario {
    Correct,
    WrongRust,
    WrongJava,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn w10i_packaged_rust_and_java_reconcile_retained_cohort() {
    run_cohort(Scenario::Correct).await;
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
    let mut cluster =
        MemberCluster::start_fixture("w10i-sdk-cohort", &credentials, MemberOptions::default())
            .await;
    let root = &cluster.spec.root;
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
        Command::new(java)
            .args(["-XX:ActiveProcessorCount=2", "-cp"])
            .arg(classpath)
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
    let load = (scenario == Scenario::Correct).then(|| {
        tokio::spawn(exercise_control(
            cluster.clients[2].clone(),
            cluster.clients[1].clone(),
            control,
            started,
            Duration::from_secs(30),
            Some(std::io::BufWriter::new(
                std::fs::File::create(&ledger).unwrap(),
            )),
            true,
        ))
    });
    let partition = scope.locate(hot.to_vec()).unwrap().0.value();
    let http = w10h_http();
    let mut expected = Vec::new();
    let mut promoted_epoch = None;
    let mut outage = 0..0;
    let killed = cluster.daemons[0].0.id();
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
        expected.extend(seed);
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
        await_java_marker(&mut probe.0, root, &complete).await;
        for key in [java_first, java_second] {
            let value = expected_value("java", &key);
            expected.push((key, value));
        }
        if phase == 0 {
            expected.push((hot.clone(), expected_value("java", &hot)));
        }
        let reader = if phase == 2 { 9 } else { 10 };
        let result = rust
            .run(phase, "check", cluster.spec.addresses[reader], &expected)
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
        rust.require(result, "check", expected.len());
        if phase == 0 {
            let original = owner_checkpoint(&http, &cluster.spec, [0, 1], partition, 1, 1).await;
            outage.start = started.elapsed().as_millis() as u64;
            cluster.daemons[0].0.kill().unwrap();
            assert!(!cluster.daemons[0].0.wait().unwrap().success());
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
        }
    }
    let status = child_exit(&mut probe.0).await;
    let log = std::fs::read_to_string(&log_path).unwrap();
    assert!(status.success(), "Java cohort failed: {log}");
    print!("{log}");
    let load = load.unwrap();
    assert!(
        !load.is_finished(),
        "control load ended before cohort verification"
    );
    let counts = load.await.unwrap();
    let verified_while_offline = std::io::BufReader::new(std::fs::File::open(ledger).unwrap())
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(&line.unwrap()).unwrap())
        .filter(|row| {
            outage.contains(&row["elapsed_ms"].as_u64().unwrap())
                && outage.contains(&row["verified_at_ms"].as_u64().unwrap())
                && row["put_ack"] == true
                && row["cas_ack"] == true
                && row["delete_ack"] == true
        })
        .count()
        * 3;
    assert!(
        verified_while_offline > 0,
        "no verified control round during the owner outage"
    );
    println!("W10i SDK cohort retained={} killed_pid={killed} rejoined_pid={} control_verified={} control_verified_offline={verified_while_offline} offline_from_ms={} offline_to_ms={}", expected.len(), cluster.daemons[0].0.id(), counts.verified, outage.start, outage.end);
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
        let path = self.root.join(format!("rust-{phase}-{action}.log"));
        let log = std::fs::File::create(&path).unwrap();
        let mut child = ChildGuard(
            Command::new(self.executable)
                .arg(action)
                .arg(self.root)
                .arg(address.to_string())
                .args(rows.iter().flat_map(|(key, value)| {
                    [
                        hex_digest(key),
                        std::str::from_utf8(value).unwrap().to_owned(),
                    ]
                }))
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
