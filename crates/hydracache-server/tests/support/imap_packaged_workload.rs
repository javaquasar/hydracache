use super::*;
use hydracache_client_hc2::{BatchItemState, BatchOperation};
use std::ffi::OsStr;
use std::fs::File;
use std::ops::Range;

#[path = "imap_packaged_ledger.rs"]
mod ledger;

const SDKS: [&str; 2] = ["rust", "java"];
const PLANNED_ROUNDS: usize = 360;

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
        for index in 0..SDKS.len() {
            let name = format!("{}-{}.seen", SDKS[index], cut.label);
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

struct KeyPlan {
    text: String,
    partitions: Vec<[u32; 2]>,
}

pub(super) struct PackagedWorkload<'a> {
    commands: &'a PackageCommands<'a>,
    children: [ChildGuard; 2],
    plans: [KeyPlan; 2],
    checkpoints: Vec<serde_json::Value>,
}

impl<'a> PackagedWorkload<'a> {
    pub(super) async fn start(
        commands: &'a PackageCommands<'a>,
        cluster: &MemberCluster,
        control: hydracache::PartitionId,
        started: tokio::time::Instant,
    ) -> Self {
        use std::fmt::Write as _;
        let scope = fixture_imap_scope();
        let plans = SDKS.map(|sdk| {
            let mut plan = KeyPlan {
                text: String::new(),
                partitions: Vec::with_capacity(PLANNED_ROUNDS),
            };
            for round in 0..PLANNED_ROUNDS {
                let keys = ["a", "b"].map(|side| {
                    cluster.pick_key(
                        1,
                        Some(2),
                        |n| format!("w10i-load/{sdk}/{round}/{side}/{n}").into_bytes(),
                        |p| p != control,
                    )
                });
                writeln!(
                    plan.text,
                    "{}\t{}",
                    hex_digest(&keys[0]),
                    hex_digest(&keys[1])
                )
                .unwrap();
                plan.partitions
                    .push(keys.map(|key| scope.locate(key.to_vec()).unwrap().0.value()));
            }
            std::fs::write(commands.root.join(format!("{sdk}-load.plan")), &plan.text).unwrap();
            plan
        });
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
            plans,
            checkpoints: Vec::new(),
        }
    }

    pub(super) async fn checkpoint(&mut self, label: &str, reader: &Hc2Client) {
        let mut entries = Vec::new();
        for (index, sdk) in SDKS.iter().enumerate() {
            let body = self.body(sdk);
            let rounds = self.replay(index, &body, false);
            let first = rounds
                .iter()
                .find(|round| round.complete())
                .expect("packaged SDK has no confirmed round at checkpoint");
            let last = rounds.iter().rfind(|round| round.complete()).unwrap();
            let selected = if first.keys == last.keys {
                vec![first]
            } else {
                vec![first, last]
            };
            reconcile(reader, selected.iter().copied()).await;
            entries.push(serde_json::json!({"sdk": sdk, "keys": selected.len() * 2, "last_completed_ms": last.end}));
        }
        println!("W10i packaged workload checkpoint {label}: {entries:?}");
        self.checkpoints
            .push(serde_json::json!({"label": label, "verified": entries}));
    }

    pub(super) async fn finish(
        mut self,
        reader: &Hc2Client,
        outage: &Range<u64>,
        held: &Range<u64>,
        returned: &Range<u64>,
        watermarks: &[[u64; 2]],
    ) {
        File::create(self.commands.root.join("packaged-load.stop")).unwrap();
        let mut summaries = Vec::new();
        for (index, sdk) in SDKS.iter().enumerate() {
            let status = super::child_exit(&mut self.children[index].0).await;
            let log = std::fs::read_to_string(self.commands.root.join(format!("{sdk}-load.log")))
                .unwrap();
            assert!(status.success(), "packaged {sdk} load failed: {log}");
            print!("{log}");
            let body = self.body(sdk);
            let rounds = self.replay(index, &body, true);
            assert!(!rounds.is_empty());
            let first = &rounds[0];
            let last = rounds.last().unwrap();
            assert!(
                last.end + 1_000 >= returned.end
                    && last.end <= returned.end + 10_000
                    && first.start < outage.start
            );
            assert!(rounds
                .windows(2)
                .all(|pair| pair[1].start - pair[0].end <= 1_000));
            let [offline, copying, post] = [outage, held, returned].map(|window| {
                rounds
                    .iter()
                    .filter(|round| round.in_window(window))
                    .count()
            });
            assert!(
                offline > 0 && copying > 0 && post > 0,
                "{sdk} lacked complete rounds during a fault or after rejoin"
            );
            reconcile(reader, rounds.iter()).await;
            let counts = rounds
                .iter()
                .fold(ledger::Counts::default(), |mut all, round| {
                    all.confirmed += round.counts.confirmed;
                    all.rejected += round.counts.rejected;
                    all.not_applied += round.counts.not_applied;
                    all.unknown += round.counts.unknown;
                    all
                });
            summaries.push(serde_json::json!({"sdk": sdk, "counts": counts, "rounds": rounds.len(), "keys": rounds.len() * 2,
                "offline_rounds": offline, "held_rounds": copying, "post_return_rounds": post, "ledger_sha256": sha256_hex(body.as_bytes())}));
        }
        let report = serde_json::json!({"schema_version": 1, "sdk": summaries, "checkpoints": self.checkpoints,
            "watermarks": watermarks, "outage": outage, "held": held, "returned": returned});
        let file = File::create(self.commands.root.join("packaged-workload.json")).unwrap();
        serde_json::to_writer_pretty(file, &report).unwrap();
        println!("W10i packaged load reconciled: {report}");
    }

    fn body(&self, sdk: &str) -> String {
        std::fs::read_to_string(self.commands.root.join(format!("{sdk}-load.tsv"))).unwrap()
    }

    fn replay<'b>(&self, index: usize, body: &'b str, sealed: bool) -> Vec<ledger::Round<'b>> {
        ledger::replay(
            body,
            &self.plans[index].text,
            &self.plans[index].partitions,
            sealed,
        )
        .unwrap_or_else(|error| panic!("{} load ledger: {error}", SDKS[index]))
    }
}

async fn reconcile<'a>(reader: &Hc2Client, rounds: impl Iterator<Item = &'a ledger::Round<'a>>) {
    let keys = rounds
        .flat_map(|round| round.keys.into_iter().zip(round.allowed))
        .collect::<Vec<_>>();
    for chunk in keys.chunks(16) {
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
            assert_eq!(outcome.receipt.unwrap().state, BatchItemState::Confirmed);
            let value = outcome.result.unwrap().value;
            assert!(
                ledger::admits(*allowed, value.as_ref().map(|value| value.value.as_ref())),
                "independent readback for {key} conflicts with packaged load history"
            );
        }
    }
}
