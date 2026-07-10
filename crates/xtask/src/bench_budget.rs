use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const DEFAULT_BUDGET_PATH: &str = "benches/budget.toml";
pub const DEFAULT_BASELINE_PATH: &str = "benches/baseline/0_37.json";
pub const DEFAULT_CURRENT_PATH: &str = "target/criterion";

#[derive(Debug)]
pub struct BenchBudgetError(String);

impl BenchBudgetError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for BenchBudgetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for BenchBudgetError {}

#[derive(Debug, Clone, PartialEq)]
pub struct BudgetRule {
    pub id: String,
    pub spec: BudgetRuleSpec,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetRuleSpec {
    pub max_regression_pct: Option<f64>,
    pub max_ns_absolute: Option<f64>,
    pub max_amplification_x: Option<f64>,
    pub numerator: Option<String>,
    pub denominator: Option<String>,
}

impl BudgetRuleSpec {
    fn validate(&self, id: &str) -> Result<(), BenchBudgetError> {
        if self.max_regression_pct.is_none()
            && self.max_ns_absolute.is_none()
            && self.max_amplification_x.is_none()
        {
            return Err(BenchBudgetError::new(format!(
                "budget rule {id} has no threshold"
            )));
        }
        validate_positive("max_regression_pct", id, self.max_regression_pct)?;
        validate_positive("max_ns_absolute", id, self.max_ns_absolute)?;
        validate_positive("max_amplification_x", id, self.max_amplification_x)?;
        if self.max_amplification_x.is_some()
            && (self.numerator.is_none() || self.denominator.is_none())
        {
            return Err(BenchBudgetError::new(format!(
                "budget rule {id} needs numerator and denominator for amplification"
            )));
        }
        Ok(())
    }
}

fn validate_positive(field: &str, id: &str, value: Option<f64>) -> Result<(), BenchBudgetError> {
    if let Some(value) = value {
        if !value.is_finite() || value <= 0.0 {
            return Err(BenchBudgetError::new(format!(
                "budget rule {id} has invalid {field}: {value}"
            )));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchMeasurements {
    pub version: u32,
    pub measurements: BTreeMap<String, BenchMeasurement>,
}

impl Default for BenchMeasurements {
    fn default() -> Self {
        Self {
            version: 1,
            measurements: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BenchMeasurement {
    pub mean_ns: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BudgetReport {
    pub failures: Vec<BudgetFailure>,
}

impl BudgetReport {
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BudgetFailure {
    pub id: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct CriterionEstimates {
    mean: CriterionEstimate,
}

#[derive(Debug, Deserialize)]
struct CriterionEstimate {
    point_estimate: f64,
}

#[derive(Debug, Deserialize)]
struct CriterionBenchmarkId {
    group_id: String,
    function_id: Option<String>,
    value_str: Option<String>,
}

impl CriterionBenchmarkId {
    fn id(&self) -> String {
        match (&self.function_id, &self.value_str) {
            (Some(function), Some(value)) => format!("{}/{function}/{value}", self.group_id),
            (Some(function), None) => format!("{}/{function}", self.group_id),
            (None, Some(value)) => format!("{}/{value}", self.group_id),
            (None, None) => self.group_id.clone(),
        }
    }
}

pub fn run(args: Vec<String>) -> Result<(), BenchBudgetError> {
    let options = BenchBudgetOptions::parse(args)?;
    let rules = load_budget(&options.budget_path)?;
    let baseline = load_measurements(&options.baseline_path)?;
    let current = load_measurements(&options.current_path)?;
    let report = check_budget(&rules, &baseline, &current);

    if report.passed() {
        println!(
            "performance budget passed: {} rule(s), current={}",
            rules.len(),
            options.current_path.display()
        );
        return Ok(());
    }

    for failure in &report.failures {
        eprintln!("{}: {}", failure.id, failure.message);
    }
    Err(BenchBudgetError::new(format!(
        "performance budget failed with {} violation(s)",
        report.failures.len()
    )))
}

#[derive(Debug, Clone)]
struct BenchBudgetOptions {
    budget_path: PathBuf,
    baseline_path: PathBuf,
    current_path: PathBuf,
}

impl BenchBudgetOptions {
    fn parse(args: Vec<String>) -> Result<Self, BenchBudgetError> {
        let mut budget_path = PathBuf::from(DEFAULT_BUDGET_PATH);
        let mut baseline_path = PathBuf::from(DEFAULT_BASELINE_PATH);
        let mut current_path = PathBuf::from(DEFAULT_CURRENT_PATH);
        let mut iter = args.into_iter();

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--budget" => {
                    budget_path = next_path(&mut iter, "--budget")?;
                }
                "--baseline" => {
                    baseline_path = next_path(&mut iter, "--baseline")?;
                }
                "--current" => {
                    current_path = next_path(&mut iter, "--current")?;
                }
                "--help" | "-h" => {
                    println!(
                        "Usage: cargo run -p xtask -- bench-budget [--budget PATH] [--baseline PATH] [--current PATH]"
                    );
                    return Err(BenchBudgetError::new("help requested"));
                }
                other => {
                    return Err(BenchBudgetError::new(format!(
                        "unsupported bench-budget argument: {other}"
                    )))
                }
            }
        }

        Ok(Self {
            budget_path,
            baseline_path,
            current_path,
        })
    }
}

fn next_path(
    iter: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<PathBuf, BenchBudgetError> {
    iter.next()
        .map(PathBuf::from)
        .ok_or_else(|| BenchBudgetError::new(format!("{flag} requires a path")))
}

pub fn load_budget(path: impl AsRef<Path>) -> Result<Vec<BudgetRule>, BenchBudgetError> {
    let source = fs::read_to_string(path.as_ref()).map_err(|error| {
        BenchBudgetError::new(format!(
            "failed to read budget {}: {error}",
            path.as_ref().display()
        ))
    })?;
    parse_budget(&source)
}

pub fn parse_budget(source: &str) -> Result<Vec<BudgetRule>, BenchBudgetError> {
    let value: toml::Value = toml::from_str(source)
        .map_err(|error| BenchBudgetError::new(format!("invalid budget TOML: {error}")))?;
    let table = value
        .as_table()
        .ok_or_else(|| BenchBudgetError::new("budget TOML root must be a table"))?;
    let mut rules = Vec::new();

    for (group, group_value) in table {
        let group_table = group_value.as_table().ok_or_else(|| {
            BenchBudgetError::new(format!("budget group {group} must be a table"))
        })?;
        for (bench, rule_value) in group_table {
            let id = format!("{group}/{bench}");
            let spec: BudgetRuleSpec = rule_value.clone().try_into().map_err(|error| {
                BenchBudgetError::new(format!("invalid budget rule {id}: {error}"))
            })?;
            spec.validate(&id)?;
            rules.push(BudgetRule { id, spec });
        }
    }

    rules.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(rules)
}

pub fn load_measurements(path: impl AsRef<Path>) -> Result<BenchMeasurements, BenchBudgetError> {
    let path = path.as_ref();
    if path.is_dir() {
        return load_criterion_measurements(path);
    }
    let bytes = fs::read(path).map_err(|error| {
        BenchBudgetError::new(format!(
            "failed to read measurements {}: {error}",
            path.display()
        ))
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        BenchBudgetError::new(format!(
            "invalid measurements JSON {}: {error}",
            path.display()
        ))
    })
}

fn load_criterion_measurements(root: &Path) -> Result<BenchMeasurements, BenchBudgetError> {
    let mut measurements = BenchMeasurements::default();
    collect_criterion_estimates(root, root, &mut measurements)?;
    if measurements.measurements.is_empty() {
        return Err(BenchBudgetError::new(format!(
            "no criterion estimates found under {}",
            root.display()
        )));
    }
    Ok(measurements)
}

fn collect_criterion_estimates(
    root: &Path,
    current: &Path,
    measurements: &mut BenchMeasurements,
) -> Result<(), BenchBudgetError> {
    for entry in fs::read_dir(current).map_err(|error| {
        BenchBudgetError::new(format!(
            "failed to read criterion directory {}: {error}",
            current.display()
        ))
    })? {
        let entry = entry.map_err(|error| {
            BenchBudgetError::new(format!(
                "failed to read criterion entry under {}: {error}",
                current.display()
            ))
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_criterion_estimates(root, &path, measurements)?;
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) != Some("estimates.json") {
            continue;
        }
        let Some(snapshot_dir_name) = path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
        else {
            continue;
        };
        if snapshot_dir_name != "new" && snapshot_dir_name != "base" {
            continue;
        }
        let bench_dir = path
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| BenchBudgetError::new("criterion estimates path is malformed"))?;
        let id = criterion_benchmark_id(root, &path, bench_dir)?;
        let bytes = fs::read(&path).map_err(|error| {
            BenchBudgetError::new(format!(
                "failed to read criterion estimates {}: {error}",
                path.display()
            ))
        })?;
        let estimates: CriterionEstimates = serde_json::from_slice(&bytes).map_err(|error| {
            BenchBudgetError::new(format!(
                "invalid criterion estimates {}: {error}",
                path.display()
            ))
        })?;
        let measurement = BenchMeasurement {
            mean_ns: estimates.mean.point_estimate,
        };
        if snapshot_dir_name == "new" {
            measurements.measurements.insert(id, measurement);
        } else {
            measurements.measurements.entry(id).or_insert(measurement);
        }
    }
    Ok(())
}

fn criterion_benchmark_id(
    root: &Path,
    estimates_path: &Path,
    bench_dir: &Path,
) -> Result<String, BenchBudgetError> {
    let Some(snapshot_dir) = estimates_path.parent() else {
        return bench_id(root, bench_dir);
    };
    let benchmark_path = snapshot_dir.join("benchmark.json");
    if !benchmark_path.is_file() {
        return bench_id(root, bench_dir);
    }
    let bytes = fs::read(&benchmark_path).map_err(|error| {
        BenchBudgetError::new(format!(
            "failed to read criterion benchmark id {}: {error}",
            benchmark_path.display()
        ))
    })?;
    let benchmark: CriterionBenchmarkId = serde_json::from_slice(&bytes).map_err(|error| {
        BenchBudgetError::new(format!(
            "invalid criterion benchmark id {}: {error}",
            benchmark_path.display()
        ))
    })?;
    let id = benchmark.id();
    if id.is_empty() {
        return Err(BenchBudgetError::new("criterion benchmark id is empty"));
    }
    Ok(id)
}

fn bench_id(root: &Path, bench_dir: &Path) -> Result<String, BenchBudgetError> {
    let relative = bench_dir.strip_prefix(root).map_err(|error| {
        BenchBudgetError::new(format!(
            "criterion bench path {} is outside {}: {error}",
            bench_dir.display(),
            root.display()
        ))
    })?;
    let id = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    if id.is_empty() {
        return Err(BenchBudgetError::new("criterion bench id is empty"));
    }
    Ok(id)
}

pub fn check_budget(
    rules: &[BudgetRule],
    baseline: &BenchMeasurements,
    current: &BenchMeasurements,
) -> BudgetReport {
    let mut failures = Vec::new();
    for rule in rules {
        check_regression(rule, baseline, current, &mut failures);
        check_absolute(rule, current, &mut failures);
        check_amplification(rule, current, &mut failures);
    }
    BudgetReport { failures }
}

fn check_regression(
    rule: &BudgetRule,
    baseline: &BenchMeasurements,
    current: &BenchMeasurements,
    failures: &mut Vec<BudgetFailure>,
) {
    let Some(max_regression_pct) = rule.spec.max_regression_pct else {
        return;
    };
    let Some(baseline_mean) = measurement("baseline", &rule.id, baseline, failures) else {
        return;
    };
    let Some(current_mean) = measurement("current", &rule.id, current, failures) else {
        return;
    };
    let allowed = baseline_mean * (1.0 + max_regression_pct / 100.0);
    if current_mean > allowed {
        failures.push(BudgetFailure {
            id: rule.id.clone(),
            message: format!(
                "mean {current_mean:.2}ns exceeds baseline {baseline_mean:.2}ns by more than {max_regression_pct:.2}% (allowed {allowed:.2}ns)"
            ),
        });
    }
}

fn check_absolute(
    rule: &BudgetRule,
    current: &BenchMeasurements,
    failures: &mut Vec<BudgetFailure>,
) {
    let Some(max_ns_absolute) = rule.spec.max_ns_absolute else {
        return;
    };
    let Some(current_mean) = measurement("current", &rule.id, current, failures) else {
        return;
    };
    if current_mean > max_ns_absolute {
        failures.push(BudgetFailure {
            id: rule.id.clone(),
            message: format!(
                "mean {current_mean:.2}ns exceeds absolute budget {max_ns_absolute:.2}ns"
            ),
        });
    }
}

fn check_amplification(
    rule: &BudgetRule,
    current: &BenchMeasurements,
    failures: &mut Vec<BudgetFailure>,
) {
    let Some(max_amplification_x) = rule.spec.max_amplification_x else {
        return;
    };
    let numerator_id = rule.spec.numerator.as_deref().unwrap_or(&rule.id);
    let denominator_id = rule.spec.denominator.as_deref().unwrap_or(&rule.id);
    let Some(numerator) = measurement("current", numerator_id, current, failures) else {
        return;
    };
    let Some(denominator) = measurement("current", denominator_id, current, failures) else {
        return;
    };
    if denominator <= 0.0 {
        failures.push(BudgetFailure {
            id: rule.id.clone(),
            message: format!("denominator {denominator_id} must be positive"),
        });
        return;
    }
    let ratio = numerator / denominator;
    if ratio > max_amplification_x {
        failures.push(BudgetFailure {
            id: rule.id.clone(),
            message: format!(
                "amplification {ratio:.2}x exceeds budget {max_amplification_x:.2}x ({numerator_id} / {denominator_id})"
            ),
        });
    }
}

fn measurement(
    source: &str,
    id: &str,
    measurements: &BenchMeasurements,
    failures: &mut Vec<BudgetFailure>,
) -> Option<f64> {
    let Some(measurement) = measurements.measurements.get(id) else {
        failures.push(BudgetFailure {
            id: id.to_owned(),
            message: format!("missing {source} measurement"),
        });
        return None;
    };
    if !measurement.mean_ns.is_finite() || measurement.mean_ns < 0.0 {
        failures.push(BudgetFailure {
            id: id.to_owned(),
            message: format!("invalid {source} mean_ns: {}", measurement.mean_ns),
        });
        return None;
    }
    Some(measurement.mean_ns)
}
