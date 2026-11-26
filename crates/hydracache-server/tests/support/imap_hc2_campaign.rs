use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Read, Write};

use serde::Serialize;

use super::*;

pub(super) const WINDOW_INTERVAL: Duration = Duration::from_millis(100);

pub(super) struct Profile {
    pub duration: Duration,
    evidence: Option<PathBuf>,
}

#[derive(Debug, Default, Serialize)]
struct Round {
    sequence: u64,
    elapsed_ms: u128,
    put_value_sha256: String,
    cas_value_sha256: String,
    put_ack: bool,
    cas_ack: bool,
    delete_ack: bool,
    unknown: u64,
}

#[derive(Debug, Default, Serialize)]
pub(super) struct Counts {
    pub rounds: u64,
    pub acknowledgements: u64,
    pub verified: u64,
    pub unknown: u64,
}

#[derive(Serialize)]
pub(super) struct Fault {
    pub killed_pid: u32,
    pub killed_at_ms: u64,
    pub recovered_at_ms: u64,
    pub rejoined_pid: u32,
    pub rejoined_at_ms: u64,
}

impl Profile {
    pub fn from_env() -> Self {
        let (duration, evidence) = match std::env::var_os("HYDRACACHE_W10I_RUN_DIR") {
            Some(path) => {
                let seconds: u64 = std::env::var("HYDRACACHE_W10I_DURATION_SECONDS")
                    .unwrap()
                    .parse()
                    .unwrap();
                assert!((10..=60).contains(&seconds) || matches!(seconds, 21600 | 86400));
                if seconds == 86400 {
                    let expected_daemon =
                        std::env::var("HYDRACACHE_W10I_EXPECTED_DAEMON_SHA256").unwrap();
                    let expected_runner =
                        std::env::var("HYDRACACHE_W10I_EXPECTED_RUNNER_SHA256").unwrap();
                    assert_eq!(
                        expected_daemon,
                        digest_file(Path::new(env!("CARGO_BIN_EXE_hydracache-server"))).unwrap(),
                        "daemon changed since the six-hour run"
                    );
                    assert_eq!(
                        expected_runner,
                        digest_file(&std::env::current_exe().unwrap()).unwrap(),
                        "runner changed since the six-hour run"
                    );
                }
                (Duration::from_secs(seconds), Some(PathBuf::from(path)))
            }
            None => (Duration::from_secs(5), None),
        };
        Self { duration, evidence }
    }

    pub fn ledger(&self) -> io::Result<Option<BufWriter<File>>> {
        self.evidence
            .as_ref()
            .map(|root| {
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(root.join("ledger.jsonl"))
                    .map(BufWriter::new)
            })
            .transpose()
    }

    pub fn receipt(
        &self,
        started: tokio::time::Instant,
        counts: &Counts,
        fault: Fault,
    ) -> io::Result<()> {
        let Some(root) = &self.evidence else {
            return Ok(());
        };
        let receipt = serde_json::json!({
            "schema_version": 1,
            "production_admitted": false,
            "source_sha": std::env::var("HYDRACACHE_W10I_SOURCE_SHA").unwrap(),
            "seconds": self.duration.as_secs(),
            "elapsed_ms": started.elapsed().as_millis(),
            "daemon_sha256": digest_file(Path::new(env!("CARGO_BIN_EXE_hydracache-server")))?,
            "runner_sha256": digest_file(&std::env::current_exe()?)?,
            "ledger_sha256": digest_file(&root.join("ledger.jsonl"))?,
            "fault": fault,
            "counts": counts
        });
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join("worker.json"))?;
        serde_json::to_writer(&mut file, &receipt)?;
        file.sync_all()
    }
}

fn digest_file(path: &Path) -> io::Result<String> {
    let mut hash = Sha256::new();
    let mut file = File::open(path)?;
    let mut buffer = [0; 16 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(hex_digest(&hash.finalize()))
}

pub(super) async fn exercise_control(
    writer: Hc2Client,
    reader: Hc2Client,
    key: Bytes,
    start: tokio::time::Instant,
    duration: Duration,
    mut ledger: Option<BufWriter<File>>,
) -> Counts {
    let mut counts = Counts::default();
    while start.elapsed() < duration {
        let seq = counts.rounds;
        let value = Bytes::from(format!("control-put-{seq}"));
        let replacement = Bytes::from(format!("control-cas-{seq}"));
        let mut round = Round {
            sequence: seq,
            elapsed_ms: start.elapsed().as_millis(),
            put_value_sha256: sha256_hex(&value),
            cas_value_sha256: sha256_hex(&replacement),
            ..Round::default()
        };
        round.put_ack = acknowledged(
            writer.put(key.clone(), value.clone(), None, None).await,
            &mut round.unknown,
        );
        if round.put_ack {
            wait_w10i_get(&reader, key.clone(), Some(&value)).await;
            round.cas_ack = acknowledged(
                writer
                    .compare_and_set(key.clone(), value, replacement.clone(), None, None)
                    .await,
                &mut round.unknown,
            );
            if round.cas_ack {
                wait_w10i_get(&reader, key.clone(), Some(&replacement)).await;
                round.delete_ack =
                    acknowledged(reader.delete(key.clone(), None).await, &mut round.unknown);
                if round.delete_ack {
                    wait_w10i_get(&writer, key.clone(), None).await;
                }
            }
        }
        let verified =
            u64::from(round.put_ack) + u64::from(round.cas_ack) + u64::from(round.delete_ack);
        counts.rounds += 1;
        counts.acknowledgements += verified;
        counts.verified += verified;
        counts.unknown += round.unknown;
        if let Some(file) = ledger.as_mut() {
            serde_json::to_writer(&mut *file, &round).unwrap();
            file.write_all(b"\n").unwrap();
        }
        tokio::time::sleep(WINDOW_INTERVAL).await;
    }
    if let Some(file) = ledger.as_mut() {
        file.flush().unwrap();
        file.get_ref().sync_all().unwrap();
    }
    counts
}

fn acknowledged(
    result: Result<hydracache_client_hc2::MutationResult, hydracache_client_hc2::ClientError>,
    unknown: &mut u64,
) -> bool {
    match result {
        Ok(reply) => {
            assert!(
                reply.applied,
                "the uncontended control partition refused an exact mutation"
            );
            true
        }
        Err(error)
            if matches!(
                error.code(),
                ErrorCode::Unavailable | ErrorCode::DeadlineExceeded
            ) =>
        {
            *unknown += 1;
            false
        }
        Err(error) => panic!("control operation failed definitively: {error}"),
    }
}
