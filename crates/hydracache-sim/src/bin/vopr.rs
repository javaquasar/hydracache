use std::env;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use hydracache_sim::{SimConfig, SimWorld};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Cli {
    seed: u64,
    steps: u64,
}

impl Cli {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let _program = args.next();
        let mut seed = None;
        let mut steps = 1_000_u64;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--seed" => {
                    let raw = args
                        .next()
                        .ok_or_else(|| "--seed requires a value".to_owned())?;
                    seed = Some(
                        raw.parse::<u64>()
                            .map_err(|_| format!("invalid --seed value '{raw}'"))?,
                    );
                }
                "--steps" => {
                    let raw = args
                        .next()
                        .ok_or_else(|| "--steps requires a value".to_owned())?;
                    steps = raw
                        .parse::<u64>()
                        .map_err(|_| format!("invalid --steps value '{raw}'"))?;
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

fn main() -> ExitCode {
    let cli = match Cli::parse(env::args()) {
        Ok(cli) => cli,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(64);
        }
    };

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

fn random_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0x44_44_44_44)
}

fn usage() -> String {
    "usage: vopr [--seed <u64>] [--steps <u64>]".to_owned()
}
