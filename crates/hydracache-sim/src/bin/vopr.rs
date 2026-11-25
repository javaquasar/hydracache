use std::collections::VecDeque;
use std::env;
use std::io::Read;
use std::process::ExitCode;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use hydracache_sim::{
    replay_world, run_soak, run_soak_with_seed_runner, ReplayScriptV1, SimConfig, SimWorld,
    SoakConfig, SoakReport,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SingleShotCli {
    seed: u64,
    steps: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SoakCli {
    master_seed: u64,
    budget: Duration,
    steps_per_seed: u64,
    max_seeds: Option<u64>,
    #[cfg(debug_assertions)]
    synthetic_failure_after_seeds: Option<u64>,
}

/// Replay a `ReplayScriptV1` exported by the demo ("Download script") or the
/// sandbox `/sim/replay` route, including every recorded intervention.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ReplayCli {
    /// Script path, or `-` for stdin.
    path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    SingleShot(SingleShotCli),
    Soak(SoakCli),
    Replay(ReplayCli),
}

impl Command {
    fn parse(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut args = args.collect::<VecDeque<_>>();
        let _program = args.pop_front();
        if args.front().is_some_and(|arg| arg == "soak") {
            args.pop_front();
            Ok(Self::Soak(SoakCli::parse(args)?))
        } else if args.iter().any(|arg| arg == "--script") {
            Ok(Self::Replay(ReplayCli::parse(args)?))
        } else {
            Ok(Self::SingleShot(SingleShotCli::parse(args)?))
        }
    }
}

impl SingleShotCli {
    fn parse(mut args: VecDeque<String>) -> Result<Self, String> {
        let mut seed = None;
        let mut steps = 1_000_u64;
        while let Some(arg) = args.pop_front() {
            match arg.as_str() {
                "--seed" => {
                    let raw = args
                        .pop_front()
                        .ok_or_else(|| "--seed requires a value".to_owned())?;
                    seed = Some(parse_u64("--seed", &raw)?);
                }
                "--steps" => {
                    let raw = args
                        .pop_front()
                        .ok_or_else(|| "--steps requires a value".to_owned())?;
                    steps = parse_non_zero_u64("--steps", &raw)?;
                }
                "--help" | "-h" => return Err(usage()),
                other => return Err(format!("unknown argument '{other}'\n{}", usage())),
            }
        }
        Ok(Self {
            seed: seed.unwrap_or_else(random_seed),
            steps,
        })
    }
}

impl ReplayCli {
    fn parse(mut args: VecDeque<String>) -> Result<Self, String> {
        let mut path = None;
        while let Some(arg) = args.pop_front() {
            match arg.as_str() {
                "--script" => {
                    let raw = args
                        .pop_front()
                        .ok_or_else(|| "--script requires a path (or - for stdin)".to_owned())?;
                    path = Some(raw);
                }
                "--seed" | "--steps" => {
                    return Err(format!(
                        "{arg} cannot be combined with --script: the script carries its seed and target step
{}",
                        usage()
                    ))
                }
                "--help" | "-h" => return Err(usage()),
                other => return Err(format!("unknown argument '{other}'
{}", usage())),
            }
        }
        path.map(|path| Self { path })
            .ok_or_else(|| "--script requires a path (or - for stdin)".to_owned())
    }
}

impl SoakCli {
    fn parse(mut args: VecDeque<String>) -> Result<Self, String> {
        let mut master_seed = None;
        let mut budget = Duration::from_secs(1);
        let mut steps_per_seed = 1_000_u64;
        let mut max_seeds = None;
        #[cfg(debug_assertions)]
        let mut synthetic_failure_after_seeds = None;

        while let Some(arg) = args.pop_front() {
            match arg.as_str() {
                "--master-seed" => {
                    let raw = args
                        .pop_front()
                        .ok_or_else(|| "--master-seed requires a value".to_owned())?;
                    master_seed = Some(parse_u64("--master-seed", &raw)?);
                }
                "--budget-secs" => {
                    let raw = args
                        .pop_front()
                        .ok_or_else(|| "--budget-secs requires a value".to_owned())?;
                    budget = Duration::from_secs(parse_u64("--budget-secs", &raw)?);
                }
                "--budget-ms" => {
                    let raw = args
                        .pop_front()
                        .ok_or_else(|| "--budget-ms requires a value".to_owned())?;
                    budget = Duration::from_millis(parse_u64("--budget-ms", &raw)?);
                }
                "--steps-per-seed" => {
                    let raw = args
                        .pop_front()
                        .ok_or_else(|| "--steps-per-seed requires a value".to_owned())?;
                    steps_per_seed = parse_non_zero_u64("--steps-per-seed", &raw)?;
                }
                "--max-seeds" => {
                    let raw = args
                        .pop_front()
                        .ok_or_else(|| "--max-seeds requires a value".to_owned())?;
                    max_seeds = Some(parse_non_zero_u64("--max-seeds", &raw)?);
                }
                #[cfg(debug_assertions)]
                "--synthetic-failure-after-seeds" => {
                    let raw = args.pop_front().ok_or_else(|| {
                        "--synthetic-failure-after-seeds requires a value".to_owned()
                    })?;
                    synthetic_failure_after_seeds =
                        Some(parse_non_zero_u64("--synthetic-failure-after-seeds", &raw)?);
                }
                "--help" | "-h" => return Err(usage()),
                other => return Err(format!("unknown argument '{other}'\n{}", usage())),
            }
        }

        Ok(Self {
            master_seed: master_seed.unwrap_or_else(random_seed),
            budget,
            steps_per_seed,
            max_seeds,
            #[cfg(debug_assertions)]
            synthetic_failure_after_seeds,
        })
    }

    fn config(&self) -> SoakConfig {
        let mut cfg = SoakConfig::new(
            self.master_seed,
            self.budget,
            self.steps_per_seed,
            SimConfig::default(),
        );
        cfg.max_seeds = self.max_seeds;
        cfg
    }
}

fn main() -> ExitCode {
    match Command::parse(env::args()) {
        Ok(Command::SingleShot(cli)) => run_single_shot(cli),
        Ok(Command::Soak(cli)) => run_soak_command(cli),
        Ok(Command::Replay(cli)) => run_replay(&cli),
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(64)
        }
    }
}

fn run_single_shot(cli: SingleShotCli) -> ExitCode {
    let mut world = SimWorld::new(cli.seed, SimConfig::default());
    let outcome = world.run(cli.steps);
    println!("seed={}", outcome.seed);
    println!("steps={}", outcome.steps);
    println!("accepted_ops={}", outcome.accepted_ops);
    println!("delivered_messages={}", outcome.delivered_messages);
    println!("history_hash={}", outcome.history_hash);
    println!("invariant_violations={}", outcome.invariant_violations);

    if outcome.invariant_violations == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    }
}

fn run_replay(cli: &ReplayCli) -> ExitCode {
    let mut json = String::new();
    let read = if cli.path == "-" {
        std::io::stdin().read_to_string(&mut json).map(|_| ())
    } else {
        std::fs::read_to_string(&cli.path).map(|text| json = text)
    };
    if let Err(error) = read {
        eprintln!("cannot read replay script '{}': {error}", cli.path);
        return ExitCode::from(66);
    }
    let script = match ReplayScriptV1::from_json(&json) {
        Ok(script) => script,
        Err(error) => {
            eprintln!("invalid replay script '{}': {error}", cli.path);
            return ExitCode::from(65);
        }
    };
    let world = match replay_world(&script, |seed| SimWorld::new(seed, SimConfig::default())) {
        Ok(world) => world,
        Err(error) => {
            eprintln!("replay script '{}' was rejected: {error}", cli.path);
            return ExitCode::from(65);
        }
    };
    let outcome = world.outcome();
    let snapshot = world.snapshot();
    println!("seed={}", outcome.seed);
    println!("steps={}", outcome.steps);
    println!("scenario={}", script.scenario.as_deref().unwrap_or("none"));
    println!("actions={}", script.actions.len());
    println!("accepted_ops={}", outcome.accepted_ops);
    println!("delivered_messages={}", outcome.delivered_messages);
    println!("history_hash={}", outcome.history_hash);
    println!("invariant_violations={}", outcome.invariant_violations);
    match snapshot.first_violation_step {
        Some(step) => println!("first_violation_step={step}"),
        None => println!("first_violation_step=none"),
    }
    println!("verdict={}", snapshot.verdict_json());

    if outcome.invariant_violations == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    }
}

fn run_soak_command(cli: SoakCli) -> ExitCode {
    let cfg = cli.config();
    let started = Instant::now();
    #[cfg(debug_assertions)]
    let outcome = if let Some(fail_after) = cli.synthetic_failure_after_seeds {
        let mut seeds = 0_u64;
        run_soak_with_seed_runner(&cfg, |_seed, steps, _sim| {
            seeds = seeds.saturating_add(1);
            (seeds >= fail_after).then(|| {
                (
                    steps,
                    vec!["synthetic_soak_failure: debug test hook".to_owned()],
                )
            })
        })
    } else {
        run_soak(&cfg)
    };
    #[cfg(not(debug_assertions))]
    let outcome = run_soak(&cfg);

    match serde_json::to_string(&SoakReport::from_outcome(&outcome, started.elapsed())) {
        Ok(report) => println!("{report}"),
        Err(err) => {
            eprintln!("failed to serialize soak report: {err}");
            return ExitCode::from(70);
        }
    }

    if outcome.first_failure.is_some() {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    }
}

fn parse_u64(flag: &str, raw: &str) -> Result<u64, String> {
    raw.parse::<u64>()
        .map_err(|_| format!("invalid {flag} value '{raw}'"))
}

fn parse_non_zero_u64(flag: &str, raw: &str) -> Result<u64, String> {
    let value = parse_u64(flag, raw)?;
    if value == 0 {
        Err(format!("{flag} must be greater than zero"))
    } else {
        Ok(value)
    }
}

fn random_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0x44_44_44_44)
}

fn usage() -> String {
    [
        "usage:",
        "  vopr [--seed <u64>] [--steps <u64>]",
        "  vopr --script <replay.json|->",
        "  vopr soak [--master-seed <u64>] [--budget-secs <u64>|--budget-ms <u64>] [--steps-per-seed <u64>] [--max-seeds <u64>]",
    ]
    .join("\n")
}
