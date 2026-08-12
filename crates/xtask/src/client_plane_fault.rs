use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use hydracache_client_plane_spike::fault_proxy::{
    FaultAction, FaultPlan, FaultReplayArtifact, ProxyDirection,
};

const RETAINED_ARTIFACTS: [&str; 16] = [
    "docs/testing/hc2-fault-proxy/h19-seed-1592590353.json",
    "docs/testing/hc2-fault-proxy/h20-client-half-close-seed-1592590354.json",
    "docs/testing/hc2-fault-proxy/h20-uncooperative-timeout-seed-1592590355.json",
    "docs/testing/hc2-fault-proxy/h20-peer-reset-seed-1592590356.json",
    "docs/testing/hc2-fault-proxy/h11-handshake-reset-seed-1592590357.json",
    "docs/testing/hc2-fault-proxy/h11-server-restart-seed-1592590358.json",
    "docs/testing/hc2-fault-proxy/h11-leader-hint-reset-seed-1592590359.json",
    "docs/testing/hc2-fault-proxy/h11-subscription-gap-seed-1592590360.json",
    "docs/testing/hc2-fault-proxy/h11-invocation-reset-seed-1592590361.json",
    "docs/testing/hc2-fault-proxy/h11-session-loss-seed-1592590362.json",
    "docs/testing/hc2-fault-proxy/h11-duplicate-event-seed-1592590363.json",
    "docs/testing/hc2-fault-proxy/h11-reconnect-exhausted-seed-1592590364.json",
    "docs/testing/hc2-fault-proxy/h03-candidate-preservation-seed-1592590365.json",
    "docs/testing/hc2-fault-proxy/h03-candidate-close-seed-1592590366.json",
    "docs/testing/hc2-fault-proxy/h03-candidate-duplicate-seed-1592590367.json",
    "docs/testing/hc2-fault-proxy/h03-candidate-reset-seed-1592590368.json",
];
const TARGET_DIR: &str = "target/hc2-fault-check";

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let root = workspace_root()?;
    match args.as_slice() {
        [] => check_at_root(&root, true),
        [flag, path] if flag == "--replay" => verify_path(&root, Path::new(path)),
        [seed_flag, seed, output_flag, output]
            if seed_flag == "--seed" && output_flag == "--output" =>
        {
            let seed = seed.parse::<u64>()?;
            write_artifact(&root, "h19", seed, Path::new(output))
        }
        [case_flag, case_id, seed_flag, seed, output_flag, output]
            if case_flag == "--case" && seed_flag == "--seed" && output_flag == "--output" =>
        {
            let seed = seed.parse::<u64>()?;
            write_artifact(&root, case_id, seed, Path::new(output))
        }
        _ => Err(
            "usage: client-plane-fault-check [--replay <path>|[--case <h19|h20-*|h11-*|h03-candidate-preservation|h03-candidate-close|h03-candidate-duplicate|h03-candidate-reset> --seed <u64> --output <path>]".into(),
        ),
    }
}

pub fn check_at_root(root: &Path, run_test: bool) -> Result<(), Box<dyn Error>> {
    for artifact in RETAINED_ARTIFACTS {
        verify_path(root, Path::new(artifact))?;
    }
    if run_test {
        let status = Command::new("cargo")
            .args([
                "test",
                "--locked",
                "-p",
                "hydracache-client-plane-spike",
                "--test",
                "fault_proxy",
                "--target-dir",
                TARGET_DIR,
            ])
            .current_dir(root)
            .status()?;
        if !status.success() {
            return Err(
                format!("HC/2 deterministic fault-proxy tests failed with {status}").into(),
            );
        }
    }
    println!("client-plane-fault-check: OK (H03/H11/H19/H20 retained seeds replayed exactly; raw payload absent)");
    Ok(())
}

fn verify_path(root: &Path, path: &Path) -> Result<(), Box<dyn Error>> {
    let path = resolve(root, path);
    let bytes = fs::read(&path)?;
    if bytes.len() > 64 * 1024 {
        return Err(format!("fault replay artifact exceeds 64 KiB: {}", path.display()).into());
    }
    let artifact: FaultReplayArtifact = serde_json::from_slice(&bytes)?;
    artifact.verify()?;
    Ok(())
}

fn write_artifact(
    root: &Path,
    case_id: &str,
    seed: u64,
    path: &Path,
) -> Result<(), Box<dyn Error>> {
    let path = resolve(root, path);
    let artifact =
        FaultReplayArtifact::create(retained_plan(case_id, seed)?, vec![17, 31, 47, 71])?;
    let mut bytes = serde_json::to_vec_pretty(&artifact)?;
    bytes.push(b'\n');
    if bytes.len() > 64 * 1024 {
        return Err("generated fault replay artifact exceeds 64 KiB".into());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, bytes)?;
    println!("wrote deterministic fault replay: {}", path.display());
    Ok(())
}

fn retained_plan(case_id: &str, seed: u64) -> Result<FaultPlan, Box<dyn Error>> {
    let plan = match case_id {
        "h19" => FaultPlan::new(
            "h19-retained-half-open-late-pressure",
            seed,
            ProxyDirection::ServerToClient,
            vec![
                FaultAction::Fragment {
                    max_chunk_bytes: 11,
                },
                FaultAction::Coalesce { max_chunks: 3 },
                FaultAction::Delay { ticks: 2 },
                FaultAction::HalfOpen,
                FaultAction::LateDelivery { ticks: 7 },
                FaultAction::BandwidthPressure {
                    bytes_per_tick: 13,
                    window_bytes: 52,
                },
            ],
        ),
        "h20-client-half-close" => FaultPlan::new(
            "h20-client-half-close",
            seed,
            ProxyDirection::ClientToServer,
            vec![
                FaultAction::Fragment { max_chunk_bytes: 7 },
                FaultAction::HalfOpen,
            ],
        ),
        "h20-uncooperative-timeout" => FaultPlan::new(
            "h20-uncooperative-timeout",
            seed,
            ProxyDirection::ServerToClient,
            vec![
                FaultAction::Delay { ticks: 10 },
                FaultAction::BlockDirection,
            ],
        ),
        "h20-peer-reset" => FaultPlan::new(
            "h20-peer-reset",
            seed,
            ProxyDirection::ClientToServer,
            vec![
                FaultAction::Fragment { max_chunk_bytes: 5 },
                FaultAction::Reset,
            ],
        ),
        "h11-handshake-reset" => FaultPlan::new(
            "h11-handshake-reset",
            seed,
            ProxyDirection::ServerToClient,
            vec![
                FaultAction::Fragment { max_chunk_bytes: 3 },
                FaultAction::Reset,
            ],
        ),
        "h11-server-restart" => FaultPlan::new(
            "h11-server-restart",
            seed,
            ProxyDirection::ServerToClient,
            vec![FaultAction::CloseAfterBytes { bytes: 37 }],
        ),
        "h11-leader-hint-reset" => FaultPlan::new(
            "h11-leader-hint-reset",
            seed,
            ProxyDirection::ServerToClient,
            vec![
                FaultAction::Delay { ticks: 3 },
                FaultAction::CloseAfterBytes { bytes: 71 },
            ],
        ),
        "h11-subscription-gap" => FaultPlan::new(
            "h11-subscription-gap",
            seed,
            ProxyDirection::ServerToClient,
            vec![
                FaultAction::Fragment { max_chunk_bytes: 5 },
                FaultAction::Drop { every_nth: 2 },
            ],
        ),
        "h11-invocation-reset" => FaultPlan::new(
            "h11-invocation-reset",
            seed,
            ProxyDirection::ServerToClient,
            vec![FaultAction::Delay { ticks: 1 }, FaultAction::Reset],
        ),
        "h11-session-loss" => FaultPlan::new(
            "h11-session-loss",
            seed,
            ProxyDirection::ClientToServer,
            vec![
                FaultAction::HalfOpen,
                FaultAction::LateDelivery { ticks: 9 },
            ],
        ),
        "h11-duplicate-event" => FaultPlan::new(
            "h11-duplicate-event",
            seed,
            ProxyDirection::ServerToClient,
            vec![FaultAction::Duplicate { copies: 1 }],
        ),
        "h11-reconnect-exhausted" => FaultPlan::new(
            "h11-reconnect-exhausted",
            seed,
            ProxyDirection::ServerToClient,
            vec![FaultAction::BlockDirection],
        ),
        "h03-candidate-preservation" => FaultPlan::new(
            "h03-candidate-preservation",
            seed,
            ProxyDirection::ClientToServer,
            vec![
                FaultAction::Fragment { max_chunk_bytes: 5 },
                FaultAction::Coalesce { max_chunks: 3 },
                FaultAction::Delay { ticks: 2 },
                FaultAction::BandwidthPressure {
                    bytes_per_tick: 7,
                    window_bytes: 21,
                },
            ],
        ),
        "h03-candidate-close" => FaultPlan::new(
            "h03-candidate-close",
            seed,
            ProxyDirection::ClientToServer,
            vec![FaultAction::CloseAfterBytes { bytes: 1 }],
        ),
        "h03-candidate-duplicate" => FaultPlan::new(
            "h03-candidate-duplicate",
            seed,
            ProxyDirection::ClientToServer,
            vec![FaultAction::Duplicate { copies: 1 }],
        ),
        "h03-candidate-reset" => FaultPlan::new(
            "h03-candidate-reset",
            seed,
            ProxyDirection::ClientToServer,
            vec![FaultAction::Reset],
        ),
        other => return Err(format!("unsupported retained fault case: {other}").into()),
    };
    Ok(plan)
}

fn resolve(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn workspace_root() -> Result<PathBuf, Box<dyn Error>> {
    let mut candidate = std::env::current_dir()?;
    loop {
        if candidate.join("Cargo.toml").is_file()
            && candidate
                .join("crates")
                .join("hydracache-client-plane-spike")
                .is_dir()
        {
            return Ok(candidate);
        }
        if !candidate.pop() {
            return Err("could not locate HydraCache workspace root".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_plan_is_valid_and_bounded() {
        let artifact = FaultReplayArtifact::create(
            retained_plan("h19", 1_592_590_353).unwrap(),
            vec![17, 31, 47, 71],
        )
        .unwrap();
        artifact.verify().unwrap();
        assert!(serde_json::to_vec(&artifact).unwrap().len() < 64 * 1024);
    }

    #[test]
    fn h20_lifecycle_plans_have_distinct_terminal_actions() {
        for (case_id, expected) in [
            ("h20-client-half-close", FaultAction::HalfOpen),
            ("h20-uncooperative-timeout", FaultAction::BlockDirection),
            ("h20-peer-reset", FaultAction::Reset),
        ] {
            let plan = retained_plan(case_id, 7).unwrap();
            assert_eq!(plan.actions.last(), Some(&expected));
            plan.validate().unwrap();
        }
    }

    #[test]
    fn h11_reconnect_and_repair_plans_are_distinct_and_bounded() {
        for (case_id, seed, terminal) in [
            ("h11-handshake-reset", 1_592_590_357, FaultAction::Reset),
            (
                "h11-server-restart",
                1_592_590_358,
                FaultAction::CloseAfterBytes { bytes: 37 },
            ),
            (
                "h11-leader-hint-reset",
                1_592_590_359,
                FaultAction::CloseAfterBytes { bytes: 71 },
            ),
            (
                "h11-subscription-gap",
                1_592_590_360,
                FaultAction::Drop { every_nth: 2 },
            ),
            ("h11-invocation-reset", 1_592_590_361, FaultAction::Reset),
            (
                "h11-session-loss",
                1_592_590_362,
                FaultAction::LateDelivery { ticks: 9 },
            ),
            (
                "h11-duplicate-event",
                1_592_590_363,
                FaultAction::Duplicate { copies: 1 },
            ),
            (
                "h11-reconnect-exhausted",
                1_592_590_364,
                FaultAction::BlockDirection,
            ),
        ] {
            let plan = retained_plan(case_id, seed).unwrap();
            assert_eq!(plan.actions.last(), Some(&terminal), "{case_id}");
            plan.validate().unwrap();
            let artifact = FaultReplayArtifact::create(plan, vec![17, 31, 47, 71]).unwrap();
            artifact.verify().unwrap();
            assert!(serde_json::to_vec(&artifact).unwrap().len() < 64 * 1024);
        }
    }

    #[test]
    fn h03_candidate_plans_are_retained_and_bounded() {
        for (case_id, seed, terminal) in [
            (
                "h03-candidate-preservation",
                1_592_590_365,
                FaultAction::BandwidthPressure {
                    bytes_per_tick: 7,
                    window_bytes: 21,
                },
            ),
            (
                "h03-candidate-close",
                1_592_590_366,
                FaultAction::CloseAfterBytes { bytes: 1 },
            ),
            (
                "h03-candidate-duplicate",
                1_592_590_367,
                FaultAction::Duplicate { copies: 1 },
            ),
            ("h03-candidate-reset", 1_592_590_368, FaultAction::Reset),
        ] {
            let plan = retained_plan(case_id, seed).unwrap();
            assert_eq!(plan.actions.last(), Some(&terminal), "{case_id}");
            plan.validate().unwrap();
            let artifact = FaultReplayArtifact::create(plan, vec![17, 31, 47, 71]).unwrap();
            artifact.verify().unwrap();
            assert!(serde_json::to_vec(&artifact).unwrap().len() < 64 * 1024);
        }
    }
}
