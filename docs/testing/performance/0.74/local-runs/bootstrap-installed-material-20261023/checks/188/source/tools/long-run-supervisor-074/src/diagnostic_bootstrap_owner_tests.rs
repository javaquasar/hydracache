use super::*;
struct Probe {
    events: Vec<&'static str>,
    fail: usize,
    drift: usize,
    refused: bool,
    retained: Option<u64>,
    retired: usize,
}
impl Probe {
    fn new() -> Self {
        Self {
            events: vec![],
            fail: 0,
            drift: 0,
            refused: false,
            retained: None,
            retired: 0,
        }
    }
    fn call(&mut self, event: &'static str) -> Result<(), BootstrapError> {
        self.events.push(event);
        if self.events.len() == self.fail {
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
        self.call("complete-original-frame")
    }
    fn inspect(&mut self) -> Result<u64, BootstrapError> {
        self.call("source-signatures-context-sealed-image")?;
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
        self.retired += 1;
        self.refused = true;
    }
}
#[test]
fn bootstrap_owner_model_closes_after_both_hashing_rounds() {
    let mut p = Probe::new();
    let observation = validate(&mut p).unwrap();
    p.retained = Some(observation);
    validate(&mut p).unwrap();
    assert_eq!(
        p.events,
        [
            "complete-original-frame",
            "source-signatures-context-sealed-image",
            "complete-original-frame",
            "source-signatures-context-sealed-image",
            "complete-original-frame"
        ]
        .repeat(2)
    );
    assert_eq!(p.retired, 0);
}
#[test]
fn bootstrap_owner_model_every_cut_is_sticky_and_retires_same_owner() {
    for cut in 1..=5 {
        let mut p = Probe::new();
        p.fail = cut;
        assert!(validate(&mut p).is_err());
        assert!(p.refused);
        assert_eq!(p.events.len(), cut);
        assert_eq!(p.retired, 1);
        p.fail = 0;
        assert!(validate(&mut p).is_err());
        assert_eq!(p.events.len(), cut);
    }
}
#[test]
fn bootstrap_owner_model_context_image_or_retained_drift_cannot_rebind() {
    for drift in [2, 4] {
        let mut p = Probe::new();
        p.drift = drift;
        assert!(validate(&mut p).is_err());
        assert!(p.refused);
    }
    let mut p = Probe::new();
    p.retained = Some(73);
    assert!(validate(&mut p).is_err());
    assert_eq!(p.events.len(), 2);
    assert!(p.refused);
}
