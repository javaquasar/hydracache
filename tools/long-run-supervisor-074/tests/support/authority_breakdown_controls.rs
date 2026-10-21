//! Test-only decomposition. No phase alone is an authority or start capability.
use super::super::tests::checked;
use super::*;

#[derive(Clone, Copy)]
pub(crate) enum Phase {
    Full,
    Directory,
    Inventory,
    Records,
    Markers,
    Bind,
    Scan,
    Expected,
    Path,
}
pub(crate) const PHASES: [(&str, Phase); 9] = [
    ("full", Phase::Full),
    ("directory", Phase::Directory),
    ("inventory", Phase::Inventory),
    ("records", Phase::Records),
    ("markers", Phase::Markers),
    ("bind", Phase::Bind),
    ("scan", Phase::Scan),
    ("expected", Phase::Expected),
    ("path", Phase::Path),
];

fn scan(directory: &Directory) -> Result<BTreeSet<String>, AuthorityError> {
    let mut seen = BTreeSet::new();
    for entry in fs::read_dir(format!("/proc/self/fd/{}", directory.leaf.as_raw_fd()))? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| AuthorityError::Invalid)?;
        if !seen.insert(name) || seen.len() > MAX_GENERATIONS * 2 {
            return Err(AuthorityError::Budget);
        }
    }
    if seen.len() % 2 != 0 {
        return Err(AuthorityError::Invalid);
    }
    Ok(seen)
}

pub(crate) struct Fixture {
    // Drop the original locked owner before its synthetic temporary tree.
    registry: FixtureAuthorityRegistry,
    policy: CheckedWorkerPolicy,
    observed: BTreeSet<String>,
    depth: usize,
    root: tempfile::TempDir,
}
impl Fixture {
    pub(crate) fn new(depth: usize) -> Self {
        assert!(matches!(depth, 1 | 7));
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        assert!(uid > 0 && gid > 0);
        let root = tempfile::Builder::new()
            .prefix("hydracache-authority-")
            .tempdir_in("/tmp")
            .unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut policy = checked(7);
        let mut registry = initialize_fixture_registry(root.path(), uid, gid, &policy).unwrap();
        for epoch in 8..8 + (depth / 2) as u64 {
            registry.refuse(RefusalReason::Operator).unwrap();
            policy = checked(epoch);
            registry.advance(&policy).unwrap();
        }
        let observed = scan(&registry.directory).unwrap();
        assert_eq!(registry.view().unwrap().revision, depth as u64);
        Self {
            registry,
            policy,
            observed,
            depth,
            root,
        }
    }
    pub(crate) fn run(&mut self, phase: Phase) -> Result<(), AuthorityError> {
        match phase {
            Phase::Full => {
                let view = self
                    .registry
                    .inspect(&self.policy.trust, &self.policy.context)?;
                assert_eq!(view.revision, self.depth as u64);
                assert!(!view.refused);
                std::hint::black_box(view);
            }
            Phase::Directory => self.registry.directory.revalidate()?,
            Phase::Inventory => {
                assert_eq!(self.registry.directory.inventory()?, self.depth);
            }
            Phase::Records => {
                for entry in &self.registry.entries {
                    entry.record.revalidate(
                        &self.registry.directory.leaf,
                        self.registry.directory.uid,
                        self.registry.directory.gid,
                        MAX_RECORD_BYTES,
                    )?;
                }
            }
            Phase::Markers => {
                for entry in &self.registry.entries {
                    entry.marker.revalidate(
                        &self.registry.directory.leaf,
                        self.registry.directory.uid,
                        self.registry.directory.gid,
                        COMMIT_MARKER_BYTES,
                    )?;
                }
            }
            Phase::Bind => {
                let record = self
                    .registry
                    .records
                    .last()
                    .ok_or(AuthorityError::Invalid)?;
                record.bind(&self.policy.trust, &self.policy.context)?;
                std::hint::black_box(record.view());
            }
            Phase::Scan => {
                let observed = scan(&self.registry.directory)?;
                assert_eq!(observed.len(), self.depth * 2);
                std::hint::black_box(observed);
            }
            Phase::Expected => {
                assert!(Directory::expected_inventory(&self.observed, self.depth));
            }
            Phase::Path => {
                std::hint::black_box(format!(
                    "/proc/self/fd/{}",
                    self.registry.directory.leaf.as_raw_fd()
                ));
            }
        }
        Ok(())
    }
    #[cfg(feature = "fixed-leaf-alloc-probe")]
    pub(crate) fn identity(&self) -> serde_json::Value {
        serde_json::json!({
            "generations":self.depth,
            "revision":self.registry.view().unwrap().revision,
            "epoch":self.policy.trust.epoch,
            "root_name_bytes":self.root.path().file_name().unwrap().len(),
            "leaf_fd":self.registry.directory.leaf.as_raw_fd(),
            "entries":self.registry.entries.iter().enumerate().map(|(index, entry)| {
                serde_json::json!({
                    "revision":index+1,
                    "record_name_bytes":entry.record.name.len(),
                    "marker_name_bytes":entry.marker.name.len(),
                    "record_bytes":entry.record.raw.len(),
                    "marker_bytes":entry.marker.raw.len(),
                    "record_sha256":sha256_hex(&entry.record.raw),
                    "marker_sha256":sha256_hex(&entry.marker.raw),
                })
            }).collect::<Vec<_>>(),
        })
    }
}

#[test]
fn authority_breakdown_actual_phases_are_healthy_and_stale_expected_is_not_admission() {
    for depth in [1, 7] {
        let mut fixture = Fixture::new(depth);
        for (_, phase) in PHASES {
            fixture.run(phase).unwrap();
        }
        fs::remove_file(fixture.root.path().join(name(depth, "commit"))).unwrap();
        // This deliberately stale pure control is still true, not a live check.
        fixture.run(Phase::Expected).unwrap();
        assert!(fixture.run(Phase::Inventory).is_err());
        assert!(fixture.run(Phase::Full).is_err());
        assert!(fixture.registry.is_failed());
        assert!(matches!(
            fixture.run(Phase::Full),
            Err(AuthorityError::Refused)
        ));
        assert!(!fixture.root.path().join(name(depth, "commit")).exists());
    }
}
