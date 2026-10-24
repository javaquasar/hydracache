//! One exact synthetic lock-fixture delta; all production provider bytes remain.
#[path = "../../../../tools/material-lock-fixture-074/contract.rs"]
pub(crate) mod contract;
use crate::census_runtime::{census_caller, census_reference, transport};
use std::time::Instant;

const REVIEWED: &str = "c91068bfa8b297bb075b63457da235aaa9c6232e";

fn frozen(revision: &str, deadline: Instant) -> String {
    assert!(revision == REVIEWED || revision == contract::BASE);
    let reference = census_reference::reference(revision, contract::SOURCE)
        .unwrap_or_else(|failure| panic!("census adapter refusal: {failure:?}"));
    census_caller::panic_api::frozen(census_caller::read(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        &format!("{revision}:{}", contract::SOURCE),
        &reference,
        deadline,
    ))
}

pub(crate) fn original() -> String {
    frozen(contract::BASE, Instant::now() + transport::BOUND)
}

pub(crate) fn accepts(original: &str, raw: &str) -> bool {
    accepts_until(original, raw, Instant::now() + transport::BOUND)
}

fn accepts_until(original: &str, raw: &str, deadline: Instant) -> bool {
    raw.replace("\r\n", "\n") == frozen(REVIEWED, deadline) && contract::accepts(original, raw)
}

pub(crate) fn view(name: &str, raw: &str) -> String {
    if name != "diagnostic_artifacts_linux.rs" {
        return raw.to_owned();
    }
    let deadline = Instant::now() + transport::BOUND;
    let helper = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(contract::CONTROL),
    )
    .unwrap();
    assert!(
        contract::helper_matches(&helper),
        "unreviewed fixture control"
    );
    let original = frozen(contract::BASE, deadline);
    assert!(
        accepts_until(&original, raw, deadline),
        "unreviewed provider body"
    );
    original
}
