use hydracache_long_run_supervisor_074::diagnostic_signer_gnu::inspect_gnu_declarations;

fn p16(b: &mut [u8], at: usize, n: u16) {
    b[at..at + 2].copy_from_slice(&n.to_le_bytes());
}
fn p32(b: &mut [u8], at: usize, n: u32) {
    b[at..at + 4].copy_from_slice(&n.to_le_bytes());
}
fn p64(b: &mut [u8], at: usize, n: u64) {
    b[at..at + 8].copy_from_slice(&n.to_le_bytes());
}

// Independent synthetic byte vectors, not compiled helpers or executable inputs.
fn image(interpreter: bool, needed: &[&str], soname: Option<&str>) -> Vec<u8> {
    let mut b = vec![0u8; 4096];
    b[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    p16(&mut b, 16, 3);
    p16(&mut b, 18, 62);
    p32(&mut b, 20, 1);
    p64(&mut b, 24, 0x10200);
    p64(&mut b, 32, 64);
    p16(&mut b, 52, 64);
    p16(&mut b, 54, 56);
    let loader = b"/lib64/ld-linux-x86-64.so.2\0";
    let mut table = vec![0u8];
    let mut tags = Vec::new();
    for name in needed {
        tags.push((1u64, table.len() as u64));
        table.extend_from_slice(name.as_bytes());
        table.push(0);
    }
    if let Some(name) = soname {
        tags.push((14, table.len() as u64));
        table.extend_from_slice(name.as_bytes());
        table.push(0);
    }
    tags.extend([(5, 0x10800), (10, table.len() as u64), (0, 0)]);
    let mut headers = vec![
        (1u32, 5u32, 0u64, 4096u64, 4096u64),
        (2, 4, 1024, (tags.len() * 16) as u64, 8),
        (0x6474_e551, 6, 0, 0, 16),
    ];
    if interpreter {
        headers.push((3, 4, 768, loader.len() as u64, 1));
    }
    p16(&mut b, 56, headers.len() as u16);
    for (i, (tag, flags, offset, bytes, align)) in headers.into_iter().enumerate() {
        let at = 64 + i * 56;
        p32(&mut b, at, tag);
        p32(&mut b, at + 4, flags);
        p64(&mut b, at + 8, offset);
        p64(
            &mut b,
            at + 16,
            if bytes == 0 { 0 } else { 0x10000 + offset },
        );
        p64(&mut b, at + 32, bytes);
        p64(&mut b, at + 40, bytes);
        p64(&mut b, at + 48, align);
    }
    if interpreter {
        b[768..768 + loader.len()].copy_from_slice(loader);
    }
    for (i, (tag, value)) in tags.into_iter().enumerate() {
        p64(&mut b, 1024 + i * 16, tag);
        p64(&mut b, 1032 + i * 16, value);
    }
    b[2048..2048 + table.len()].copy_from_slice(&table);
    b
}
fn images() -> [Vec<u8>; 4] {
    [
        image(true, &["libc.so.6", "libgcc_s.so.1"], None),
        image(false, &[], Some("ld-linux-x86-64.so.2")),
        image(false, &["ld-linux-x86-64.so.2"], Some("libc.so.6")),
        image(false, &["libc.so.6"], Some("libgcc_s.so.1")),
    ]
}
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
