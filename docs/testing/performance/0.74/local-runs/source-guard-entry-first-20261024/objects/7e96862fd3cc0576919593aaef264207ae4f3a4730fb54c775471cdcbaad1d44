#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod gate;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;
fn read(path: &str) -> String {
    std::fs::read_to_string(gate::root().join(path))
        .unwrap()
        .replace("\r\n", "\n")
}
fn paths() -> Vec<String> {
    let mut paths: Vec<_> = [
        gate::FIXTURE,
        gate::OBSERVER,
        gate::TESTS,
        gate::SOURCE_TEST,
        gate::HISTORY_TEST,
    ]
    .iter()
    .map(|path| (*path).to_owned())
    .collect();
    paths.extend(
        std::iter::once(gate::performance::MAIN.to_owned()).chain(
            gate::performance::READERS
                .iter()
                .map(|(path, _)| (*path).to_owned()),
        ),
    );
    paths.extend(
        gate::COMPAT
            .iter()
            .map(|name| format!("tools/long-run-supervisor-074/tests/support/{name}")),
    );
    paths
}
#[test]
fn installed_runtime_is_the_exact_candidate_and_explicit_compatibility_set() {
    for path in paths() {
        assert!(
            gate::accepts(&path, &read(&path)),
            "exact installed file {path}"
        );
    }
    for (artifact, installed) in [
        ("candidate.rs", gate::FIXTURE),
        ("observer.rs", gate::OBSERVER),
    ] {
        let path = format!("tools/authority-storage-source-074/{artifact}");
        assert_eq!(read(&path), gate::frozen(gate::ARTIFACT, &path));
        let expected: proc_macro2::TokenStream = read(&path).parse().unwrap();
        let actual: proc_macro2::TokenStream = read(installed).parse().unwrap();
        assert_eq!(actual.to_string(), expected.to_string());
    }
}
#[test]
fn baseline_partial_install_and_unrelated_effects_refuse_before_restoration() {
    for path in paths() {
        let raw = read(&path);
        assert!(!gate::accepts(
            &path,
            &format!("{raw}\nfn unexpected_effect() {{}}")
        ));
        assert!(std::panic::catch_unwind(|| gate::view(
            &path,
            format!("{raw}\nfn unexpected_effect() {{}}")
        ))
        .is_err());
    }
    for path in [gate::FIXTURE, gate::OBSERVER] {
        assert!(
            !gate::accepts(path, &gate::frozen(gate::BASE, path)),
            "baseline is not an installed candidate"
        );
    }
    let path = "tools/long-run-supervisor-074/src/unrelated.rs";
    assert!(!gate::accepts(path, "unrelated"));
    assert_eq!(
        gate::view(path, "unrelated drift".into()),
        "unrelated drift"
    );
}
#[test]
fn approved_predecessor_views_do_not_replace_current_runtime_validation() {
    for path in paths() {
        let revision = if path == gate::SOURCE_TEST {
            gate::ARTIFACT
        } else if path == gate::HISTORY_TEST {
            gate::HISTORY
        } else {
            gate::BASE
        };
        assert_eq!(
            gate::view(&path, read(&path)),
            gate::frozen(revision, &path)
        );
    }
    let raw = read(gate::FIXTURE);
    for (old, new) in [
        ("raw.clear();", ""),
        ("self.latch_refusal();", "self.failed = true;"),
        ("regular(&before, uid, gid, 0o400)?;", ""),
        ("self.directory.leaf.sync_all()?;", ""),
        ("named.2.capacity() <= budget", "true"),
        ("entry.marker.evict_scratch();", ""),
    ] {
        for (at, _) in raw.match_indices(old) {
            let mut changed = raw.clone();
            changed.replace_range(at..at + old.len(), new);
            assert!(!gate::accepts(gate::FIXTURE, &changed));
        }
    }
}
#[test]
fn legacy_adapters_preserve_complete_prior_test_bodies_and_assertions() {
    for (path, rev, count) in [
        (gate::SOURCE_TEST, gate::ARTIFACT, 8),
        (gate::HISTORY_TEST, gate::HISTORY, 10),
    ] {
        let original = gate::frozen(rev, path);
        let expected = gate::legacy_adapter(path, &original);
        let actual = read(path);
        assert_eq!(
            expected
                .parse::<proc_macro2::TokenStream>()
                .unwrap()
                .to_string(),
            actual
                .parse::<proc_macro2::TokenStream>()
                .unwrap()
                .to_string()
        );
        let parsed = syn::parse_file(&actual).unwrap();
        let tests = parsed.items.into_iter().filter(|item| matches!(item, syn::Item::Fn(function) if function.attrs.iter().any(|attr| attr.path().is_ident("test")))).count();
        assert_eq!(tests, count);
    }
}

#[test]
fn full_contract_adapters_change_only_one_preservation_reader_per_module() {
    assert_eq!(gate::performance::READERS.len(), 58);
    for (path, reader) in gate::performance::READERS {
        let original = gate::frozen(gate::BASE, path);
        let installed = read(path);
        let expected = gate::performance::reader(&original, reader);
        assert!(storage_contract::accepts(&expected, &installed));
        assert_eq!(
            original.matches("#[test]").count(),
            installed.matches("#[test]").count()
        );
        assert!(!gate::accepts(
            path,
            &installed.replace("assert_eq!", "assert_ne!")
        ));
        assert_eq!(gate::view(path, installed), original);
    }
}
