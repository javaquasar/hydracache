//! Each real worker history is checked independently; different legal winners are permitted.
use hydracache_cluster_testkit::value_plane_extended_concurrent_075::{
    generated_extended_schedule, ExtendedRound, ExtendedSchedule, MAX_HISTORY_BYTES,
    MAX_SCHEDULE_BYTES,
};
use std::{error::Error, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegalVerdict {
    Valid,
    Invalid,
    SearchExhausted,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedConcurrentReport {
    pub seed: i64,
    pub rounds: usize,
    pub process_canaries: usize,
    pub search_budget: usize,
}
pub fn parse_verdicts(
    output: &[u8],
    schedule: &ExtendedSchedule,
) -> Result<Vec<LegalVerdict>, Box<dyn Error>> {
    schedule.encode()?;
    if output.len() > 4096 || !output.is_ascii() || output.contains(&b'\r') {
        return Err("extended verdict encoding/bound".into());
    }
    let source = std::str::from_utf8(output)?;
    let mut lines = source.strip_suffix('\n').unwrap_or(source).split('\n');
    let header = lines
        .next()
        .unwrap_or_default()
        .split('\t')
        .collect::<Vec<_>>();
    if header
        != vec![
            "imap-extended-verdict-v1",
            &schedule.seed.to_string(),
            &schedule.rounds.len().to_string(),
        ]
    {
        return Err("extended verdict identity".into());
    }
    let mut result = Vec::new();
    for round in &schedule.rounds {
        let f = lines
            .next()
            .ok_or("missing verdict")?
            .split('\t')
            .collect::<Vec<_>>();
        if f.len() != 3 || f[0] != round.id.to_string() {
            return Err("verdict round identity".into());
        }
        let states = f[2].parse::<usize>()?;
        if states == 0 || states > 100000 || states.to_string() != f[2] {
            return Err("verdict explored bound".into());
        }
        result.push(match f[1] {
            "valid" => LegalVerdict::Valid,
            "invalid" => LegalVerdict::Invalid,
            "search_exhausted" => LegalVerdict::SearchExhausted,
            _ => return Err("unknown verdict".into()),
        });
    }
    if lines.next().is_some() {
        return Err("extra verdict".into());
    }
    Ok(result)
}
fn check_envelope(output: &[u8], schedule: &ExtendedSchedule) -> Result<(), Box<dyn Error>> {
    if output.len() > MAX_HISTORY_BYTES || !output.is_ascii() || output.contains(&b'\r') {
        return Err("extended history encoding/bound".into());
    }
    let source = std::str::from_utf8(output)?;
    let mut lines = source.strip_suffix('\n').unwrap_or(source).split('\n');
    let header = lines
        .next()
        .unwrap_or_default()
        .split('\t')
        .collect::<Vec<_>>();
    if header
        != vec![
            "imap-extended-history-v1",
            &schedule.seed.to_string(),
            &schedule.rounds.len().to_string(),
        ]
    {
        return Err("extended history identity".into());
    }
    for round in &schedule.rounds {
        let fields = lines
            .next()
            .ok_or("missing history")?
            .split('\t')
            .collect::<Vec<_>>();
        if fields.len() != 5
            || fields[0] != round.id.to_string()
            || fields[1] != round.family
            || fields[2] != round.order
        {
            return Err("history round substitution".into());
        }
    }
    if lines.next().is_some() {
        return Err("extra history".into());
    }
    Ok(())
}
pub fn execute_java(
    root: &Path,
    schedule: &ExtendedSchedule,
    defect: bool,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut args = vec!["--schedule", "-"];
    if defect {
        args.push("--canary-unconditional-version");
    }
    let output = crate::imap_api_differential::execute_java_bounded(
        root,
        "ExtendedConcurrentRunner",
        schedule.encode()?,
        &args,
        MAX_HISTORY_BYTES,
    )?;
    check_envelope(&output, schedule)?;
    Ok(output)
}
pub fn execute_rust(
    root: &Path,
    schedule: &ExtendedSchedule,
    defect: bool,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    let target = if target.is_absolute() {
        target
    } else {
        root.join(target)
    };
    let peer = target.join("debug").join(if cfg!(windows) {
        "imap-extended-concurrent-owner-075.exe"
    } else {
        "imap-extended-concurrent-owner-075"
    });
    if !peer.is_file() {
        return Err("actual extended concurrent Rust peer is missing".into());
    }
    let mut command = std::process::Command::new(peer);
    command.current_dir(root);
    if defect {
        command.arg("--canary-unconditional-version");
    }
    let output = crate::imap_api_differential::execute_process_bounded(
        command,
        schedule.encode()?,
        MAX_HISTORY_BYTES,
    )?;
    check_envelope(&output, schedule)?;
    Ok(output)
}
pub fn validate_history(
    root: &Path,
    schedule: &ExtendedSchedule,
    history: &[u8],
    budget: usize,
) -> Result<Vec<LegalVerdict>, Box<dyn Error>> {
    if budget == 0 || budget > 100000 {
        return Err("legal-history search bound".into());
    }
    check_envelope(history, schedule)?;
    let budget = budget.to_string();
    let output = crate::imap_api_differential::execute_java_bounded(
        root,
        "ExtendedConcurrentRunner",
        std::str::from_utf8(history)?.into(),
        &["--check-history", "-", "--search-budget", &budget],
        4096,
    )?;
    parse_verdicts(&output, schedule)
}
pub fn check_schedule(
    root: &Path,
    schedule: &ExtendedSchedule,
    budget: usize,
) -> Result<ExtendedConcurrentReport, Box<dyn Error>> {
    if budget == 0 || budget > 100000 {
        return Err("legal-history search bound".into());
    }
    schedule.encode()?;
    for (language, history) in [
        ("Java", execute_java(root, schedule, false)?),
        ("Rust", execute_rust(root, schedule, false)?),
    ] {
        let verdicts = validate_history(root, schedule, &history, budget)?;
        if let Some((index, verdict)) = verdicts
            .iter()
            .enumerate()
            .find(|(_, v)| **v != LegalVerdict::Valid)
        {
            let source = crate::imap_api_differential::retain_source(
                root,
                &schedule.encode()?,
                &format!("extended-concurrent-seed-{}", schedule.seed),
            )?;
            let observation = crate::imap_api_differential::retain_source(
                root,
                std::str::from_utf8(&history)?,
                &format!("extended-concurrent-{language}-seed-{}", schedule.seed),
            )?;
            return Err(format!(
                "{} seed={} round={} language={language}; schedule={} history={}",
                if *verdict == LegalVerdict::SearchExhausted {
                    "extended search exhausted"
                } else {
                    "illegal extended history"
                },
                schedule.seed,
                schedule.rounds[index].id,
                source.display(),
                observation.display()
            )
            .into());
        }
    }
    let canary = ExtendedSchedule {
        seed: schedule.seed,
        rounds: vec![ExtendedRound {
            id: 1,
            family: "bulk_aba".into(),
            order: "right_first".into(),
        }],
    };
    for history in [
        execute_java(root, &canary, true)?,
        execute_rust(root, &canary, true)?,
    ] {
        if validate_history(root, &canary, &history, 4096)? != vec![LegalVerdict::Invalid] {
            return Err("actual concurrent process canary escaped".into());
        }
    }
    Ok(ExtendedConcurrentReport {
        seed: schedule.seed,
        rounds: schedule.rounds.len(),
        process_canaries: 2,
        search_budget: budget,
    })
}
pub fn check_actual(
    root: &Path,
    seed: i64,
    rounds: usize,
) -> Result<ExtendedConcurrentReport, Box<dyn Error>> {
    check_schedule(root, &generated_extended_schedule(seed, rounds)?, 4096)
}
pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let mut release = None;
    let mut seed = 117;
    let mut rounds = 15;
    let mut script = None;
    let mut seen = std::collections::BTreeSet::new();
    let mut fields = args.chunks_exact(2);
    for pair in &mut fields {
        if !seen.insert(pair[0].as_str()) {
            return Err("duplicate concurrent option".into());
        }
        match pair[0].as_str() {
            "--release" => release = Some(pair[1].as_str()),
            "--seed" => {
                seed = pair[1].parse()?;
                if seed.to_string() != pair[1] {
                    return Err("canonical seed".into());
                }
            }
            "--rounds" => {
                rounds = pair[1].parse()?;
                if rounds.to_string() != pair[1] {
                    return Err("canonical rounds".into());
                }
            }
            "--schedule" => script = Some(&pair[1]),
            _ => return Err("unknown concurrent option".into()),
        }
    }
    if release != Some("0.75")
        || !fields.remainder().is_empty()
        || script.is_some() && (seen.contains("--seed") || seen.contains("--rounds"))
    {
        return Err("concurrent release/options".into());
    }
    let schedule = if let Some(path) = script {
        use std::io::Read;
        let mut source = String::new();
        std::fs::File::open(path)?
            .take(MAX_SCHEDULE_BYTES as u64 + 1)
            .read_to_string(&mut source)?;
        ExtendedSchedule::parse(&source)?
    } else {
        generated_extended_schedule(seed, rounds)?
    };
    let root = crate::doc_check::find_repo_root()?;
    crate::imap_api_differential::prepare_java_at_root(&root)?;
    let report = check_schedule(&root, &schedule, 4096)?;
    println!("imap-extended-concurrent 0.75: {report:?}");
    Ok(())
}
