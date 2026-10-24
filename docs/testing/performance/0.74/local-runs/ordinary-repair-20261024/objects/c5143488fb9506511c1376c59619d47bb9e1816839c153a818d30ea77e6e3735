//! Owned-context reference and ordered borrowed comparisons. No live-result reuse.
use super::*;
use ed25519_dalek::SigningKey;

#[derive(Clone, Copy)]
pub(crate) enum Binding {
    Reference,
    Proposed,
    Actual,
}
#[derive(Clone)]
pub(crate) struct BindingFixture {
    record: Record,
    trust: WorkerPolicyTrust,
    host: AssertedWorkerHost,
}
fn reference(
    record: &Record,
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<(), AuthorityError> {
    if record.issuer_key_hex != key_hex(trust)
        || record.policy_sha256 != trust.policy_sha256
        || record.policy_epoch != trust.epoch
        || record.context() != *host
    {
        return Err(AuthorityError::Binding);
    }
    Ok(())
}
fn proposed(
    record: &Record,
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<(), AuthorityError> {
    if record.issuer_key_hex != key_hex(trust)
        || record.policy_sha256 != trust.policy_sha256
        || record.policy_epoch != trust.epoch
        || record.machine_id != host.machine_id
        || record.boot_id != host.boot_id
        || record.user_namespace != host.user_namespace
        || record.mount_namespace != host.mount_namespace
    {
        return Err(AuthorityError::Binding);
    }
    Ok(())
}
fn change_first(text: &mut String) {
    text.replace_range(..1, if text.starts_with('a') { "b" } else { "a" });
}
impl BindingFixture {
    pub(crate) fn new(case: &str) -> Self {
        let policy = super::tests::checked(7);
        let mut fixture = Self {
            record: Record::from_checked(&policy).unwrap(),
            trust: policy.trust,
            host: policy.context,
        };
        match case {
            "healthy" => (),
            "issuer-mismatch" => {
                fixture.trust.key = SigningKey::from_bytes(&[75; 32]).verifying_key();
                change_first(&mut fixture.host.machine_id);
            }
            "epoch-mismatch" => {
                fixture.trust.epoch += 1;
                change_first(&mut fixture.host.machine_id);
            }
            "context-mismatch" => change_first(&mut fixture.host.machine_id),
            _ => panic!("unknown bounded binding fixture"),
        }
        fixture
    }
    pub(crate) fn check(&self, mode: Binding) -> Result<(), AuthorityError> {
        match mode {
            Binding::Reference => reference(&self.record, &self.trust, &self.host),
            Binding::Proposed => proposed(&self.record, &self.trust, &self.host),
            Binding::Actual => self.record.bind(&self.trust, &self.host),
        }
    }
}
#[test]
fn authority_context_seeded_fields_pins_and_refusal_classes_match_owned_reference() {
    let mut seed = 0x074c_07e5_u64;
    let mut fixture = BindingFixture::new("healthy");
    for _ in 0..256 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        fixture.record.machine_id = crate::hex(&seed.to_le_bytes()).repeat(2);
        fixture.record.boot_id = format!(
            "{:08x}-{:04x}-1111-2222-{:012x}",
            seed as u32,
            (seed >> 32) as u16,
            seed & 0xffff_ffff_ffff
        );
        fixture.record.user_namespace = super::super::NamespaceIdentity {
            device: seed.max(1),
            inode: seed.rotate_left(13).max(1),
        };
        fixture.record.mount_namespace = super::super::NamespaceIdentity {
            device: seed.rotate_right(7).max(1),
            inode: seed.rotate_left(29).max(1),
        };
        fixture.host = fixture.record.context();
        let bytes = fixture.record.bytes().unwrap();
        assert_eq!(Record::parse(&bytes).unwrap(), fixture.record);
        for case in 0..17 {
            let mut changed = fixture.clone();
            match case {
                0 => (),
                1 => changed.trust.key = SigningKey::from_bytes(&[75; 32]).verifying_key(),
                2 => change_first(&mut changed.trust.policy_sha256),
                3 => changed.trust.epoch += 1,
                4 => change_first(&mut changed.host.machine_id),
                5 => change_first(&mut changed.host.boot_id),
                6 => changed.host.user_namespace.device ^= 1,
                7 => changed.host.user_namespace.inode ^= 1,
                8 => changed.host.mount_namespace.device ^= 1,
                9 => changed.host.mount_namespace.inode ^= 1,
                10 => {
                    change_first(&mut changed.host.machine_id);
                    change_first(&mut changed.host.boot_id);
                    changed.host.mount_namespace.inode ^= 1;
                }
                11 => {
                    changed.trust.key = SigningKey::from_bytes(&[75; 32]).verifying_key();
                    change_first(&mut changed.host.machine_id);
                }
                12 => {
                    changed.trust.epoch += 1;
                    change_first(&mut changed.host.machine_id);
                }
                13 => {
                    change_first(&mut changed.trust.policy_sha256);
                    change_first(&mut changed.host.machine_id);
                }
                14 => changed.host.machine_id.clear(),
                15 => changed.host.boot_id = "не boot 🚫".into(),
                _ => {
                    changed.host.user_namespace.device = 0;
                    changed.host.user_namespace.inode = 0;
                    changed.host.mount_namespace.device = 0;
                    changed.host.mount_namespace.inode = 0;
                }
            }
            for mode in [Binding::Reference, Binding::Proposed, Binding::Actual] {
                let result = changed.check(mode);
                assert_eq!(result.is_ok(), case == 0);
                if case != 0 {
                    assert!(matches!(result, Err(AuthorityError::Binding)));
                }
            }
            assert_eq!(changed.record.bytes().unwrap(), bytes);
            assert_eq!(
                sha256_hex(&changed.record.bytes().unwrap()),
                sha256_hex(&bytes)
            );
        }
    }
    eprintln!("authority context equivalence seed=0x074c07e5 contexts=256 cases_per_context=17");
}
#[test]
fn authority_context_bounded_fixtures_keep_prefix_and_context_failures_distinct() {
    for case in [
        "healthy",
        "issuer-mismatch",
        "epoch-mismatch",
        "context-mismatch",
    ] {
        let fixture = BindingFixture::new(case);
        for mode in [Binding::Reference, Binding::Proposed, Binding::Actual] {
            assert_eq!(fixture.check(mode).is_ok(), case == "healthy");
        }
    }
    assert!(std::panic::catch_unwind(|| BindingFixture::new("unknown")).is_err());
}
