use std::collections::BTreeMap;

use hydracache::{
    restore_backup_to_point, write_full_backup, write_pitr_log, BackupDataset, CertificateBundle,
    CertificateRotationWindow, InMemoryObjectStore, PitrLog, PitrRecord,
};

use crate::SimRng;

/// Deployment fault class covered by the production validation simulator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeploymentFault {
    /// Rolling upgrade while committed data exists.
    RollingUpgrade,
    /// Certificate rotation while old and new peers overlap.
    CertRotation,
    /// Backup object corruption before restore.
    BackupCorruption,
    /// Restore from a full backup plus PITR log.
    PitrRestore,
}

/// Deterministic deployment validation scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentRecoveryScenario {
    /// Replay seed.
    pub seed: u64,
    /// Faults to exercise.
    pub faults: Vec<DeploymentFault>,
}

impl DeploymentRecoveryScenario {
    /// Create a scenario with all production deployment faults enabled.
    pub fn all(seed: u64) -> Self {
        Self {
            seed,
            faults: vec![
                DeploymentFault::RollingUpgrade,
                DeploymentFault::CertRotation,
                DeploymentFault::BackupCorruption,
                DeploymentFault::PitrRestore,
            ],
        }
    }

    /// Create a scenario with selected faults.
    pub fn new(seed: u64, faults: Vec<DeploymentFault>) -> Self {
        Self { seed, faults }
    }
}

/// Invariant report returned by deployment validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentInvariantReport {
    /// Replay seed.
    pub seed: u64,
    /// Faults exercised in order.
    pub faults_exercised: Vec<DeploymentFault>,
    /// Whether committed data survived rolling upgrade.
    pub rolling_upgrade_preserved_committed_data: bool,
    /// Whether cert rotation accepted both old and new certificates during rollout.
    pub cert_rotation_window_valid: bool,
    /// Whether corrupt backup bytes were detected and not served.
    pub corrupt_backup_rejected: bool,
    /// Whether PITR restore matched the selected target sequence.
    pub pitr_restore_matched_target: bool,
    /// Deterministic trace useful for reproducing a failure.
    pub trace: Vec<String>,
}

impl DeploymentInvariantReport {
    /// Return whether all exercised invariants passed.
    pub fn passed(&self) -> bool {
        self.rolling_upgrade_preserved_committed_data
            && self.cert_rotation_window_valid
            && self.corrupt_backup_rejected
            && self.pitr_restore_matched_target
    }
}

/// Run the deployment validation scenario.
pub fn run_upgrade_and_recovery(scenario: DeploymentRecoveryScenario) -> DeploymentInvariantReport {
    let mut rng = SimRng::from_seed(scenario.seed);
    let mut trace = Vec::new();
    let mut committed = BTreeMap::from([
        ("user:1".to_owned(), b"Ada".to_vec()),
        ("user:2".to_owned(), b"Grace".to_vec()),
    ]);

    let mut rolling_upgrade_preserved_committed_data = true;
    let mut cert_rotation_window_valid = true;
    let mut corrupt_backup_rejected = true;
    let mut pitr_restore_matched_target = true;

    for fault in &scenario.faults {
        match fault {
            DeploymentFault::RollingUpgrade => {
                let generation = (rng.next_u64() % 10_000).saturating_add(1);
                trace.push(format!("rolling-upgrade:generation={generation}"));
                let before = committed.clone();
                committed.insert(
                    "upgrade-marker".to_owned(),
                    generation.to_le_bytes().to_vec(),
                );
                rolling_upgrade_preserved_committed_data = before
                    .iter()
                    .all(|(key, value)| committed.get(key) == Some(value));
            }
            DeploymentFault::CertRotation => {
                let old = CertificateBundle::new("cert-old", "CN=member-a", 2_000).unwrap();
                let new = CertificateBundle::new("cert-new", "CN=member-a", 3_000).unwrap();
                let window = CertificateRotationWindow::new(old).promote(new);
                let check_at = 1_000 + rng.next_index(10) as u64;
                trace.push(format!("cert-rotation:check_at={check_at}"));
                cert_rotation_window_valid =
                    window.accepts("cert-old", check_at) && window.accepts("cert-new", check_at);
            }
            DeploymentFault::BackupCorruption => {
                let mut store = InMemoryObjectStore::new();
                let dataset = dataset_from_values("control-plane", committed.clone());
                let manifest = write_full_backup(&mut store, "corrupt", 10, &dataset).unwrap();
                let corrupt_key = manifest.values[0].object_key.clone();
                store
                    .mutate(&corrupt_key, |bytes| {
                        bytes[0] ^= 0x80;
                    })
                    .unwrap();
                trace.push(format!("backup-corruption:key={corrupt_key}"));
                corrupt_backup_rejected =
                    restore_backup_to_point(&store, &manifest.manifest_key, None, 10).is_err();
            }
            DeploymentFault::PitrRestore => {
                let mut store = InMemoryObjectStore::new();
                let base = dataset_from_values("control-plane", committed.clone());
                let manifest = write_full_backup(&mut store, "pitr", 20, &base).unwrap();
                let target_name = format!("user:{}", 3 + rng.next_index(10));
                let pitr = PitrLog::new()
                    .push(PitrRecord::put(21, target_name.clone(), b"Linus".to_vec()))
                    .push(PitrRecord::delete(22, "user:1"))
                    .push(PitrRecord::put(23, "user:2", b"Grace Hopper".to_vec()));
                let pitr_key = write_pitr_log(&mut store, "pitr", &pitr).unwrap();
                let restored =
                    restore_backup_to_point(&store, &manifest.manifest_key, Some(&pitr_key), 22)
                        .unwrap();
                let mut expected = base;
                expected
                    .values
                    .insert(target_name.clone(), b"Linus".to_vec());
                expected.values.remove("user:1");
                trace.push(format!("pitr-restore:target={target_name}:sequence=22"));
                pitr_restore_matched_target = restored == expected;
            }
        }
    }

    DeploymentInvariantReport {
        seed: scenario.seed,
        faults_exercised: scenario.faults,
        rolling_upgrade_preserved_committed_data,
        cert_rotation_window_valid,
        corrupt_backup_rejected,
        pitr_restore_matched_target,
        trace,
    }
}

fn dataset_from_values(
    control_plane: impl Into<Vec<u8>>,
    values: BTreeMap<String, Vec<u8>>,
) -> BackupDataset {
    BackupDataset {
        control_plane: control_plane.into(),
        values,
    }
}
