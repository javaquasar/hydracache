#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod contract;
use proc_macro2::{Delimiter, Group, Ident, Punct, Spacing, TokenStream, TokenTree};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn frozen(path: &str) -> String {
    let output = std::process::Command::new("git")
        .current_dir(root())
        .args(["show", &format!("{}:{path}", contract::SOURCE)])
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().replace('\r', "")
}
fn candidate() -> String {
    read("tools/authority-storage-source-074/candidate.rs")
}
fn observer() -> String {
    read("tools/authority-storage-source-074/observer.rs")
}
fn tokens(text: &str) -> TokenStream {
    text.parse().unwrap()
}
fn canonical(stream: TokenStream) -> String {
    fn adjust(stream: TokenStream) -> TokenStream {
        let trees: Vec<_> = stream.into_iter().collect();
        trees
            .iter()
            .enumerate()
            .map(|(index, tree)| match tree {
                TokenTree::Group(group) => {
                    TokenTree::Group(Group::new(group.delimiter(), adjust(group.stream())))
                }
                TokenTree::Punct(punct)
                    if punct.as_char() == '?'
                        && trees.get(index + 1).is_some_and(
                            |next| matches!(next, TokenTree::Punct(dot) if dot.as_char()=='.'),
                        ) =>
                {
                    TokenTree::Punct(Punct::new('?', Spacing::Alone))
                }
                other => other.clone(),
            })
            .collect()
    }
    adjust(stream).to_string()
}
fn body(stream: TokenStream, name: &str) -> TokenStream {
    let trees: Vec<_> = stream.into_iter().collect();
    let mut found = Vec::new();
    for index in 0..trees.len().saturating_sub(1) {
        if trees[index].to_string() == "fn" && trees[index + 1].to_string() == name {
            found.push(
                trees[index + 2..]
                    .iter()
                    .find_map(|tree| match tree {
                        TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                            Some(group.stream())
                        }
                        _ => None,
                    })
                    .unwrap(),
            );
        }
    }
    assert_eq!(found.len(), 1, "unique body {name}");
    found.remove(0)
}
fn implementation(source: &str, name: &str) -> TokenStream {
    let trees: Vec<_> = tokens(source).into_iter().collect();
    let mut found = Vec::new();
    for index in 0..trees.len().saturating_sub(2) {
        if trees[index].to_string() == "impl" && trees[index + 1].to_string() == name {
            let TokenTree::Group(group) = &trees[index + 2] else {
                panic!("impl block")
            };
            assert_eq!(group.delimiter(), Delimiter::Brace);
            found.push(group.stream());
        }
    }
    assert_eq!(
        found.len(),
        if name == "FixtureAuthorityRegistry" {
            2
        } else {
            1
        }
    );
    found.into_iter().collect()
}

// Strip exact expression-only observations, never an arbitrary closure or statement.
fn unhook(stream: TokenStream, sites: &mut Vec<String>) -> TokenStream {
    let trees: Vec<_> = stream.into_iter().collect();
    let mut output = TokenStream::new();
    let mut index = 0;
    while index < trees.len() {
        if index + 3 < trees.len()
            && trees[index].to_string() == "observer"
            && trees[index + 1].to_string() == "."
            && trees[index + 2].to_string() == "around"
        {
            let TokenTree::Group(args) = &trees[index + 3] else {
                panic!("hook args")
            };
            assert_eq!(args.delimiter(), Delimiter::Parenthesis);
            let args: Vec<_> = args.stream().into_iter().collect();
            assert!(args.len() > 7);
            for (position, expected) in [
                (0, "Site"),
                (1, ":"),
                (2, ":"),
                (4, ","),
                (5, "|"),
                (6, "|"),
            ] {
                assert_eq!(args[position].to_string(), expected);
            }
            sites.push(args[3].to_string());
            let mut expression: TokenStream = args[7..].iter().cloned().collect();
            let values: Vec<_> = expression.clone().into_iter().collect();
            if let [TokenTree::Group(group)] = values.as_slice() {
                if group.delimiter() == Delimiter::Brace {
                    let block: syn::Block = syn::parse2(expression.clone()).unwrap();
                    assert_eq!(block.stmts.len(), 1);
                    assert!(matches!(block.stmts[0], syn::Stmt::Expr(_, None)));
                    expression = group.stream();
                }
            }
            output.extend(unhook(expression, sites));
            index += 4;
            continue;
        }
        let helper = trees[index].to_string();
        if matches!(helper.as_str(), "leaf_read_reusing" | "leaf_validate") {
            let TokenTree::Group(args) = &trees[index + 1] else {
                panic!("helper args")
            };
            assert_eq!(args.delimiter(), Delimiter::Parenthesis);
            let mut args: Vec<_> = args.stream().into_iter().collect();
            assert_eq!(args[0].to_string(), "observer");
            assert_eq!(args[1].to_string(), ",");
            if args.last().is_some_and(|token| token.to_string() == ",") {
                args.pop(); // optional trailing comma in this recognized function call only
            }
            let (method, skip) = if helper == "leaf_validate" {
                assert_eq!(args[2].to_string(), "leaf");
                assert_eq!(args[3].to_string(), ",");
                ("self.validate", 4)
            } else {
                ("Self::read_reusing", 2)
            };
            let rest = unhook(args[skip..].iter().cloned().collect(), sites);
            output.extend(tokens(&format!("{method}({rest})")));
            index += 2;
            continue;
        }
        match &trees[index] {
            TokenTree::Group(group) => output.extend([TokenTree::Group(Group::new(
                group.delimiter(),
                unhook(group.stream(), sites),
            ))]),
            TokenTree::Ident(ident) if ident == "leaf" => {
                output.extend([TokenTree::Ident(Ident::new("self", ident.span()))])
            }
            other => output.extend([other.clone()]),
        }
        index += 1;
    }
    output
}
fn check_hooks(control: &str, candidate: &str) {
    let runtime = implementation(candidate, "Leaf");
    for (helper, method, expected_sites) in [
        (
            "leaf_read",
            "read",
            &[
                "LeafOpen",
                "LeafBefore",
                "ParentMetadata",
                "Buffer",
                "Read",
                "After",
            ][..],
        ),
        (
            "leaf_read_reusing",
            "read_reusing",
            &[
                "LeafOpen",
                "LeafBefore",
                "ParentMetadata",
                "Buffer",
                "Read",
                "After",
            ][..],
        ),
        ("leaf_validate", "validate", &["HeldMetadata"][..]),
        ("leaf_revalidate", "revalidate", &[][..]),
    ] {
        let mut sites = Vec::new();
        let restored = unhook(body(tokens(control), helper), &mut sites);
        assert_eq!(sites, expected_sites, "hook order {helper}");
        assert_eq!(
            canonical(restored),
            canonical(body(runtime.clone(), method)),
            "complete body {helper}"
        );
    }
}

#[test]
fn exact_unapplied_files_match_pinned_patch_and_unchanged_runtime() {
    let baseline = frozen(contract::FIXTURE);
    assert_eq!(
        Sha256::digest(baseline.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        "3b3c75f41dbd8a8db1f73d61af3b0c701a666c3ebeed326ae49012ed0c01fb6c"
    );
    assert_eq!(
        read(contract::FIXTURE),
        baseline,
        "candidate is not installed"
    );
    let old_observer = frozen(contract::OBSERVER);
    assert_eq!(
        read(contract::OBSERVER),
        old_observer,
        "old observer remains historical"
    );
    assert!(contract::accepts(
        &contract::candidate(&baseline),
        &candidate()
    ));
    assert!(contract::accepts(
        &contract::observer(&old_observer),
        &observer()
    ));
}

#[test]
fn observer_complete_bodies_restore_candidate_and_exact_hook_order() {
    check_hooks(&observer(), &candidate());
}

#[test]
fn storage_validation_and_effect_mutations_refuse() {
    let original = candidate();
    let expected = contract::candidate(&frozen(contract::FIXTURE));
    let cases = [
        ("regular(&before, uid, gid, 0o400)?;", ""),
        ("before.dev() != dir.metadata()?.dev()", "false"),
        ("before.len() > budget as u64", "false"),
        ("budget as u64 + 1", "budget as u64"),
        ("raw.len() as u64 != stamp.len", "false"),
        ("named.2 != self.raw", "false"),
        ("named.1 != self.stamp", "false"),
        (
            "raw.capacity() == before.len() as usize",
            "raw.capacity() >= before.len() as usize",
        ),
        ("raw.clear();", ""),
        ("named.2.capacity() <= budget", "true"),
        ("named.2.capacity() == named.1.len as usize", "true"),
        ("scratch: None,", "scratch: Some(raw.clone()),"),
        ("self.scratch = None;", "self.raw.clear();"),
        ("entry.record.evict_scratch();", ""),
        ("entry.marker.evict_scratch();", ""),
        ("self.evict_scratch();", ""),
        ("self.latch_refusal();", "self.failed = true;"),
        ("libc::O_NOFOLLOW", "0"),
        ("self.directory.leaf.sync_all()?;", ""),
        ("file.sync_all()?;", ""),
        ("fail(fault, PublicationStage::DurableCommit)?;", ""),
        ("self.revalidate()?;", ""),
        (
            "for entry in &mut self.entries",
            "for entry in &self.entries",
        ),
    ];
    for (old, new) in cases {
        assert!(original.contains(old), "mutation selected: {old}");
        for (at, _) in original.match_indices(old) {
            let mut changed = original.clone();
            changed.replace_range(at..at + old.len(), new);
            assert!(
                !contract::accepts(&expected, &changed),
                "mutation refused: {old} at {at}"
            );
        }
    }
    for suffix in [
        "\nfn injected() {}",
        "\nimpl Leaf { fn injected(&self) {} }",
        "\nconst EXTRA: bool = true;",
    ] {
        assert!(!contract::accepts(
            &expected,
            &format!("{original}{suffix}")
        ));
    }
    assert!(!contract::accepts(
        &expected,
        &format!("{original}{original}")
    ));
}

#[test]
fn every_refusal_site_is_independently_guarded() {
    let original = candidate();
    let expected = contract::candidate(&frozen(contract::FIXTURE));
    let sites: Vec<_> = original
        .match_indices("self.latch_refusal();")
        .map(|(at, _)| at)
        .collect();
    assert_eq!(sites.len(), 4);
    for at in sites {
        let mut changed = original.clone();
        changed.replace_range(
            at..at + "self.latch_refusal();".len(),
            "self.failed = true;",
        );
        assert!(!contract::accepts(&expected, &changed));
    }
    assert_eq!(original.matches("self.failed = true;").count(), 1);
    assert!(original.contains("self.failed = true;\n        self.evict_scratch();"));
}

#[test]
fn observer_mutations_and_extra_effects_refuse() {
    let original = observer();
    let expected = contract::observer(&frozen(contract::OBSERVER));
    for (old, new) in [
        ("Site::LeafOpen", "Site::After"),
        ("Site::HeldMetadata", "Site::After"),
        ("leaf: &mut Leaf", "leaf: &Leaf"),
        ("&mut self.registry.entries", "&self.registry.entries"),
        ("&mut entry.record", "&entry.record"),
        ("&mut entry.marker", "&entry.marker"),
        ("leaf.evict_scratch();", ""),
        ("raw.clear();", ""),
        ("observer.around(Site::LeafBefore, || file.metadata())?", "file.metadata()?"),
        ("observer.around(Site::LeafBefore, || file.metadata())?", "observer.around(Site::LeafBefore, || observer.around(Site::LeafBefore, || file.metadata()))?"),
        (
            "open_at(dir, name, false)",
            "{ leaf_effect(); open_at(dir, name, false) }",
        ),
    ] {
        assert!(original.contains(old));
        let changed = original.replacen(old, new, 1);
        assert!(
            !contract::accepts(&expected, &changed),
            "observer mutation: {old}"
        );
    }
    for changed in [
        original.replacen("Site::LeafOpen", "Site::After", 1),
        original.replacen(
            "open_at(dir, name, false)",
            "{ injected(); open_at(dir, name, false) }",
            1,
        ),
        original.replacen("fn leaf_validate<", "fn absent<", 1),
        format!("{original}\nfn leaf_validate() {{}}"),
    ] {
        assert!(std::panic::catch_unwind(|| check_hooks(&changed, &candidate())).is_err());
    }
}

#[test]
fn storage_only_body_restoration_preserves_fresh_io_and_comparisons() {
    let baseline = implementation(&frozen(contract::FIXTURE), "Leaf");
    let current = implementation(&candidate(), "Leaf");
    let selected = canonical(tokens(
        "match slot.take() {
        Some(mut raw) if raw.capacity() == before.len() as usize => { raw.clear(); raw }
        _ => Vec::with_capacity(before.len() as usize),
    }",
    ));
    let read = canonical(body(current.clone(), "read_reusing"));
    assert_eq!(read.matches(&selected).count(), 1);
    assert_eq!(
        read.replace(
            &selected,
            &canonical(tokens("Vec::with_capacity(before.len() as usize)"))
        ),
        canonical(body(baseline.clone(), "read"))
    );
    let retain = canonical(tokens("if named.2.capacity() == named.1.len as usize && named.2.capacity() <= budget { self.scratch = Some(named.2); }"));
    let validate = canonical(body(current, "validate"));
    assert_eq!(validate.matches(&retain).count(), 1);
    let restored = validate.replace(&retain, "").replace(
        &canonical(tokens(
            "Self::read_reusing(dir, &self.name, uid, gid, budget, &mut self.scratch)",
        )),
        &canonical(tokens("Self::read(dir, &self.name, uid, gid, budget)")),
    );
    assert_eq!(
        canonical(tokens(&restored)),
        canonical(body(baseline, "revalidate"))
    );
}

#[test]
fn original_read_reopen_publication_and_validation_order_remain_intact() {
    let original = frozen(contract::FIXTURE);
    let changed = candidate();
    assert_eq!(
        canonical(body(implementation(&original, "Leaf"), "read")),
        canonical(body(implementation(&changed, "Leaf"), "read"))
    );
    assert_eq!(
        canonical(body(tokens(&original), "open_directory")),
        canonical(body(tokens(&changed), "open_directory"))
    );
    let old_publish = canonical(body(
        implementation(&original, "FixtureAuthorityRegistry"),
        "publish",
    ));
    let new_publish = canonical(body(
        implementation(&changed, "FixtureAuthorityRegistry"),
        "publish",
    ));
    assert_eq!(
        new_publish.replace(
            &canonical(tokens("self.latch_refusal();")),
            &canonical(tokens("self.failed = true;"))
        ),
        old_publish
    );
    let old_validate = canonical(body(
        implementation(&original, "FixtureAuthorityRegistry"),
        "revalidate",
    ));
    let new_validate = canonical(body(
        implementation(&changed, "FixtureAuthorityRegistry"),
        "revalidate",
    ));
    let restored = new_validate
        .replace(
            &canonical(tokens("&mut self.entries")),
            &canonical(tokens("&self.entries")),
        )
        .replace(
            &canonical(tokens("self.latch_refusal();")),
            &canonical(tokens("self.failed = true;")),
        );
    assert_eq!(restored, old_validate);
}

#[test]
fn patch_generator_refuses_changed_or_duplicated_anchor_shapes() {
    let original = frozen(contract::FIXTURE);
    for changed in [
        original.replace("self.failed = true;", "self.failed = false;"),
        original.replacen("    name: String,", "    renamed: String,", 1),
        format!("{original}{original}"),
    ] {
        assert!(std::panic::catch_unwind(|| contract::candidate(&changed)).is_err());
    }
}
