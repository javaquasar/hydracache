use super::*;

fn fixture() -> AssertedInstalledEntry {
    AssertedInstalledEntry {
        original: AssertedEntry {
            owner: [74, 75, 0, 0, 0, 0],
            origins: [1, 2, 3, 4],
            bounds: [300, 60, 10, 10],
            attempt: [1; 32],
            original_context: [2; 32],
            root: [1, 2, 10],
            serial: 1,
            names: [[3; 32]; 2],
            documents: [[4; 32]; 7],
            identities: std::array::from_fn(|i| [1, i as u64 + 2, if i == 5 { 256 } else { 128 }]),
            streams: [[1, 2]; 4],
            guardians: [[76, 1, 3], [77, 1, 4]],
            image_hashes: [[5; 32]; 5],
            legacy_hash: [6; 32],
            census_hash: [7; 32],
            state_hash: [8; 32],
        },
        installed_identity: [1, 99, 128],
        installed_sha256: [5; 32],
        parent_inventory_sha256: [9; 32],
    }
}

#[test]
fn bootstrap_installed_entry_exact_layout_embeds_old_bytes_and_origins() {
    let value = fixture();
    let raw = value.encode().unwrap();
    assert_eq!(raw.len(), 1280);
    assert_eq!(&raw[..8], b"HCBI0741");
    assert_eq!(&raw[16..1040], value.original.encode().unwrap());
    assert_eq!(&raw[1040..1048], 1_u64.to_le_bytes());
    assert_eq!(&raw[1048..1056], 99_u64.to_le_bytes());
    assert_eq!(&raw[1056..1064], 128_u64.to_le_bytes());
    assert_eq!(&raw[1064..1096], [5; 32]);
    assert_eq!(&raw[1096..1128], &Sha256::digest(INSTALLED_PATH)[..]);
    assert_eq!(&raw[1128..1160], [9; 32]);
    assert_eq!(&raw[1160..1164], 32_u32.to_le_bytes());
    assert_eq!(&raw[1164..1168], 16_u32.to_le_bytes());
    assert_eq!(
        &raw[1168..1200],
        &Sha256::digest(value.original.encode().unwrap())[..]
    );
    assert_eq!(AssertedInstalledEntry::decode(&raw).unwrap(), value);
}

#[test]
fn bootstrap_installed_entry_every_single_bit_change_is_canonical_or_refused() {
    let value = fixture();
    let raw = value.encode().unwrap();
    for at in 0..BYTES {
        for bit in 0..8 {
            let mut changed = raw;
            changed[at] ^= 1 << bit;
            if let Ok(parsed) = AssertedInstalledEntry::decode(&changed) {
                assert_ne!(parsed, value, "invisible byte {at} bit {bit}");
                assert_eq!(parsed.encode().unwrap(), changed);
            }
        }
    }
}

#[test]
fn bootstrap_installed_entry_refuses_lengths_versions_padding_fixed_counts_and_hashes() {
    let raw = fixture().encode().unwrap();
    for n in [0, 1, 256, 1024, 1279, 1281, 2560] {
        let mut bad = raw.to_vec();
        bad.resize(n, 0);
        assert!(AssertedInstalledEntry::decode(&bad).is_err());
    }
    for at in (0..16)
        .chain(1096..1128)
        .chain(1160..1200)
        .chain(1200..1280)
    {
        let mut bad = raw;
        bad[at] ^= 1;
        assert!(AssertedInstalledEntry::decode(&bad).is_err(), "byte {at}");
    }
    for at in [16, 24, 28, 236, 239, 400] {
        let mut bad = raw;
        bad[at] ^= 1;
        assert!(AssertedInstalledEntry::decode(&bad).is_err());
    }
}

#[test]
fn bootstrap_installed_entry_refuses_alias_size_content_and_zero_identity() {
    let value = fixture();
    for id in value.original.identities {
        let mut bad = value.clone();
        bad.installed_identity[..2].copy_from_slice(&id[..2]);
        assert!(bad.encode().is_err());
    }
    for at in 0..3 {
        let mut bad = value.clone();
        bad.installed_identity[at] = 0;
        assert!(bad.encode().is_err());
    }
    let mut bad = value.clone();
    bad.installed_identity[2] += 1;
    assert!(bad.encode().is_err());
    let mut bad = value.clone();
    bad.installed_sha256[0] ^= 1;
    assert!(bad.encode().is_err());
    for bytes in [128 * 1024 * 1024, 128 * 1024 * 1024 + 1] {
        let mut changed = value.clone();
        changed.installed_identity[2] = bytes;
        changed.original.identities[2][2] = bytes;
        assert_eq!(changed.encode().is_ok(), bytes == 128 * 1024 * 1024);
    }
    let mut different_device = value;
    different_device.installed_identity[..2].copy_from_slice(&[2, 4]);
    assert!(
        different_device.encode().is_ok(),
        "inode number alone is not identity"
    );
}

#[test]
fn bootstrap_installed_entry_preserves_embedded_shape_and_deadline_refusals() {
    for at in 0..4 {
        let mut bad = fixture();
        bad.original.bounds[at] = 0;
        assert!(bad.encode().is_err());
    }
    let mut bad = fixture();
    bad.original.root[2] += 1;
    assert!(bad.encode().is_err());
    let mut bad = fixture();
    bad.original.owner[0] = 1;
    assert!(bad.encode().is_err());
    let mut bad = fixture();
    bad.original.serial = 0;
    assert!(bad.encode().is_err());
    // Recompute the embedded digest: it must not hide a noncanonical old entry.
    let mut raw = fixture().encode().unwrap();
    raw[16 + 220] = 1;
    let digest = Sha256::digest(&raw[16..1040]);
    raw[1168..1200].copy_from_slice(&digest);
    assert!(AssertedInstalledEntry::decode(&raw).is_err());
    // Recompute again for a structurally valid but cross-image mismatch.
    let mut raw = fixture().encode().unwrap();
    raw[16 + 832] ^= 1;
    let digest = Sha256::digest(&raw[16..1040]);
    raw[1168..1200].copy_from_slice(&digest);
    assert!(AssertedInstalledEntry::decode(&raw).is_err());
}

#[test]
fn bootstrap_installed_entry_canonical_forgery_is_not_authenticated() {
    let mut forged = fixture();
    forged.original.owner[0] += 1;
    forged.original.origins = [900, 901, 902, 903];
    forged.original.attempt = [0; 32];
    forged.original.documents = [[0; 32]; 7];
    forged.parent_inventory_sha256 = [0; 32];
    // A parser cannot attest these assertions to a live owner, policy or clocks.
    assert_eq!(
        AssertedInstalledEntry::decode(&forged.encode().unwrap()).unwrap(),
        forged
    );
}

#[test]
fn bootstrap_installed_entry_no_old_reader_fallback_or_magic_reinterpretation() {
    let value = fixture();
    let new = value.encode().unwrap();
    let old = value.original.encode().unwrap();
    assert!(AssertedEntry::decode(&new).is_err());
    assert!(AssertedInstalledEntry::decode(&old).is_err());
    let mut changed = new;
    changed[..8].copy_from_slice(b"HCBT0741");
    assert!(AssertedInstalledEntry::decode(&changed).is_err());
    assert!(AssertedEntry::decode(&changed[..1024]).is_err());
}
