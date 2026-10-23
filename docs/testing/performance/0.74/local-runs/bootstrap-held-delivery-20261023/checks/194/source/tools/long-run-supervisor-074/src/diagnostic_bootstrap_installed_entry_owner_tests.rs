use super::*;
struct Probe {
    events: Vec<&'static str>,
    refused: bool,
    retained: Option<u64>,
    cut: usize,
    drift: usize,
    resource: bool,
    origins: [u64; 4],
}
impl Probe {
    fn new() -> Self {
        Self {
            events: vec![],
            refused: false,
            retained: None,
            cut: 0,
            drift: 0,
            resource: false,
            origins: [1, 2, 3, 4],
        }
    }
    fn step(&mut self, name: &'static str) -> Result<(), BootstrapError> {
        self.events.push(name);
        if self.events.len() == self.cut {
            Err(BootstrapError::Binding)
        } else {
            Ok(())
        }
    }
}
impl Checks for Probe {
    type Observation = u64;
    fn refused(&self) -> bool {
        self.refused
    }
    fn full_frame(&mut self) -> Result<(), BootstrapError> {
        self.step("old-installed-complete-frame")
    }
    fn inspect(&mut self) -> Result<u64, BootstrapError> {
        self.step("same-old-entry-and-new-record")?;
        if !self.resource {
            if self.retained.is_some() {
                return Err(BootstrapError::Binding);
            }
            self.resource = true;
        }
        Ok(if self.events.len() == self.drift {
            75
        } else {
            74
        })
    }
    fn retained(&self) -> Option<&u64> {
        self.retained.as_ref()
    }
    fn refuse(&mut self) {
        self.refused = true;
    }
}
#[test]
fn bootstrap_installed_material_owner_keeps_frames_origins_and_no_retained_recreation() {
    let mut p = Probe::new();
    p.retained = Some(validate(&mut p).unwrap());
    validate(&mut p).unwrap();
    assert_eq!(
        p.events,
        [
            "old-installed-complete-frame",
            "same-old-entry-and-new-record",
            "old-installed-complete-frame",
            "same-old-entry-and-new-record",
            "old-installed-complete-frame"
        ]
        .repeat(2)
    );
    assert_eq!(p.origins, [1, 2, 3, 4]);
    p.resource = false;
    assert!(validate(&mut p).is_err());
    assert!(!p.resource && p.refused);
    let at = p.events.len();
    p.resource = true;
    assert!(validate(&mut p).is_err());
    assert_eq!(p.events.len(), at);
}
#[test]
fn bootstrap_installed_material_owner_every_cut_and_drift_latch() {
    for cut in 1..=5 {
        let mut p = Probe::new();
        p.cut = cut;
        assert!(validate(&mut p).is_err());
        assert!(p.refused);
        assert_eq!(p.events.len(), cut);
        p.cut = 0;
        assert!(validate(&mut p).is_err());
        assert_eq!(p.events.len(), cut);
    }
    for cut in [2, 4] {
        let mut p = Probe::new();
        p.drift = cut;
        assert!(validate(&mut p).is_err());
        assert!(p.refused);
    }
}
#[test]
fn bootstrap_installed_material_owner_api_consumes_without_execution_or_extraction() {
    fn contract(owner: InstalledBootstrapUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {
        match owner.materialize_installed_entry() {
            Ok(mut held) => {
                let _ = held.revalidate();
            }
            Err(error) => {
                assert!(error.is_refused());
                let _ = error.error();
            }
        }
    }
    let _contract = contract;
}
