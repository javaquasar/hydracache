//! Owned expected names versus identical formatting into a bounded stack buffer.
use super::*;

pub(crate) fn reference(seen: &BTreeSet<String>, generations: usize) -> bool {
    seen.iter()
        .map(String::as_str)
        .eq((1..=generations)
            .flat_map(|revision| [name(revision, "commit"), name(revision, "json")]))
}

pub(crate) fn proposed(seen: &BTreeSet<String>, generations: usize) -> bool {
    let mut observed = seen.iter();
    for revision in 1..=generations {
        for suffix in ["commit", "json"] {
            let mut expected = std::io::Cursor::new([0_u8; 23]);
            write!(&mut expected, "{revision:016x}.{suffix}")
                .expect("bounded journal name fits 23 bytes");
            let length = expected.position() as usize;
            if observed.next().map(|name| name.as_bytes()) != Some(&expected.get_ref()[..length]) {
                return false;
            }
        }
    }
    observed.next().is_none()
}

#[test]
fn authority_expected_name_differential_depths_mutations_and_cardinality() {
    use super::inventory_controls::observed;
    let mut seed = 0x074e_1a9e_u64;
    for depth in 0..=MAX_GENERATIONS {
        let seen = observed(depth);
        assert!(reference(&seen, depth));
        assert!(proposed(&seen, depth));
        assert!(!proposed(&seen, depth + 1));
        if depth > 0 {
            assert!(!proposed(&seen, depth - 1));
        }
        for _ in 0..16 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let mut mutated = seen.clone();
            let revision = 1 + seed as usize % (depth + 1);
            let suffix = if seed & 1 == 0 { "json" } else { "commit" };
            mutated.remove(&name(revision, suffix));
            mutated.insert(format!("{seed:016X}.{suffix}"));
            assert_eq!(reference(&mutated, depth), proposed(&mutated, depth));
        }
    }
    for depth in [1, 7, MAX_GENERATIONS] {
        for revision in [1, depth / 2 + 1, depth] {
            for suffix in ["commit", "json"] {
                let original = name(revision, suffix);
                for position in 0..original.len() {
                    let mut changed = original.as_bytes().to_vec();
                    changed[position] = if changed[position] == b'X' {
                        b'Y'
                    } else {
                        b'X'
                    };
                    let mut seen = observed(depth);
                    seen.remove(&original);
                    seen.insert(String::from_utf8(changed).unwrap());
                    assert!(!reference(&seen, depth));
                    assert!(!proposed(&seen, depth));
                }
                for malformed in [
                    "",
                    "1.json",
                    "0000000000000000.json",
                    "é",
                    "\0",
                    "0000000000000001.JSON",
                    "0000000000000001.commit.extra",
                ] {
                    let mut seen = observed(depth);
                    seen.remove(&original);
                    seen.insert(malformed.into());
                    assert_eq!(reference(&seen, depth), proposed(&seen, depth));
                }
                let mut missing = observed(depth);
                missing.remove(&original);
                assert!(!proposed(&missing, depth));
                let mut extra = observed(depth);
                extra.insert("extra".into());
                assert!(!proposed(&extra, depth));
            }
        }
    }
    eprintln!("expected-name seed=0x074e1a9e depths=0..128 mutations=16/depth");
}

#[test]
fn authority_expected_name_stack_capacity_matches_owned_formatter() {
    for revision in (0..=MAX_GENERATIONS).chain([0xff, 0x100, 0xffff, usize::MAX]) {
        for suffix in ["commit", "json"] {
            let mut expected = std::io::Cursor::new([0_u8; 23]);
            write!(&mut expected, "{revision:016x}.{suffix}").unwrap();
            let length = expected.position() as usize;
            assert_eq!(
                &expected.get_ref()[..length],
                name(revision, suffix).as_bytes()
            );
            assert_eq!(length, 17 + suffix.len());
        }
    }
}
