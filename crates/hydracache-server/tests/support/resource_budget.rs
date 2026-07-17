#![allow(dead_code)]

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub type ResourceBudgetResult<T = ()> = Result<T, Box<dyn Error>>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceSample {
    pub running_children: u64,
    pub tracked_connections: u64,
    pub held_snapshot_messages: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rss_kib: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_fds: Option<u64>,
}

impl ResourceSample {
    fn peak(samples: &[Self]) -> Self {
        Self {
            running_children: samples
                .iter()
                .map(|sample| sample.running_children)
                .max()
                .unwrap_or(0),
            tracked_connections: samples
                .iter()
                .map(|sample| sample.tracked_connections)
                .max()
                .unwrap_or(0),
            held_snapshot_messages: samples
                .iter()
                .map(|sample| sample.held_snapshot_messages)
                .max()
                .unwrap_or(0),
            rss_kib: samples.iter().filter_map(|sample| sample.rss_kib).max(),
            open_fds: samples.iter().filter_map(|sample| sample.open_fds).max(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceBudget {
    pub max_child_delta: u64,
    pub max_connection_delta: u64,
    pub max_held_snapshot_messages: u64,
    pub max_rss_growth_kib: u64,
    pub max_fd_growth: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceBudgetArtifact {
    pub schema_version: u32,
    pub release: String,
    pub seed: u64,
    pub platform: String,
    pub budget: ResourceBudget,
    pub baseline: ResourceSample,
    pub peak: ResourceSample,
    pub final_sample: ResourceSample,
    pub samples: Vec<ResourceSample>,
}

impl ResourceBudgetArtifact {
    pub fn new(
        release: impl Into<String>,
        seed: u64,
        samples: Vec<ResourceSample>,
        budget: ResourceBudget,
    ) -> Self {
        let baseline = samples.first().copied().unwrap_or_default();
        let final_sample = samples.last().copied().unwrap_or_default();
        Self {
            schema_version: 1,
            release: release.into(),
            seed,
            platform: std::env::consts::OS.to_owned(),
            budget,
            baseline,
            peak: ResourceSample::peak(&samples),
            final_sample,
            samples,
        }
    }

    pub fn validate_for_release(&self, expected_release: &str) -> ResourceBudgetResult {
        if self.schema_version != 1 {
            return Err(format!(
                "resource artifact schema_version must be 1, got {}",
                self.schema_version
            )
            .into());
        }
        if self.release != expected_release {
            return Err(format!(
                "resource artifact release must be {expected_release}, got {}",
                self.release
            )
            .into());
        }
        if self.platform.is_empty() {
            return Err("resource artifact platform must not be empty".into());
        }
        let Some(first) = self.samples.first() else {
            return Err("resource artifact must contain at least one sample".into());
        };
        let last = self
            .samples
            .last()
            .expect("a non-empty sample sequence has a last element");
        if self.baseline != *first {
            return Err("resource artifact baseline must equal its first sample".into());
        }
        if self.final_sample != *last {
            return Err("resource artifact final_sample must equal its last sample".into());
        }
        if self.peak != ResourceSample::peak(&self.samples) {
            return Err("resource artifact peak must be derived from all samples".into());
        }
        Ok(())
    }

    pub fn validate_linux_proof(&self) -> ResourceBudgetResult {
        if self.platform != "linux" {
            return Err(format!(
                "resource artifact from platform {} cannot claim Linux /proc proof",
                self.platform
            )
            .into());
        }
        if self
            .samples
            .iter()
            .any(|sample| sample.rss_kib.is_none() || sample.open_fds.is_none())
        {
            return Err(
                "Linux resource proof requires rss_kib and open_fds in every sample".into(),
            );
        }
        Ok(())
    }

    pub fn validate_budget(&self) -> ResourceBudgetResult {
        if self.peak.running_children
            > self
                .baseline
                .running_children
                .saturating_add(self.budget.max_child_delta)
        {
            return Err("running child peak exceeded the declared budget".into());
        }
        if self.peak.tracked_connections
            > self
                .baseline
                .tracked_connections
                .saturating_add(self.budget.max_connection_delta)
        {
            return Err("tracked connection peak exceeded the declared budget".into());
        }
        if self.peak.held_snapshot_messages > self.budget.max_held_snapshot_messages {
            return Err("held snapshot message peak exceeded the declared budget".into());
        }
        if let (Some(baseline), Some(peak)) = (self.baseline.rss_kib, self.peak.rss_kib) {
            if peak > baseline.saturating_add(self.budget.max_rss_growth_kib) {
                return Err("peak RSS exceeded the declared growth budget".into());
            }
        }
        if let (Some(baseline), Some(peak)) = (self.baseline.open_fds, self.peak.open_fds) {
            if peak > baseline.saturating_add(self.budget.max_fd_growth) {
                return Err("peak FD count exceeded the declared growth budget".into());
            }
        }
        Ok(())
    }

    pub fn write_workspace_evidence(
        &self,
        release_directory: impl AsRef<Path>,
        file_name: impl AsRef<Path>,
    ) -> ResourceBudgetResult<PathBuf> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("target/test-evidence")
            .join(release_directory)
            .join(file_name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, serde_json::to_vec_pretty(self)?)?;
        Ok(path)
    }
}
