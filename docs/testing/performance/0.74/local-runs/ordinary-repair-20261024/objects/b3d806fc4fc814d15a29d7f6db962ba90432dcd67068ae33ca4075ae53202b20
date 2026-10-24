//! Expected-set materialization only. No live scan is cached or elided.
use super::*;

pub(crate) fn reference(seen: &BTreeSet<String>, generations: usize) -> bool {
    let expected: BTreeSet<_> = (1..=generations)
        .flat_map(|revision| [name(revision, "json"), name(revision, "commit")])
        .collect();
    *seen == expected
}
pub(crate) fn proposed(seen: &BTreeSet<String>, generations: usize) -> bool {
    seen.iter()
        .map(String::as_str)
        .eq((1..=generations)
            .flat_map(|revision| [name(revision, "commit"), name(revision, "json")]))
}
pub(crate) fn observed(generations: usize) -> BTreeSet<String> {
    (1..=generations)
        .flat_map(|revision| [name(revision, "json"), name(revision, "commit")])
        .collect()
}
fn reference_inventory(leaf: &File) -> Result<usize, AuthorityError> {
    let mut seen = BTreeSet::new();
    for entry in fs::read_dir(format!("/proc/self/fd/{}", leaf.as_raw_fd()))? {
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
    let generations = seen.len() / 2;
    if !reference(&seen, generations) {
        return Err(AuthorityError::Invalid);
    }
    Ok(generations)
}
fn class(value: Result<usize, AuthorityError>) -> (Option<usize>, &'static str, Option<i32>) {
    match value {
        Ok(count) => (Some(count), "ok", None),
        Err(AuthorityError::Invalid) => (None, "invalid", None),
        Err(AuthorityError::Budget) => (None, "budget", None),
        Err(AuthorityError::Io(error)) => (None, "io", error.raw_os_error()),
        Err(_) => panic!("unexpected inventory refusal"),
    }
}
#[test]
fn authority_inventory_expected_names_seeded_mutations_and_exact_cardinality() {
    let mut seed = 0x0741_1e17_u64;
    for generations in 0..=MAX_GENERATIONS {
        let seen = observed(generations);
        assert!(reference(&seen, generations));
        assert!(proposed(&seen, generations));
        // Equality checks length as well as content; neither arm uses bare zip.
        assert!(!proposed(&seen, generations + 1));
        if generations > 0 {
            assert!(!proposed(&seen, generations - 1));
        }
        for _ in 0..16 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let mut mutated = seen.clone();
            let revision = 1 + seed as usize % (generations + 1);
            let suffix = if seed & 1 == 0 { "json" } else { "commit" };
            mutated.remove(&name(revision, suffix));
            mutated.insert(format!("{seed:016X}.{suffix}"));
            assert_eq!(
                reference(&mutated, generations),
                proposed(&mutated, generations)
            );
            let reversed: BTreeSet<_> = mutated.iter().rev().cloned().collect();
            assert_eq!(
                reference(&reversed, generations),
                proposed(&reversed, generations)
            );
        }
    }
    for malformed in [
        "1.json",
        "0000000000000000.json",
        "0000000000000002.json",
        "0000000000000001.JSON",
        "0000000000000001.commit.extra",
        "000000000000000a.json",
        "000000000000000A.json",
        "0000000000000001.jso",
        "",
        "é",
    ] {
        let mut seen = observed(1);
        seen.remove(&name(1, "json"));
        seen.insert(malformed.into());
        assert!(!reference(&seen, 1));
        assert!(!proposed(&seen, 1));
    }
    eprintln!("inventory seed=0x07411e17 depths=0..128 mutations=16/depth");
}
#[test]
fn authority_inventory_actual_filesystem_matches_frozen_scan_and_refusals() {
    use std::os::unix::ffi::OsStringExt;
    assert_ne!(unsafe { libc::geteuid() }, 0);
    for case in 0..9 {
        let root = tempfile::Builder::new()
            .prefix("hydracache-authority-")
            .tempdir_in("/tmp")
            .unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut directory = Directory::open(root.path(), unsafe { libc::geteuid() }, unsafe {
            libc::getegid()
        })
        .unwrap();
        let generations = match case {
            0 => 0,
            1 => 1,
            2 => 7,
            3 => MAX_GENERATIONS,
            4 => MAX_GENERATIONS + 1,
            _ => 1,
        };
        for file in observed(generations).iter().rev() {
            fs::write(root.path().join(file), []).unwrap();
        }
        match case {
            5 => fs::remove_file(root.path().join(name(1, "json"))).unwrap(),
            6 => fs::rename(
                root.path().join(name(1, "json")),
                root.path().join("1.json"),
            )
            .unwrap(),
            7 => fs::rename(
                root.path().join(name(1, "json")),
                root.path().join(std::ffi::OsString::from_vec(vec![0xff])),
            )
            .unwrap(),
            8 => {
                directory.leaf = File::open(root.path().join(name(1, "json"))).unwrap();
            }
            _ => (),
        }
        let expected = class(reference_inventory(&directory.leaf));
        assert_eq!(class(directory.inventory()), expected);
        assert_eq!(
            expected.1,
            match case {
                0..=3 => "ok",
                4 => "budget",
                8 => "io",
                _ => "invalid",
            }
        );
    }
}
