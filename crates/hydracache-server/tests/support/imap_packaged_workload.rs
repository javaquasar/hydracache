use super::*;
use hydracache_client_hc2::{BatchItemState, BatchOperation};
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

use hydracache_cluster_testkit::w10i_packaged_ledger as ledger;

const SDKS: [&str; 2] = ["rust", "java"];
const AUDIT_CHUNK: usize = 256;

pub(super) struct PackageCommands<'a> {
    pub root: &'a Path,
    pub rust: &'a OsStr,
    pub java: &'a OsStr,
    pub classpath: &'a OsStr,
}

impl PackageCommands<'_> {
    fn spawn(
        &self,
        sdk: &str,
        action: &str,
        address: SocketAddr,
        label: &str,
        args: &[String],
    ) -> ChildGuard {
        let mut cmd = if sdk == "rust" {
            Command::new(self.rust)
        } else {
            let mut cmd = Command::new(self.java);
            cmd.args(["-XX:ActiveProcessorCount=2", "-cp"])
                .arg(self.classpath)
                .arg("io.hydracache.hazelcast.W10iPackagedLoadProbe");
            cmd
        };
        let path = self.root.join(format!("{sdk}-{label}.log"));
        let log = File::create(path).unwrap();
        ChildGuard(
            cmd.arg(action)
                .arg(self.root)
                .arg(address.to_string())
                .args(args)
                .stdout(Stdio::from(log.try_clone().unwrap()))
                .stderr(Stdio::from(log))
                .spawn()
                .unwrap(),
        )
    }

    pub(super) async fn watch(
        &self,
        cut: WatchCut<'_>,
        key: &Bytes,
        writer: &Hc2Client,
    ) -> WatchPair {
        let encoded = hex_digest(key);
        let children = std::array::from_fn(|index| {
            self.spawn(
                SDKS[index],
                "watch",
                cut.address,
                cut.label,
                &[
                    cut.label.to_owned(),
                    encoded.clone(),
                    cut.resume[index].to_string(),
                    cut.previous.to_owned(),
                    cut.label.to_owned(),
                    if cut.hold { "hold" } else { "close" }.into(),
                ],
            )
        });
        let mut pair = WatchPair {
            children,
            label: cut.label.to_string(),
            marks: [0; 2],
            held: cut.hold,
        };
        for (sdk, child) in SDKS.iter().zip(&mut pair.children) {
            await_java_marker(
                &mut child.0,
                self.root,
                &format!("{sdk}-{}.ready", cut.label),
            )
            .await;
        }
        let value = Bytes::copy_from_slice(cut.label.as_bytes());
        assert!(
            retry_w10i(|| writer.put(key.clone(), value.clone(), None, None))
                .await
                .applied
        );
        for (index, sdk) in SDKS.iter().enumerate() {
            let name = format!("{sdk}-{}.seen", cut.label);
            let child = &mut pair.children[index].0;
            await_java_marker(child, self.root, &name).await;
            pair.marks[index] = std::fs::read_to_string(self.root.join(name))
                .unwrap()
                .parse()
                .unwrap();
            assert!(pair.marks[index] > cut.resume[index]);
        }
        pair
    }
}

pub(super) struct WatchCut<'a> {
    pub label: &'a str,
    pub address: SocketAddr,
    pub resume: [u64; 2],
    pub previous: &'a str,
    pub hold: bool,
}

pub(super) struct WatchPair {
    children: [ChildGuard; 2],
    label: String,
    marks: [u64; 2],
    held: bool,
}

impl WatchPair {
    pub(super) async fn finish(mut self, tools: &PackageCommands<'_>) -> [u64; 2] {
        if self.held {
            assert!(self
                .children
                .iter_mut()
                .all(|child| child.0.try_wait().unwrap().is_none()));
            File::create(tools.root.join(format!("watch-{}.release", self.label))).unwrap();
        }
        for (sdk, child) in SDKS.iter().zip(&mut self.children) {
            let status = super::child_exit(&mut child.0).await;
            let log = std::fs::read_to_string(tools.root.join(format!("{sdk}-{}.log", self.label)))
                .unwrap();
            assert!(status.success(), "packaged {sdk} subscriber failed: {log}");
            print!("{log}");
        }
        self.marks
    }
}

pub(super) fn prepare_plans(
    root: &Path,
    cluster: &MemberCluster,
    reserved: &[hydracache::PartitionId],
    duration: Duration,
) -> usize {
    let rounds = ledger::planned_rounds(duration.as_secs()).expect("valid packaged duration");
    let scope = fixture_imap_scope();
    let members = placement_members(&cluster.spec.names);
    let eligible: Vec<_> = (0..D2_PARTITION_COUNT.get())
        .map(|id| {
            let place = RendezvousClusterOwnership
                .replicas_for_key(&format!("partition-{id}"), &members, 2)
                .unwrap();
            !reserved.contains(&hydracache::PartitionId::new(id))
                && place.primary == members[1].node_id
                && place.backups == [members[2].node_id.clone()]
        })
        .collect();
    assert!(
        eligible.iter().any(|&eligible| eligible),
        "no partition for packaged load"
    );
    for sdk in SDKS {
        let mut plan =
            BufWriter::new(File::create_new(root.join(format!("{sdk}-load.plan"))).unwrap());
        let mut candidates = 0..;
        let mut key = || loop {
            let key = format!("w10i-load/{sdk}/{}", candidates.next().unwrap());
            let (partition, _) = scope.locate(key.as_bytes().to_vec()).unwrap();
            if eligible[partition.value() as usize] {
                break (hex_digest(key.as_bytes()), partition.value());
            }
        };
        for _ in 0..rounds {
            let [(a, pa), (b, pb)] = [key(), key()];
            writeln!(plan, "{a}\t{b}\t{pa}\t{pb}").unwrap();
        }
        plan.flush().unwrap();
    }
    rounds
}

pub(super) struct PackagedWorkload<'a> {
    commands: &'a PackageCommands<'a>,
    children: [ChildGuard; 2],
    limit: usize,
    checkpoints: Vec<serde_json::Value>,
}

impl<'a> PackagedWorkload<'a> {
    pub(super) async fn start(
        commands: &'a PackageCommands<'a>,
        cluster: &MemberCluster,
        limit: usize,
        started: tokio::time::Instant,
    ) -> Self {
        let origin = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            - started.elapsed().as_millis();
        let mut children = SDKS.map(|sdk| {
            commands.spawn(
                sdk,
                "load",
                cluster.spec.addresses[11],
                "load",
                &[origin.to_string()],
            )
        });
        for (sdk, child) in SDKS.iter().zip(&mut children) {
            await_java_marker(&mut child.0, commands.root, &format!("{sdk}-load.ready")).await;
        }
        Self {
            commands,
            children,
            limit,
            checkpoints: Vec::new(),
        }
    }

    pub(super) async fn checkpoint(&mut self, label: &str, reader: &Hc2Client) {
        let mut entries = Vec::new();
        for sdk in SDKS {
            let root = self.commands.root.to_owned();
            let limit = self.limit;
            let selected = tokio::task::spawn_blocking(move || {
                let mut first = None;
                let mut last = None;
                for round in replay(&root, sdk, false, limit) {
                    let round = round.unwrap_or_else(|error| panic!("{sdk} load ledger: {error}"));
                    if round.complete() {
                        if first.is_none() {
                            first = Some(round);
                        } else {
                            last = Some(round);
                        }
                    }
                }
                [first, last].into_iter().flatten().collect::<Vec<_>>()
            })
            .await
            .unwrap();
            let last = selected
                .last()
                .expect("packaged SDK has no confirmed round at checkpoint");
            reconcile(reader, &selected).await;
            entries.push(serde_json::json!({"sdk": sdk, "keys": selected.len() * 2, "last_completed_ms": last.end}));
        }
        println!("W10i packaged workload checkpoint {label}: {entries:?}");
        self.checkpoints
            .push(serde_json::json!({"label": label, "verified": entries}));
    }

    pub(super) async fn finish(
        mut self,
        reader: &Hc2Client,
        windows: &ledger::Windows,
        watermarks: &[[u64; 2]],
    ) {
        File::create_new(self.commands.root.join("packaged-load.stop")).unwrap();
        let mut summaries = Vec::new();
        for (index, sdk) in SDKS.iter().enumerate() {
            let status = super::child_exit(&mut self.children[index].0).await;
            let log = std::fs::read_to_string(self.commands.root.join(format!("{sdk}-load.log")))
                .unwrap();
            assert!(status.success(), "packaged {sdk} load failed: {log}");
            print!("{log}");
            let mut coverage = ledger::Coverage::default();
            let mut chunk = Vec::with_capacity(AUDIT_CHUNK);
            for round in replay(self.commands.root, sdk, true, self.limit) {
                let round = round.unwrap_or_else(|error| panic!("{sdk} load ledger: {error}"));
                coverage
                    .observe(&round, windows)
                    .unwrap_or_else(|error| panic!("{sdk}: {error}"));
                chunk.push(round);
                if chunk.len() == AUDIT_CHUNK {
                    reconcile(reader, &chunk).await;
                    chunk.clear();
                }
            }
            reconcile(reader, &chunk).await;
            let progress = coverage
                .finish(windows)
                .unwrap_or_else(|error| panic!("{sdk}: {error}"));
            let root = self.commands.root;
            summaries.push(serde_json::json!({
                "sdk": sdk, "progress": progress,
                "ledger_sha256": imap_hc2_campaign::digest_file(&root.join(format!("{sdk}-load.tsv"))).unwrap(),
                "plan_sha256": imap_hc2_campaign::digest_file(&root.join(format!("{sdk}-load.plan"))).unwrap(),
            }));
        }
        let report = serde_json::json!({
            "schema_version": 2, "sdk": summaries, "checkpoints": self.checkpoints,
            "watermarks": watermarks, "windows": windows,
        });
        let file = File::create_new(self.commands.root.join("packaged-workload.json")).unwrap();
        serde_json::to_writer_pretty(&file, &report).unwrap();
        file.sync_all().unwrap();
        println!("W10i packaged load reconciled: {report}");
    }
}

fn replay(
    root: &Path,
    sdk: &str,
    sealed: bool,
    limit: usize,
) -> ledger::Replay<BufReader<File>, BufReader<File>> {
    let open =
        |suffix| BufReader::new(File::open(root.join(format!("{sdk}-load.{suffix}"))).unwrap());
    ledger::Replay::new(open("tsv"), open("plan"), sealed, limit, sdk)
}

async fn reconcile(reader: &Hc2Client, rounds: &[ledger::Round]) {
    let keys = rounds
        .iter()
        .flat_map(|round| round.keys.iter().zip(round.allowed))
        .collect::<Vec<_>>();
    for chunk in keys.chunks(64) {
        let outcomes = retry_w10i(|| async {
            let operations = chunk
                .iter()
                .map(|(key, _)| BatchOperation::Get {
                    key: (0..key.len())
                        .step_by(2)
                        .map(|index| u8::from_str_radix(&key[index..index + 2], 16).unwrap())
                        .collect::<Vec<_>>()
                        .into(),
                })
                .collect();
            let outcomes = reader.batch_outcomes(operations, None).await?;
            for outcome in &outcomes {
                outcome.result.as_ref().map_err(Clone::clone)?;
            }
            Ok(outcomes)
        })
        .await;
        assert_eq!(outcomes.len(), chunk.len());
        for (outcome, (key, allowed)) in outcomes.into_iter().zip(chunk) {
            let receipt = outcome.receipt.unwrap();
            assert_eq!(receipt.state, BatchItemState::Confirmed);
            assert!(receipt.applied.is_some());
            let value = outcome.result.unwrap().value;
            assert!(
                ledger::admits(*allowed, value.as_ref().map(|value| value.value.as_ref())),
                "independent readback for {key} conflicts with packaged load history"
            );
        }
    }
}
