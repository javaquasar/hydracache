use super::*;
struct Probe {
    events: Vec<&'static str>,
    failed: usize,
    drift: usize,
    retained: Option<u64>,
    refused: bool,
    origins: [u64; 4],
}
impl Probe {
    fn new() -> Self {
        Self {
            events: vec![],
            failed: 0,
            drift: 0,
            retained: None,
            refused: false,
            origins: [1, 2, 3, 4],
        }
    }
    fn step(&mut self, name: &'static str) -> Result<(), BootstrapError> {
        self.events.push(name);
        if self.events.len() == self.failed {
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
        self.step("old-complete-bootstrap-frame")
    }
    fn inspect(&mut self) -> Result<u64, BootstrapError> {
        self.step("installed-source-and-old-entry-census")?;
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
fn bootstrap_installed_owner_models_keep_three_old_full_frames_and_original_origins() {
    let mut p = Probe::new();
    p.retained = Some(validate(&mut p).unwrap());
    validate(&mut p).unwrap();
    assert_eq!(
        p.events,
        [
            "old-complete-bootstrap-frame",
            "installed-source-and-old-entry-census",
            "old-complete-bootstrap-frame",
            "installed-source-and-old-entry-census",
            "old-complete-bootstrap-frame"
        ]
        .repeat(2)
    );
    assert_eq!(p.origins, [1, 2, 3, 4]);
}
#[test]
fn bootstrap_installed_owner_models_every_cut_and_retained_drift_latch_without_retry() {
    for cut in 1..=5 {
        let mut p = Probe::new();
        p.failed = cut;
        assert!(validate(&mut p).is_err());
        assert_eq!(p.events.len(), cut);
        assert!(p.refused);
        p.failed = 0;
        assert!(validate(&mut p).is_err());
        assert_eq!(p.events.len(), cut);
    }
    for cut in [2, 4] {
        let mut p = Probe::new();
        p.drift = cut;
        assert!(validate(&mut p).is_err());
        assert!(p.refused);
    }
    let mut p = Probe::new();
    p.retained = Some(73);
    assert!(validate(&mut p).is_err());
    assert_eq!(p.events.len(), 2);
}
#[test]
fn bootstrap_installed_owner_production_api_consumes_original_without_extraction() {
    fn contract(owner: BootstrapOwnedUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {
        match owner.bind_installed_origin() {
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
