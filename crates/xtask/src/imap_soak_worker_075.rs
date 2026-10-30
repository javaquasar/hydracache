//! Long-lived reference service worker. The process is never replaced between rounds.
use crate::imap_soak_075::*;
use hydracache_cluster_testkit::{
    value_plane_collection_075::CollectionBounds,
    value_plane_model_075::MutationIdentity,
    value_plane_service_075::{ServiceCommand, ServiceListenerOptions, UnifiedIMapService},
};
use std::{
    error::Error,
    io::Write,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
pub struct WorkerConfig {
    pub profile: SoakProfile,
    pub control: PathBuf,
    pub fault: String,
}
impl WorkerConfig {
    pub fn parse(args: &[String]) -> Result<Self, Box<dyn Error>> {
        let (args, fault) =
            if let Some(fault) = args.first().and_then(|arg| arg.strip_prefix("--canary=")) {
                (&args[1..], fault.to_string())
            } else {
                (args, String::new())
            };
        if args.len() != 8 {
            return Err("expected eight bounded soak worker arguments".into());
        }
        let active: u64 = args[0].parse()?;
        let startup: u64 = args[4].parse()?;
        let hard = active
            .checked_add(startup)
            .and_then(|n| n.checked_add(10000))
            .ok_or("worker budget overflow")?;
        let bounds = SoakBounds {
            active_ms: active,
            period_ms: args[1].parse()?,
            maximum_rounds: args[2].parse()?,
            warmup_rounds: args[3].parse()?,
            startup_ms: startup,
            drain_ms: 10000,
            hard_ms: hard,
            host_remaining_ms: hard
                .checked_add(5000)
                .ok_or("worker host budget overflow")?,
        };
        let profile = SoakProfile {
            schema: "imap075-soak-profile-v1".into(),
            source_sha: args[6].clone(),
            cohort: Cohort::Reference,
            seed: args[5].parse()?,
            bounds,
            runtime: "worker-bound-by-producer".into(),
            artifacts: vec![BoundArtifact {
                path: "producer-bound".into(),
                sha256: "a".repeat(64),
            }],
        };
        profile.validate()?;
        let control = PathBuf::from(&args[7]);
        if !control.is_absolute() || !control.is_dir() {
            return Err("owned control directory required".into());
        }
        if ![
            "",
            "leak",
            "replacement",
            "replacement-child",
            "hang",
            "exit",
            "malformed",
            "flood",
            "profile",
        ]
        .contains(&fault.as_str())
        {
            return Err("unknown soak canary".into());
        }
        Ok(Self {
            profile,
            control,
            fault,
        })
    }
}
fn owners(service: &UnifiedIMapService) -> Owners {
    let usage = service.resource_usage();
    Owners {
        unit: "reference_owner_bytes".into(),
        values: Some(usage.values as u64),
        outcomes: Some(usage.outcomes as u64),
        pending: Some(usage.pending as u64),
        listeners: Some(usage.listeners as u64),
        streams: Some(usage.streams as u64),
        resync: Some(usage.resync as u64),
        queued: Some(0),
        correlations: Some(0),
    }
}
fn round(service: &UnifiedIMapService, seed: u64, position: u64) -> Result<String, Box<dyn Error>> {
    let listener = service
        .register_listener(ServiceListenerOptions::default())
        .map_err(|e| format!("{e:?}"))?;
    let mut command = ServiceCommand::point("set", b"a");
    command.value = Some((seed ^ position).to_be_bytes().to_vec());
    command.identity = Some(MutationIdentity::new("soak", position));
    service.execute(command).map_err(|e| format!("{e:?}"))?;
    let result = service
        .execute(ServiceCommand::point("get", b"a"))
        .map_err(|e| format!("{e:?}"))?;
    let value = result.value.ok_or("soak lost reference value")?;
    let callbacks = listener.drain().map_err(|e| format!("{e:?}"))?.len();
    drop(listener);
    service
        .retire_outcomes("soak", 3, position)
        .map_err(|e| format!("{e:?}"))?;
    drop(service.checkpoint().map_err(|e| format!("{e:?}"))?);
    validate_reference_round(
        &value,
        seed,
        position,
        callbacks,
        service.statistics().retained_outcomes,
    )
}
fn validate_reference_round(
    value: &[u8],
    seed: u64,
    position: u64,
    callbacks: usize,
    retained_outcomes: usize,
) -> Result<String, Box<dyn Error>> {
    if value != (seed ^ position).to_be_bytes() {
        return Err("reference logical value differs".into());
    }
    if callbacks != 1 {
        return Err("reference callback incomplete".into());
    }
    if retained_outcomes != 0 {
        return Err("reference retained outcome leak".into());
    }
    Ok(state_digest(value))
}
fn stop_reason(
    bounds: &SoakBounds,
    round: u64,
    elapsed: u64,
    cancelled: bool,
) -> Option<&'static str> {
    if cancelled {
        Some("cancelled")
    } else if elapsed >= bounds.active_ms && round >= 2 {
        Some("duration_complete")
    } else if round >= bounds.maximum_rounds {
        Some("round_limit")
    } else {
        None
    }
}
fn emit(frame: &Frame) -> Result<(), Box<dyn Error>> {
    crate::imap_coverage_075::flush_profile()?;
    let mut out = std::io::stdout().lock();
    serde_json::to_writer(&mut out, frame)?;
    writeln!(out)?;
    out.flush()?;
    Ok(())
}
pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let config = WorkerConfig::parse(&args)?;
    let profile = &config.profile;
    let service =
        UnifiedIMapService::new(CollectionBounds::default(), 3).map_err(|e| format!("{e:?}"))?;
    let incarnation = format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    );
    let mut frame = Frame {
        schema: "imap075-soak-frame-v1".into(),
        phase: Phase::Ready,
        pid: std::process::id(),
        incarnation,
        cohort: if config.fault == "profile" {
            Cohort::Native
        } else {
            Cohort::Reference
        },
        seed: profile.seed,
        source_sha: profile.source_sha.clone(),
        round: 0,
        active_elapsed_ms: 0,
        digest: None,
        owners: owners(&service),
        heap_bytes: None,
        rss_bytes: None,
        gc_collections: None,
        cpu_ns: None,
        threads: None,
        reason: None,
    };
    if config.fault == "malformed" {
        crate::imap_coverage_075::flush_profile()?;
        println!("not-json");
        return Ok(());
    }
    if config.fault == "flood" {
        crate::imap_coverage_075::flush_profile()?;
        std::io::stdout().write_all(&[b'x'; 8192])?;
        return Ok(());
    }
    emit(&frame)?;
    // Terminal child role: report one actual PID and exit before the handshake or round loop.
    // It cannot reach the only spawn site below, regardless of inherited environment.
    if config.fault == "replacement-child" {
        return Ok(());
    }
    let wait = Instant::now();
    while !marker(&config.control, "start")? {
        if marker(&config.control, "cancel")? {
            return Ok(());
        }
        if wait.elapsed() > Duration::from_millis(profile.bounds.startup_ms) {
            return Err("start handshake timeout".into());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    for position in 1..=profile.bounds.warmup_rounds {
        frame.digest = Some(round(&service, profile.seed, position)?);
    }
    frame.phase = Phase::Warmup;
    frame.owners = owners(&service);
    emit(&frame)?;
    frame.phase = Phase::Active;
    emit(&frame)?;
    let active = Instant::now();
    let mut leaked = None;
    loop {
        if let Some(reason) = stop_reason(
            &profile.bounds,
            frame.round,
            active.elapsed().as_millis() as u64,
            marker(&config.control, "cancel")?,
        ) {
            frame.reason = Some(reason.into());
            break;
        }
        frame.round += 1;
        frame.phase = Phase::Round;
        frame.digest = Some(round(
            &service,
            profile.seed,
            profile.bounds.warmup_rounds + frame.round,
        )?);
        if config.fault == "replacement" && frame.round == 2 {
            // Exactly one terminal child; worker_arguments contains only the eight numeric,
            // source and control arguments and never propagates the parent's canary mode.
            let mut child = std::process::Command::new(std::env::current_exe()?)
                .arg("--canary=replacement-child")
                .args(crate::imap_soak_075::worker_arguments(
                    profile,
                    &config.control,
                ))
                .spawn()?;
            child.wait()?;
            return Ok(());
        }
        if config.fault == "leak" && leaked.is_none() {
            leaked = Some(service.checkpoint().map_err(|e| format!("{e:?}"))?);
        }
        if config.fault == "exit" && frame.round == 2 {
            return Ok(());
        }
        frame.active_elapsed_ms = active.elapsed().as_millis() as u64;
        frame.owners = owners(&service);
        emit(&frame)?;
        let until = Instant::now() + Duration::from_millis(profile.bounds.period_ms);
        while Instant::now() < until && !marker(&config.control, "cancel")? {
            std::thread::sleep(
                Duration::from_millis(10).min(until.saturating_duration_since(Instant::now())),
            );
        }
    }
    frame.phase = Phase::Drained;
    frame.active_elapsed_ms = active.elapsed().as_millis() as u64;
    frame.owners = owners(&service);
    emit(&frame)?;
    if config.fault == "hang" {
        let mut checkpointed = false;
        loop {
            std::thread::sleep(Duration::from_millis(50));
            if !checkpointed {
                checkpointed = true;
                crate::imap_coverage_075::flush_profile()?;
            }
        }
    }
    crate::imap_coverage_075::flush_profile()?;
    Ok(())
}
#[cfg(test)]
#[path = "../tests/support/imap_soak_worker_checks_075.rs"]
mod tests;
