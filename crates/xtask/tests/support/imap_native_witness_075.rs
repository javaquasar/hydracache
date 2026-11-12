use super::*;

#[test]
fn witness_replay_checks_final_state_independently_of_the_search() {
    let mut fixture: Histories = serde_json::from_str(include_str!(
        "../../../../docs/testing/imap/0.75/native-concurrent-history-fixture-v1.json"
    ))
    .unwrap();
    validate(&fixture).unwrap();
    let history = &mut fixture.histories[0];
    assert_eq!(validate_witness(history).unwrap().len(), 12);
    history.final_entries[0].value = "ff".into();
    history.final_digest =
        crate::imap_qualification_075::digest(&[0, 0, 0, 1, 0x61, 0, 0, 0, 1, 0xff, 0]);
    assert_eq!(check(history, 100_000).unwrap(), Verdict::NonLinearizable);
    assert_eq!(
        validate_witness(history).err().unwrap().to_string(),
        "native witness final state differs"
    );
    assert!(validate(&fixture).is_err());
}
