#[path = "support/document_scratch_guard.rs"]
mod guard;
#[path = "support/authority_history_io.rs"]
mod history;
use proc_macro2::{Delimiter, Group, Ident, Punct, Spacing, TokenStream, TokenTree};
fn read(path: &str) -> String {
    history::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
    )
    .unwrap()
    .replace('\r', "")
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
    let mut matches = Vec::new();
    for index in 0..trees.len().saturating_sub(1) {
        if trees[index].to_string() == "fn" && trees[index + 1].to_string() == name {
            let block = trees[index + 2..]
                .iter()
                .find_map(|tree| match tree {
                    TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                        Some(group.stream())
                    }
                    _ => None,
                })
                .unwrap();
            matches.push(block);
        }
    }
    assert_eq!(matches.len(), 1, "one function body {name}");
    matches.remove(0)
}
fn implementation(stream: TokenStream, name: &str) -> TokenStream {
    let trees: Vec<_> = stream.into_iter().collect();
    let mut matches = Vec::new();
    for index in 0..trees.len().saturating_sub(2) {
        if trees[index].to_string() == "impl" && trees[index + 1].to_string() == name {
            let TokenTree::Group(group) = &trees[index + 2] else {
                panic!("impl block")
            };
            assert_eq!(group.delimiter(), Delimiter::Brace);
            matches.push(group.stream());
        }
    }
    assert_eq!(matches.len(), 1, "one implementation {name}");
    matches.remove(0)
}
fn unwrap_observations(stream: TokenStream, mode: &str, sites: &mut Vec<String>) -> TokenStream {
    let trees: Vec<_> = stream.into_iter().collect();
    let mut output = TokenStream::new();
    let mut index = 0;
    while index < trees.len() {
        if index + 3 < trees.len()
            && trees[index].to_string() == "observer"
            && trees[index + 1].to_string() == "."
            && trees[index + 2].to_string() == "around"
        {
            let TokenTree::Group(arguments) = &trees[index + 3] else {
                panic!("observation arguments")
            };
            assert_eq!(arguments.delimiter(), Delimiter::Parenthesis);
            let arguments: Vec<_> = arguments.stream().into_iter().collect();
            assert!(arguments.len() > 7);
            for (position, expected) in [
                (0, "Site"),
                (1, ":"),
                (2, ":"),
                (4, ","),
                (5, "|"),
                (6, "|"),
            ] {
                assert_eq!(arguments[position].to_string(), expected);
            }
            sites.push(arguments[3].to_string());
            let mut expression: TokenStream = arguments[7..].iter().cloned().collect();
            let values: Vec<_> = expression.clone().into_iter().collect();
            if let [TokenTree::Group(group)] = values.as_slice() {
                if group.delimiter() == Delimiter::Brace {
                    let block: syn::Block = syn::parse2(expression.clone()).unwrap();
                    assert_eq!(
                        block.stmts.len(),
                        1,
                        "only a single tail expression wrapper"
                    );
                    assert!(matches!(block.stmts[0], syn::Stmt::Expr(_, None)));
                    expression = group.stream();
                }
            }
            output.extend(unwrap_observations(expression, mode, sites));
            index += 4;
            continue;
        }
        if trees[index].to_string() == "leaf_read" && index + 1 < trees.len() {
            let TokenTree::Group(arguments) = &trees[index + 1] else {
                panic!("leaf_read arguments")
            };
            assert_eq!(arguments.delimiter(), Delimiter::Parenthesis);
            let arguments: Vec<_> = arguments.stream().into_iter().collect();
            assert_eq!(arguments[0].to_string(), "observer");
            assert_eq!(arguments[1].to_string(), ",");
            let rest: TokenStream = arguments[2..].iter().cloned().collect();
            let rest = unwrap_observations(rest, mode, sites);
            output.extend(tokens(&format!("Self::read({rest})")));
            index += 2;
            continue;
        }
        match &trees[index] {
            TokenTree::Group(group) => output.extend([TokenTree::Group(Group::new(
                group.delimiter(),
                unwrap_observations(group.stream(), mode, sites),
            ))]),
            TokenTree::Ident(ident)
                if (mode == "inventory" && ident == "directory")
                    || (mode == "leaf_revalidate" && ident == "leaf") =>
            {
                output.extend([TokenTree::Ident(Ident::new("self", ident.span()))])
            }
            TokenTree::Ident(ident) if mode == "inventory" && ident == "Directory" => {
                output.extend([TokenTree::Ident(Ident::new("Self", ident.span()))])
            }
            other => output.extend([other.clone()]),
        }
        index += 1;
    }
    output
}
fn restored_control(source: &str, name: &str) -> String {
    let mut sites = Vec::new();
    let mut output = unwrap_observations(body(tokens(source), name), name, &mut sites).to_string();
    let expected: &[&str] = match name {
        "inventory" => &[
            "Path",
            "DirOpen",
            "DirNext",
            "DirName",
            "DirInsert",
            "DirExpected",
        ],
        "leaf_read" => &[
            "LeafOpen",
            "LeafBefore",
            "ParentMetadata",
            "Buffer",
            "Read",
            "After",
        ],
        "leaf_revalidate" => &["HeldMetadata"],
        _ => panic!("unknown control"),
    };
    assert_eq!(sites, expected);
    if name == "inventory" {
        let expanded =
            tokens("let mut entries = fs::read_dir(path)?; while let Some(entry) = entries.next()")
                .to_string();
        let original = tokens("for entry in fs::read_dir(path)?").to_string();
        assert_eq!(
            output.matches(&expanded).count(),
            1,
            "one equivalent iterator expansion"
        );
        output = output.replacen(&expanded, &original, 1);
        let retirement = tokens("drop(entries);").to_string();
        assert_eq!(
            output.matches(&retirement).count(),
            1,
            "one original iterator retirement boundary"
        );
        output = output.replacen(&retirement, "", 1);
    }
    canonical(tokens(&output))
}
#[test]
fn authority_owners_only_adds_exact_hooks_and_preserves_every_runtime_byte() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs",
        "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs",
        "docs/testing/gated-test-registry.toml",
    ] {
        let raw = read(path);
        let output = history::command()
            .args([
                "show",
                &format!("578fd6409eaf393f3d658bf209b439fcc42e6110:{path}"),
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let baseline = String::from_utf8(output.stdout).unwrap().replace('\r', "");
        let restored = crate::hc074_authority_owners_view!(path, &raw);
        assert_eq!(restored, baseline);
        assert_eq!(
            crate::hc074_authority_owners_view!(path, &restored),
            restored
        );
        let marker = if path.ends_with("fixture.rs") {
            "pub(crate) mod owners_controls;"
        } else if path.ends_with("alloc_tests.rs") {
            "mod authority_owners_alloc;"
        } else {
            "test = \"authority_owners_allocation_attribution\""
        };
        for changed in [raw.replacen(marker, "changed", 1), format!("{raw}{raw}")] {
            assert!(
                std::panic::catch_unwind(|| crate::hc074_authority_owners_view!(path, &changed))
                    .is_err()
            );
        }
        let unrelated = format!("{raw}\n// unrelated drift\n");
        assert!(
            crate::hc074_authority_owners_view!(path, &unrelated).contains("// unrelated drift")
        );
    }
    assert_eq!(guard::normalize("unrelated", "unchanged"), "unchanged");
}
#[test]
fn authority_owners_observation_wrappers_restore_exact_runtime_tokens_and_refuse_drift() {
    let runtime = tokens(&read(
        "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs",
    ));
    let controls = read("tools/long-run-supervisor-074/tests/support/authority_owners_controls.rs");
    for (helper, owner, method) in [
        ("inventory", "Directory", "inventory"),
        ("leaf_read", "Leaf", "read"),
        ("leaf_revalidate", "Leaf", "revalidate"),
    ] {
        let expected = canonical(body(implementation(runtime.clone(), owner), method));
        assert_eq!(restored_control(&controls, helper), expected);
    }
    let expected = restored_control(&controls, "inventory");
    for changed in [
        controls.replace(
            "seen.len() > MAX_GENERATIONS",
            "seen.len() >= MAX_GENERATIONS",
        ),
        controls.replace("fd path is ASCII", "fd  path is ASCII"),
    ] {
        assert_ne!(restored_control(&changed, "inventory"), expected);
    }
    for changed in [
        controls.replacen("Site::Path", "Site::DirOpen", 1),
        controls.replacen("drop(entries);", "", 1),
        controls.replacen("drop(entries);", "drop(entries); drop(entries);", 1),
        format!("{controls}\nfn inventory() {{}}"),
        controls.replacen("fn inventory<", "fn missing_inventory<", 1),
    ] {
        assert!(std::panic::catch_unwind(|| restored_control(&changed, "inventory")).is_err());
    }
    for expression in ["result()?.field", "result()? .field"] {
        let _: syn::Expr = syn::parse_str(expression).unwrap();
    }
    assert_eq!(
        canonical(tokens("result()?.field")),
        canonical(tokens("result()? .field"))
    );
    assert_ne!(canonical(tokens("a && b")), canonical(tokens("a & &b")));
    assert_ne!(canonical(tokens("\"?.\"")), canonical(tokens("\"? .\"")));
}
#[test]
fn authority_owners_contract_keeps_request_scope_and_frozen_io_reader() {
    let contract: toml::Value = toml::from_str(&read(
        "docs/testing/performance/0.74/authority-owners-contract.toml",
    ))
    .unwrap();
    for flag in [
        "runtime_changed",
        "candidate_enabled",
        "allocation_reduction_claimed",
        "server_admitted",
        "latency_gain_claimed",
        "fresh_checks_changed",
        "deadlines_changed",
    ] {
        assert_eq!(contract[flag].as_bool(), Some(false));
    }
    let frozen = read("tools/long-run-supervisor-074/tests/support/validation_leaf_requests.rs");
    let probe = read("tools/long-run-supervisor-074/tests/support/authority_owners_alloc.rs");
    let original = frozen
        .split("struct Io {")
        .nth(1)
        .unwrap()
        .split("struct Journal")
        .next()
        .unwrap();
    let copied = probe
        .split("struct Io {")
        .nth(1)
        .unwrap()
        .split("#[test]")
        .next()
        .unwrap();
    assert_eq!(
        tokens(&format!("struct Io {{{original}")).to_string(),
        tokens(&format!("struct Io {{{copied}")).to_string()
    );
    assert!(probe.contains("no nested whole-operation capture"));
    assert!(
        probe.contains("assert_eq!(counts[actual], counts[traced])")
            || probe.contains("assert_eq!(counts[actual],counts[traced])")
    );
}
