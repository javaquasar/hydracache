use hydracache_long_run_supervisor_074::diagnostic_signer_gnu::inspect_gnu_declarations;

#[path = "support/diagnostic_gnu_images.rs"]
mod images;
use images::{image, images, p16, p32, p64};
fn check(b: &[Vec<u8>; 4]) -> std::io::Result<serde_json::Value> {
    Ok(serde_json::to_value(inspect_gnu_declarations(b.each_ref().map(Vec::as_slice))?).unwrap())
}

#[test]
fn synthetic_catalog_binds_each_role_and_bytes_without_authority() {
    let b = images();
    let c = check(&b).unwrap();
    assert_eq!(
        c["schema_version"],
        "diagnostic-signer-gnu-declarations-074-v1"
    );
    assert_eq!(c["images"].as_array().unwrap().len(), 4);
    for (i, name) in [
        "signer",
        "ld-linux-x86-64.so.2",
        "libc.so.6",
        "libgcc_s.so.1",
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(c["images"][i]["role"], *name);
        assert_eq!(
            c["images"][i]["sha256"],
            hydracache_long_run_supervisor_074::diagnostic_artifacts::ArtifactDigest::of(&b[i])
                .sha256
        );
        assert_eq!(c["images"][i]["bytes"], b[i].len());
    }
    for flag in [
        "complete_elf_validation",
        "loaded_closure_proved",
        "runtime_custody_authenticated",
        "original_start_authenticated",
        "signing_allowed",
        "admission_allowed",
    ] {
        assert_eq!(c[flag], false, "{flag}");
    }
}
#[test]
fn roles_wrong_soname_and_missing_dependencies_refuse() {
    for case in 0..6 {
        let mut b = images();
        match case {
            0 => b.swap(1, 2),
            1 => b[0] = image(true, &["libc.so.6"], None),
            2 => b[1] = image(false, &["libc.so.6"], Some("ld-linux-x86-64.so.2")),
            3 => b[2] = image(false, &["foreign.so"], Some("libc.so.6")),
            4 => b[0] = image(true, &["libc.so.6", "libgcc_s.so.1", "signer"], None),
            _ => b[0] = image(true, &["libc.so.6", "libgcc_s.so.1"], Some("signer")),
        }
        assert!(check(&b).is_err(), "case={case}");
    }
}
#[test]
fn scripts_short_oversized_abi_and_interpreter_refuse() {
    for index in 0..4 {
        for cut in 0..64 {
            let mut b = images();
            b[index].truncate(cut);
            assert!(check(&b).is_err());
        }
        let mut b = images();
        b[index] = vec![0; 8 * 1024 * 1024 + 1];
        assert!(check(&b).is_err());
        let mut b = images();
        b[index] = b"#!/bin/sh\nexit 0\n".to_vec();
        assert!(check(&b).is_err());
        let mut b = images();
        p16(&mut b[index], 18, 183);
        assert!(check(&b).is_err());
    }
    let mut b = images();
    b[0][768] = b'.';
    assert!(check(&b).is_err());
}
#[test]
fn search_paths_duplicate_needed_writable_exec_and_unknown_tags_refuse() {
    for tag in [15, 29, 22, u64::MAX] {
        let mut b = images();
        p64(&mut b[0], 1024, tag);
        assert!(check(&b).is_err());
    }
    let mut b = images();
    b[0] = image(true, &["libc.so.6", "libc.so.6", "libgcc_s.so.1"], None);
    assert!(check(&b).is_err());
    let mut b = images();
    p32(&mut b[0], 68, 7);
    assert!(check(&b).is_err());
    let mut b = images();
    p64(&mut b[0], 72, u64::MAX);
    assert!(check(&b).is_err());
}
#[test]
fn unchanged_declarations_still_record_changed_bytes() {
    let mut b = images();
    let original = check(&b).unwrap();
    b[0][4095] = 1;
    let changed = check(&b).unwrap();
    assert_ne!(
        original["images"][0]["sha256"],
        changed["images"][0]["sha256"]
    );
    assert_eq!(
        original["images"][0]["needed"],
        changed["images"][0]["needed"]
    );
    assert_eq!(changed["runtime_custody_authenticated"], false);
}
#[test]
fn seeded_mutations_do_not_panic_and_independent_readers_agree() {
    let original = images();
    let mut seed = 0x0740_c076u64;
    for _ in 0..256 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut b = original.clone();
        let role = seed as usize % 4;
        let offset = (seed >> 2) as usize % b[role].len();
        b[role][offset] ^= 1;
        assert!(std::panic::catch_unwind(|| check(&b)).is_ok());
    }
    let readers: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                let b = images();
                check(&b).unwrap()
            })
        })
        .collect();
    for reader in readers {
        assert_eq!(reader.join().unwrap(), check(&original).unwrap());
    }
    println!("seed=0x0740c076 mutations=256 independent_readers=4 authority=false");
}
