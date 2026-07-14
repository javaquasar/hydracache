//! Documentation-consistency checks for the release manifest.
//!
//! Validates `docs/plans/releases.toml` (the authoritative release registry, see
//! `docs/RULES.md` R-11):
//!
//! - every `file` referenced by an entry exists on disk;
//! - no two non-draft / non-superseded entries share a `version`;
//! - every `depends_on` value resolves to a `version` present in the manifest;
//! - `status` is one of the allowed values;
//! - `version = "TBD"` is only allowed for `draft` / `superseded` entries.
//! - a shipped `0.43.0` entry must explicitly confirm that the networked control
//!   plane is wired, so a modeled-vs-networked gap cannot be marked shipped by
//!   accident.
//! - every in-prose `V0_*.md` plan reference under `docs/plans/` resolves to a
//!   real plan file.
//! - every manifest plan that carries a strict `**Status:**` header has that
//!   header agree with its manifest `status` (TD-0006). A header that drifts
//!   (e.g. a shipped plan still saying "planned") fails the gate instead of
//!   passing silently. Plans that use a prose status (drafts) are skipped.
//! - when a release declares `work_items`, the manifest list exactly matches
//!   the plan's W-item headings and `INDEX.md` carries the generated coverage
//!   marker, so a plan expansion cannot disappear from the release registry.
//! - every shipped non-legacy release in `releases.toml` has a matching
//!   `docs/releases/<version>.md` note so GitHub release publishing cannot pass
//!   with missing public notes.
//! - every ADR uses the single `0001-title.md` filename scheme, has a unique
//!   number, and is listed from `docs/adr/README.md`.
//! - every publishable workspace crate is present in both release publish scripts.
//!
//! This turns the "release sequencing is recorded, not implied" rule into an
//! executable gate so doc drift (e.g. two plans claiming the same version, or a
//! plan referencing a sibling that no longer exists) fails CI instead of silently
//! rotting.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

const ALLOWED_STATUS: [&str; 5] = ["shipped", "in-progress", "planned", "draft", "superseded"];
const ALLOWED_REDIS_COMPAT_STATUS: [&str; 6] = [
    "supported",
    "supported_with_caveat",
    "candidate",
    "admin_disabled",
    "hydracache_extension",
    "unsupported",
];
const ALLOWED_REDIS_COMPAT_ORACLE: [&str; 7] = [
    "exact",
    "normalized_error",
    "normalized_metadata",
    "documented_divergence",
    "hydracache_only",
    "ttl_tolerance",
    "candidate",
];

#[derive(serde::Deserialize)]
struct Manifest {
    #[serde(default)]
    release: Vec<Release>,
}

#[derive(serde::Deserialize)]
struct Release {
    version: String,
    file: String,
    status: String,
    #[serde(default)]
    #[allow(dead_code)]
    theme: String,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    networked_control_plane: Option<bool>,
    #[serde(default)]
    work_items: Vec<String>,
}

#[derive(serde::Deserialize)]
struct RedisCompatManifest {
    version: u16,
    surface: String,
    supported_resp: String,
    redis_oracle: RedisOracle,
    #[serde(default)]
    commands: Vec<RedisCompatCommand>,
}

#[derive(serde::Deserialize)]
struct RedisOracle {
    #[serde(default)]
    images: Vec<String>,
    #[serde(default)]
    normalization: String,
}

#[derive(serde::Deserialize)]
struct RedisCompatCommand {
    name: String,
    status: String,
    #[serde(default)]
    kind: String,
    oracle: String,
    #[serde(default)]
    tests: Vec<String>,
}

#[derive(serde::Deserialize)]
struct WorkspaceManifest {
    workspace: WorkspaceTable,
}

#[derive(serde::Deserialize)]
struct WorkspaceTable {
    #[serde(default)]
    members: Vec<String>,
}

#[derive(serde::Deserialize)]
struct PackageManifest {
    package: PackageTable,
}

#[derive(serde::Deserialize)]
struct PackageTable {
    name: String,
    #[serde(default = "default_publish_setting")]
    publish: PublishSetting,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum PublishSetting {
    Bool(bool),
    Registries(Vec<String>),
}

fn default_publish_setting() -> PublishSetting {
    PublishSetting::Bool(true)
}

impl PublishSetting {
    fn is_publishable(&self) -> bool {
        match self {
            PublishSetting::Bool(value) => *value,
            PublishSetting::Registries(registries) => !registries.is_empty(),
        }
    }
}

/// Locate the repository root by ascending from the cargo manifest dir and the
/// current directory until `docs/plans/releases.toml` is found.
pub fn find_repo_root() -> Result<PathBuf, Box<dyn Error>> {
    let mut starts: Vec<PathBuf> = Vec::new();
    if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        starts.push(PathBuf::from(dir));
    }
    if let Ok(dir) = std::env::current_dir() {
        starts.push(dir);
    }
    for start in starts {
        let mut dir: &Path = start.as_path();
        loop {
            if dir.join("docs/plans/releases.toml").is_file() {
                return Ok(dir.to_path_buf());
            }
            match dir.parent() {
                Some(parent) => dir = parent,
                None => break,
            }
        }
    }
    Err("could not locate repo root (docs/plans/releases.toml not found)".into())
}

/// Validate the manifest under `root`. Returns the list of problems found (empty =
/// consistent). `Err` is reserved for IO / parse failures.
pub fn check(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let manifest_path = root.join("docs/plans/releases.toml");
    let text = fs::read_to_string(&manifest_path)
        .map_err(|err| format!("reading {}: {err}", manifest_path.display()))?;
    let manifest: Manifest = toml::from_str(&text)
        .map_err(|err| format!("parsing {}: {err}", manifest_path.display()))?;

    let mut problems = Vec::new();
    let known_versions: HashSet<&str> = manifest
        .release
        .iter()
        .map(|r| r.version.as_str())
        .collect();
    let mut active_versions: HashMap<&str, ()> = HashMap::new();

    for r in &manifest.release {
        if !ALLOWED_STATUS.contains(&r.status.as_str()) {
            problems.push(format!(
                "{}: invalid status '{}' (allowed: {})",
                r.file,
                r.status,
                ALLOWED_STATUS.join(", ")
            ));
        }
        let is_draftish = r.status == "draft" || r.status == "superseded";

        if r.version == "TBD" && !is_draftish {
            problems.push(format!(
                "{}: version 'TBD' is only allowed for draft/superseded entries",
                r.file
            ));
        }

        if r.status == "shipped" && r.networked_control_plane == Some(false) {
            problems.push(format!(
                "{}: shipped release cannot set networked_control_plane = false",
                r.file
            ));
        }

        if r.version == "0.43.0" && r.status == "shipped" && r.networked_control_plane != Some(true)
        {
            problems.push(format!(
                "{}: shipped 0.43.0 must set networked_control_plane = true",
                r.file
            ));
        }

        if !root.join(&r.file).is_file() {
            problems.push(format!("{}: file does not exist", r.file));
        }

        if r.version != "TBD"
            && !is_draftish
            && active_versions.insert(r.version.as_str(), ()).is_some()
        {
            problems.push(format!(
                "duplicate version '{}' among active (non-draft) plans",
                r.version
            ));
        }

        for dep in &r.depends_on {
            if !known_versions.contains(dep.as_str()) {
                problems.push(format!(
                    "{}: depends_on '{}' does not match any version in the manifest",
                    r.file, dep
                ));
            }
        }
    }

    problems.extend(check_plan_header_status(root, &manifest.release)?);
    problems.extend(check_release_work_items(root, &manifest.release)?);
    problems.extend(check_release_notes_for_shipped_releases(
        root,
        &manifest.release,
    )?);
    problems.extend(check_in_prose_plan_links(root)?);
    problems.extend(check_adr_index(root)?);
    problems.extend(check_publishable_crates_in_publish_scripts(root)?);
    problems.extend(check_redis_compat_conformance(root)?);
    problems.extend(check_redis_compat_docs_examples(root)?);

    Ok(problems)
}

fn check_release_work_items(
    root: &Path,
    releases: &[Release],
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut problems = Vec::new();
    let index_path = root.join("docs/plans/INDEX.md");
    let index = if releases
        .iter()
        .any(|release| !release.work_items.is_empty())
    {
        match fs::read_to_string(&index_path) {
            Ok(text) => Some(text),
            Err(err) => {
                problems.push(format!(
                    "docs/plans/INDEX.md: required by release work_items but could not be read: {err}"
                ));
                None
            }
        }
    } else {
        None
    };

    for release in releases {
        if release.work_items.is_empty() {
            continue;
        }

        let mut declared = BTreeSet::new();
        for item in &release.work_items {
            if !is_work_item_id(item) {
                problems.push(format!(
                    "{}: invalid work item '{}' (expected W<number> with an optional lowercase suffix)",
                    release.file, item
                ));
            }
            if !declared.insert(item.as_str()) {
                problems.push(format!(
                    "{}: duplicate work item '{}' in releases.toml",
                    release.file, item
                ));
            }
        }

        let plan_path = root.join(&release.file);
        if plan_path.is_file() {
            let plan = fs::read_to_string(&plan_path)
                .map_err(|err| format!("reading {}: {err}", plan_path.display()))?;
            let headings = extract_work_item_headings(&plan);

            for item in &declared {
                if !headings.contains(*item) {
                    problems.push(format!(
                        "{}: manifest work item '{}' has no matching plan heading",
                        release.file, item
                    ));
                }
            }
            for item in headings {
                if !declared.contains(item.as_str()) {
                    problems.push(format!(
                        "{}: plan work item '{}' is missing from releases.toml work_items",
                        release.file, item
                    ));
                }
            }
        }

        if let Some(index) = &index {
            let marker = release_work_item_marker(release);
            if !index.contains(&marker) {
                problems.push(format!(
                    "docs/plans/INDEX.md: missing release work-item marker '{marker}'"
                ));
            }
        }
    }

    Ok(problems)
}

fn extract_work_item_headings(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let hash_count = line.chars().take_while(|ch| *ch == '#').count();
            if !(2..=4).contains(&hash_count) {
                return None;
            }
            let heading = line[hash_count..].trim_start();
            let item = heading.split_once('.')?.0;
            is_work_item_id(item).then(|| item.to_owned())
        })
        .collect()
}

fn is_work_item_id(item: &str) -> bool {
    let Some(rest) = item.strip_prefix('W') else {
        return false;
    };
    let digit_count = rest.chars().take_while(|ch| ch.is_ascii_digit()).count();
    if digit_count == 0 {
        return false;
    }
    let suffix = &rest[digit_count..];
    suffix.is_empty() || (suffix.len() == 1 && suffix.as_bytes()[0].is_ascii_lowercase())
}

fn release_work_item_marker(release: &Release) -> String {
    format!(
        "<!-- release-work-items:{}={} -->",
        release.version,
        release.work_items.join(",")
    )
}

fn check_release_notes_for_shipped_releases(
    root: &Path,
    releases: &[Release],
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut problems = Vec::new();
    for r in releases {
        if r.status != "shipped" || r.version == "TBD" {
            continue;
        }

        let note = format!("docs/releases/{}.md", r.version);
        if !root.join(&note).is_file() {
            problems.push(format!(
                "{}: shipped release '{}' is missing {}",
                r.file, r.version, note
            ));
        }
    }

    Ok(problems)
}

/// Validate that each manifest plan's `> - **Status:** <value>` header agrees with
/// the `status` recorded for it in `releases.toml` (TD-0006). `doc-check` already
/// keeps `releases.toml` and `INDEX.md` in sync, but the in-plan header could drift
/// silently (e.g. a shipped plan still reading "planned"). A plan that uses a prose
/// status line instead of the strict `**Status:**` marker (e.g. an idea-capture
/// draft) is skipped rather than flagged, so only real drift fails the gate.
fn check_plan_header_status(
    root: &Path,
    releases: &[Release],
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut problems = Vec::new();
    for r in releases {
        let path = root.join(&r.file);
        if !path.is_file() {
            continue; // missing-file is already reported by check().
        }
        let text = fs::read_to_string(&path)
            .map_err(|err| format!("reading {}: {err}", path.display()))?;
        if let Some(header) = parse_status_header(&text) {
            if !header.eq_ignore_ascii_case(&r.status) {
                problems.push(format!(
                    "{}: plan header status '{}' does not match manifest status '{}'",
                    r.file, header, r.status
                ));
            }
        }
    }
    Ok(problems)
}

/// Extract the leading status token from a strict `**Status:** <value>` header
/// line. Only the first `[A-Za-z0-9-]+` run is taken, so an annotated header such
/// as `**Status:** shipped — <prose>` or `**Status:** planned.` still yields the
/// bare status word (`shipped`, `in-progress`, `planned`, …). Returns `None` when
/// no such marker is present (prose-status plans are intentionally not matched).
fn parse_status_header(text: &str) -> Option<String> {
    const MARKER: &str = "**Status:**";
    for line in text.lines() {
        if let Some(idx) = line.find(MARKER) {
            let token: String = line[idx + MARKER.len()..]
                .trim_start()
                .chars()
                .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
                .collect();
            if !token.is_empty() {
                return Some(token);
            }
        }
    }
    None
}

fn check_publishable_crates_in_publish_scripts(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let workspace_manifest_path = root.join("Cargo.toml");
    if !workspace_manifest_path.is_file() {
        return Ok(Vec::new());
    }

    let workspace_text = fs::read_to_string(&workspace_manifest_path)
        .map_err(|err| format!("reading {}: {err}", workspace_manifest_path.display()))?;
    let workspace: WorkspaceManifest = toml::from_str(&workspace_text)
        .map_err(|err| format!("parsing {}: {err}", workspace_manifest_path.display()))?;

    let mut problems = Vec::new();
    let mut publishable = BTreeSet::new();
    for member in workspace.workspace.members {
        let manifest_path = root.join(&member).join("Cargo.toml");
        if !manifest_path.is_file() {
            problems.push(format!(
                "{member}: workspace member Cargo.toml does not exist"
            ));
            continue;
        }

        let text = fs::read_to_string(&manifest_path)
            .map_err(|err| format!("reading {}: {err}", manifest_path.display()))?;
        let manifest: PackageManifest = toml::from_str(&text)
            .map_err(|err| format!("parsing {}: {err}", manifest_path.display()))?;
        if manifest.package.publish.is_publishable() {
            publishable.insert(manifest.package.name);
        }
    }

    for script in [
        "scripts/package-publishable.ps1",
        "scripts/verify-release-readiness.ps1",
    ] {
        let path = root.join(script);
        let text = fs::read_to_string(&path)
            .map_err(|err| format!("reading {}: {err}", path.display()))?;
        let listed = extract_quoted_hydracache_packages(&text);
        for package in publishable.difference(&listed) {
            problems.push(format!(
                "{script}: publishable crate '{package}' is missing from the publish package list"
            ));
        }
    }

    Ok(problems)
}

fn extract_quoted_hydracache_packages(text: &str) -> BTreeSet<String> {
    text.split('"')
        .skip(1)
        .step_by(2)
        .filter(|value| *value == "hydracache" || value.starts_with("hydracache-"))
        .map(ToOwned::to_owned)
        .collect()
}

fn check_redis_compat_conformance(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let manifest_path = root.join("docs/integrations/redis_compat_conformance.json");
    if !manifest_path.is_file() {
        return Ok(Vec::new());
    }

    let docs_path = root.join("docs/integrations/redis-compat.md");
    let text = fs::read_to_string(&manifest_path)
        .map_err(|err| format!("reading {}: {err}", manifest_path.display()))?;
    let manifest: RedisCompatManifest = serde_json::from_str(&text)
        .map_err(|err| format!("parsing {}: {err}", manifest_path.display()))?;

    let mut problems = Vec::new();
    if !docs_path.is_file() {
        problems.push("docs/integrations/redis-compat.md: file does not exist".to_owned());
    }
    if manifest.version != 1 {
        problems.push(format!(
            "docs/integrations/redis_compat_conformance.json: unsupported manifest version {}",
            manifest.version
        ));
    }
    if manifest.surface != "hydracache-redis-resp-edge" {
        problems.push(format!(
            "docs/integrations/redis_compat_conformance.json: unexpected surface '{}'",
            manifest.surface
        ));
    }
    if manifest.supported_resp != "RESP2+RESP3" {
        problems.push(format!(
            "docs/integrations/redis_compat_conformance.json: supported_resp must be RESP2+RESP3 for 0.63.0, got '{}'",
            manifest.supported_resp
        ));
    }
    if manifest.redis_oracle.images.is_empty() {
        problems.push(
            "docs/integrations/redis_compat_conformance.json: redis_oracle.images must not be empty"
                .to_owned(),
        );
    }
    if manifest.redis_oracle.normalization.trim().is_empty() {
        problems.push(
            "docs/integrations/redis_compat_conformance.json: redis_oracle.normalization must not be empty"
                .to_owned(),
        );
    }
    for image in &manifest.redis_oracle.images {
        if image.trim().is_empty() {
            problems.push(
                "docs/integrations/redis_compat_conformance.json: redis_oracle image must not be empty"
                    .to_owned(),
            );
        }
        if image.ends_with(":latest") || image == "redis" {
            problems.push(format!(
                "docs/integrations/redis_compat_conformance.json: redis_oracle image '{image}' must be pinned, not latest"
            ));
        }
    }
    if manifest.commands.is_empty() {
        problems.push(
            "docs/integrations/redis_compat_conformance.json: commands must not be empty"
                .to_owned(),
        );
    }

    let has_deployment_scope = manifest
        .commands
        .iter()
        .any(|command| command.kind == "deployment_scope");
    let redis_multinode_test_path =
        root.join("crates/hydracache-server/tests/redis_resp_multinode.rs");
    let redis_multinode_tests = if has_deployment_scope {
        match fs::read_to_string(&redis_multinode_test_path) {
            Ok(text) => text,
            Err(err) => {
                problems.push(format!(
                    "crates/hydracache-server/tests/redis_resp_multinode.rs: required for Redis deployment_scope rows but could not be read: {err}"
                ));
                String::new()
            }
        }
    } else {
        String::new()
    };

    let mut names = HashSet::new();
    for command in &manifest.commands {
        let source = format!(
            "docs/integrations/redis_compat_conformance.json command '{}'",
            command.name
        );
        if command.name.trim().is_empty() {
            problems.push(format!("{source}: name must not be empty"));
        }
        if !names.insert(command.name.to_ascii_uppercase()) {
            problems.push(format!("{source}: duplicate command name"));
        }
        if !ALLOWED_REDIS_COMPAT_STATUS.contains(&command.status.as_str()) {
            problems.push(format!(
                "{source}: invalid status '{}' (allowed: {})",
                command.status,
                ALLOWED_REDIS_COMPAT_STATUS.join(", ")
            ));
        }
        if !ALLOWED_REDIS_COMPAT_ORACLE.contains(&command.oracle.as_str()) {
            problems.push(format!(
                "{source}: invalid oracle '{}' (allowed: {})",
                command.oracle,
                ALLOWED_REDIS_COMPAT_ORACLE.join(", ")
            ));
        }
        if command.kind.trim().is_empty() {
            problems.push(format!("{source}: kind must not be empty"));
        }

        let requires_tests = !matches!(command.status.as_str(), "unsupported");
        if requires_tests && command.tests.is_empty() {
            problems.push(format!(
                "{source}: status '{}' requires at least one covering test",
                command.status
            ));
        }
        if command
            .tests
            .iter()
            .any(|test| test.trim().is_empty() || test.contains(char::is_whitespace))
        {
            problems.push(format!(
                "{source}: test names must be non-empty identifiers"
            ));
        }

        if command.kind == "deployment_scope" {
            if command.oracle != "documented_divergence" {
                problems.push(format!(
                    "{source}: deployment_scope rows must use documented_divergence oracle"
                ));
            }
            if command.tests.is_empty() {
                problems.push(format!(
                    "{source}: deployment_scope rows require at least one multinode sentinel test"
                ));
            }
            for test in &command.tests {
                if !redis_multinode_tests.contains(&format!("fn {test}(")) {
                    problems.push(format!(
                        "{source}: deployment_scope test '{test}' must be implemented in crates/hydracache-server/tests/redis_resp_multinode.rs"
                    ));
                }
            }
        }

        if matches!(
            command.status.as_str(),
            "supported" | "supported_with_caveat"
        ) && matches!(
            command.oracle.as_str(),
            "candidate" | "documented_divergence" | "hydracache_only"
        ) {
            problems.push(format!(
                "{source}: supported Redis command cannot use oracle '{}'",
                command.oracle
            ));
        }

        if command.name.to_ascii_uppercase().starts_with("HC.")
            && !matches!(
                command.oracle.as_str(),
                "hydracache_only" | "candidate" | "documented_divergence"
            )
        {
            problems.push(format!(
                "{source}: HC.* command must use hydracache_only, candidate, or documented_divergence oracle"
            ));
        }
    }

    Ok(problems)
}

fn check_redis_compat_docs_examples(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let docs_path = root.join("docs/integrations/redis-compat.md");
    if !docs_path.is_file() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&docs_path)
        .map_err(|err| format!("reading {}: {err}", docs_path.display()))?;
    if !text.contains("## Executable Examples") {
        return Ok(Vec::new());
    }

    let mut problems = Vec::new();
    for heading in [
        "### redis-cli",
        "### Rust (redis-rs)",
        "### Python (redis-py)",
        "### Node (node-redis)",
        "### Go (go-redis)",
        "### JVM (Jedis)",
    ] {
        let Some(section) = section_after_heading(&text, heading) else {
            problems.push(format!(
                "docs/integrations/redis-compat.md: missing executable example section '{heading}'"
            ));
            continue;
        };
        if !section.contains("Gate: `redis_clients`") {
            problems.push(format!(
                "docs/integrations/redis-compat.md: example section '{heading}' must name Gate: `redis_clients`"
            ));
        }
        if !section.contains("```") {
            problems.push(format!(
                "docs/integrations/redis-compat.md: example section '{heading}' must include a fenced code block"
            ));
        }
    }
    Ok(problems)
}

fn section_after_heading<'a>(text: &'a str, heading: &str) -> Option<&'a str> {
    let start = text.find(heading)?;
    let rest = &text[start + heading.len()..];
    let end = rest.find("\n### ").unwrap_or(rest.len());
    Some(&rest[..end])
}

fn check_in_prose_plan_links(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let plan_dir = root.join("docs/plans");
    if !plan_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut problems = Vec::new();
    for entry in
        fs::read_dir(&plan_dir).map_err(|err| format!("reading {}: {err}", plan_dir.display()))?
    {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let text = fs::read_to_string(&path)
            .map_err(|err| format!("reading {}: {err}", path.display()))?;
        let source = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string();
        let mut seen = HashSet::new();
        for link in extract_plan_links(&text) {
            if !seen.insert(link.clone()) {
                continue;
            }
            if !plan_dir.join(&link).is_file() {
                problems.push(format!("{source}: references missing plan '{link}'"));
            }
        }
    }

    Ok(problems)
}

fn extract_plan_links(text: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut search_from = 0;

    while let Some(offset) = text[search_from..].find("V0_") {
        let start = search_from + offset;
        let rest = &text[start..];
        let Some(md_offset) = rest.find(".md") else {
            break;
        };
        let end = start + md_offset + ".md".len();
        let candidate = &text[start..end];
        if is_plan_filename(candidate) {
            links.push(candidate.to_owned());
            search_from = end;
        } else {
            search_from = start + "V0_".len();
        }
    }

    links
}

fn is_plan_filename(candidate: &str) -> bool {
    candidate.ends_with(".md")
        && candidate
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
}

fn check_adr_index(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let adr_dir = root.join("docs/adr");
    if !adr_dir.is_dir() {
        return Ok(Vec::new());
    }

    let readme_path = adr_dir.join("README.md");
    let readme = fs::read_to_string(&readme_path)
        .map_err(|err| format!("reading {}: {err}", readme_path.display()))?;

    let mut problems = Vec::new();
    let mut numbers: HashMap<String, String> = HashMap::new();
    for entry in
        fs::read_dir(&adr_dir).map_err(|err| format!("reading {}: {err}", adr_dir.display()))?
    {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if file_name == "README.md" {
            continue;
        }

        let Some(number) = adr_number(file_name) else {
            problems.push(format!(
                "docs/adr/{file_name}: ADR filename must use NNNN-title.md"
            ));
            continue;
        };
        if let Some(existing) = numbers.insert(number.clone(), file_name.to_owned()) {
            problems.push(format!(
                "docs/adr/{file_name}: duplicate ADR number {number} already used by {existing}"
            ));
        }
        if !readme.contains(file_name) {
            problems.push(format!(
                "docs/adr/README.md: missing ADR index entry for {file_name}"
            ));
        }

        let text = fs::read_to_string(&path)
            .map_err(|err| format!("reading {}: {err}", path.display()))?;
        let expected_heading = format!("# ADR-{number}:");
        if !text.starts_with(&expected_heading) {
            problems.push(format!(
                "docs/adr/{file_name}: heading must start with '{expected_heading}'"
            ));
        }
    }

    Ok(problems)
}

fn adr_number(file_name: &str) -> Option<String> {
    let bytes = file_name.as_bytes();
    if bytes.len() <= 8 || bytes.get(4) != Some(&b'-') || !file_name.ends_with(".md") {
        return None;
    }
    let number = &file_name[..4];
    if number.bytes().all(|byte| byte.is_ascii_digit()) {
        Some(number.to_owned())
    } else {
        None
    }
}

/// CLI entry point: `cargo xtask doc-check [--root <path>]`.
pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let mut root: Option<PathBuf> = None;
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--root" => {
                root = Some(PathBuf::from(
                    it.next().ok_or("--root requires a path argument")?,
                ))
            }
            other => return Err(format!("unknown doc-check argument: {other}").into()),
        }
    }

    let root = match root {
        Some(root) => root,
        None => find_repo_root()?,
    };

    let problems = check(&root)?;
    if problems.is_empty() {
        println!("doc-check: OK (releases.toml is consistent)");
        Ok(())
    } else {
        for problem in &problems {
            eprintln!("doc-check: {problem}");
        }
        Err(format!("doc-check found {} problem(s)", problems.len()).into())
    }
}

#[cfg(test)]
mod tests {
    use super::parse_status_header;

    #[test]
    fn parses_status_header_trimming_trailing_period() {
        let text = "# Plan\n\n> - **What:** x\n> - **Status:** shipped.\n";
        assert_eq!(parse_status_header(text).as_deref(), Some("shipped"));
    }

    #[test]
    fn parses_in_progress_value() {
        assert_eq!(
            parse_status_header("> - **Status:** in-progress.").as_deref(),
            Some("in-progress")
        );
    }

    #[test]
    fn takes_only_the_leading_token_from_an_annotated_header() {
        assert_eq!(
            parse_status_header("> - **Status:** shipped — Phase F gates validate the claim.")
                .as_deref(),
            Some("shipped")
        );
    }

    #[test]
    fn prose_status_and_missing_header_return_none() {
        // Draft-style prose status uses `**Status: DRAFT ...**`, not the strict marker.
        assert_eq!(
            parse_status_header("> **Status: DRAFT — version TBD.**"),
            None
        );
        assert_eq!(parse_status_header("no status header at all"), None);
    }
}
