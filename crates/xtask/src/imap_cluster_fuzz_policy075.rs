//! Scheduled-fuzz policy and executed local corpus checks, never ship admission.
use crate::imap_qualification_075::read_relative;
use serde_yaml::Value;
use std::error::Error;
use std::path::Path;

/// Execute the retained corpus in every local/PR cluster lane, independently of
/// the scheduled mutation fuzzer. Expected refusals must be the named boundary.
pub fn check_corpus_at_root(root: &Path) -> Result<usize, Box<dyn std::error::Error>> {
    use hydracache_cluster_testkit::value_plane_cluster_chaos_075::{
        fuzz_cluster_schedule, ClusterScheduleError,
    };
    let cases = [
        ("point-replay.json", None),
        ("future-version.json", Some("fixture version")),
        ("early-response.json", Some("phase order")),
        ("bad-hex.json", Some("hex")),
    ];
    for (name, refusal) in cases {
        let bytes = crate::imap_qualification_075::read_relative(
            root,
            &format!("fuzz/corpus/fuzz_imap_cluster_075/{name}"),
            65536,
        )?;
        match (refusal, fuzz_cluster_schedule(&bytes)) {
            (None, Ok(_)) => {}
            (Some(expected), Err(ClusterScheduleError::InvalidSchedule(actual)))
                if actual == expected => {}
            (_, actual) => {
                return Err(format!(
                    "retained cluster corpus {name} changed disposition: {actual:?}"
                )
                .into())
            }
        }
    }
    Ok(cases.len())
}

const WORKFLOW: &str = ".github/workflows/ci.yml";
const RUN_STEP: &str = "Run bounded 0.75 IMap cluster fuzz foundation";
const UPLOAD_STEP: &str = "Upload 0.75 IMap cluster fuzz artifacts";
const CURSOR_STEP: &str = "Run 0.72 management cursor fuzz proof";
const RUN: &str = "cargo +nightly fuzz run fuzz_imap_cluster_075 -- -max_total_time=60 -max_len=65536 -rss_limit_mb=2048 -timeout=10";
const UPLOAD: &str = "actions/upload-artifact@b7c566a772e6b6bfb58ed0dc250532a479d7789f";
const ARTIFACT_PATHS: [&str; 2] = [
    "fuzz/artifacts/fuzz_imap_cluster_075/**",
    "fuzz/corpus/fuzz_imap_cluster_075/**",
];

/// Inspect the existing nightly job without running a fuzzer or changing its policy.
pub fn check_at_root(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let bytes = read_relative(root, WORKFLOW, 512 * 1024)?;
    let workflow: Value = serde_yaml::from_slice(&bytes)?;
    let job = &workflow["jobs"]["fuzz-nightly"];
    let steps = job["steps"]
        .as_sequence()
        .ok_or("fuzz-nightly steps required")?;
    let mut problems = Vec::new();
    if ignores_failure(&job["continue-on-error"]) {
        problems.push("fuzz-nightly cannot ignore failed jobs".into());
    }
    let named = |name: &str| {
        steps
            .iter()
            .enumerate()
            .filter_map(|(index, step)| (step["name"].as_str() == Some(name)).then_some(index))
            .collect::<Vec<_>>()
    };
    let runs = named(RUN_STEP);
    let uploads = named(UPLOAD_STEP);
    if runs.len() != 1 || uploads.len() != 1 {
        problems.push("one 0.75 cluster fuzz run and artifact upload required".into());
        return Ok(problems);
    }
    let run_index = runs[0];
    let run = &steps[run_index];
    if run["run"]
        .as_str()
        .map(|text| text.split_whitespace().collect::<Vec<_>>())
        != Some(RUN.split_whitespace().collect())
        || run["timeout-minutes"].as_u64() != Some(10)
        || run["working-directory"].as_str() != Some("fuzz")
    {
        problems.push(
            "0.75 cluster fuzz must retain its exact command and time/input/RSS bounds".into(),
        );
    }
    if ignores_failure(&run["continue-on-error"]) || !run["if"].is_null() {
        problems.push("0.75 cluster fuzz cannot ignore or independently skip execution".into());
    }
    let last_cleanup = steps.iter().rposition(|step| {
        step["run"]
            .as_str()
            .is_some_and(|text| text.contains("git clean") && text.contains("fuzz/corpus"))
    });
    let last_cursor_proof = named(CURSOR_STEP).last().copied();
    if last_cleanup.is_none_or(|index| index >= run_index)
        || last_cursor_proof.is_none_or(|index| index >= run_index)
    {
        problems.push(
            "0.75 cluster fuzz must follow the final corpus cleanup and 0.72 cursor proof".into(),
        );
    }
    let upload_index = uploads[0];
    let upload = &steps[upload_index];
    let paths = upload["with"]["path"].as_str().map(|text| {
        text.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
    });
    if upload_index <= run_index
        || upload["if"].as_str() != Some("always()")
        || upload["uses"].as_str() != Some(UPLOAD)
        || upload["with"]["name"].as_str() != Some("imap-cluster-fuzz-075-${{ github.sha }}")
        || upload["with"]["if-no-files-found"].as_str() != Some("error")
        || paths != Some(ARTIFACT_PATHS.to_vec())
        || ignores_failure(&upload["continue-on-error"])
    {
        problems.push("0.75 cluster fuzz must always retain its corpus and failure artifacts using the immutable upload action".into());
    }
    Ok(problems)
}

fn ignores_failure(value: &Value) -> bool {
    !value.is_null() && value != &Value::Bool(false)
}
