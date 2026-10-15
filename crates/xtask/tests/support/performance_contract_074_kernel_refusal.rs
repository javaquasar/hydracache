//! Restrict attribution instrumentation to its exact test-only expansion.
use proc_macro2::{Delimiter, Group, Punct, Spacing, TokenStream, TokenTree};

const BASELINE: &str = "914aa749369a78a57ae8aebb8e95c8a878366fbf";
const HEADER: &str = r#"
#[cfg(test)]
#[path = "diagnostic_kernel_refusal_trace.rs"]
pub(crate) mod refusal_trace;
macro_rules! kernel_invalid {
    ($site:literal) => { kernel_invalid!($site, [0; 6]) };
    ($site:literal, $facts:expr) => {{
        #[cfg(test)]
        $crate::diagnostic_kernel_admission::refusal_trace::record($site, file!(), line!(), $facts);
        KernelError::Invalid
    }};
}
macro_rules! kernel_required {
    ($option:expr, $site:literal) => {{
        #[cfg(test)]
        { ($option).ok_or_else(|| kernel_invalid!($site)) }
        #[cfg(not(test))]
        { ($option).ok_or(KernelError::Invalid) }
    }};
}
"#;
#[derive(Default)]
struct Counts {
    sites: usize,
    lazy: usize,
}
fn rewrite(stream: TokenStream, counts: &mut Counts) -> TokenStream {
    let input: Vec<_> = stream.into_iter().collect();
    let mut out = TokenStream::new();
    let mut i = 0;
    while i < input.len() {
        if matches!(&input[i], TokenTree::Ident(n) if n == "kernel_invalid") {
            assert!(matches!(input.get(i + 1), Some(TokenTree::Punct(p)) if p.as_char() == '!'));
            assert!(
                matches!(input.get(i + 2), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis)
            );
            out.extend("KernelError::Invalid".parse::<TokenStream>().unwrap());
            counts.sites += 1;
            i += 3;
            continue;
        }
        if matches!(&input[i], TokenTree::Ident(n) if n == "kernel_required") {
            assert!(matches!(input.get(i + 1), Some(TokenTree::Punct(p)) if p.as_char() == '!'));
            let Some(TokenTree::Group(group)) = input.get(i + 2) else {
                panic!("missing Option macro arguments");
            };
            assert_eq!(group.delimiter(), Delimiter::Parenthesis);
            let args: Vec<_> = group.stream().into_iter().collect();
            let comma = args
                .iter()
                .position(|t| matches!(t, TokenTree::Punct(p) if p.as_char() == ','))
                .unwrap();
            assert!(matches!(args.get(comma + 1), Some(TokenTree::Literal(_))));
            assert!(
                args.len() == comma + 2
                    || args.len() == comma + 3
                        && matches!(&args[comma + 2], TokenTree::Punct(p) if p.as_char() == ',')
            );
            out.extend(rewrite(args[..comma].iter().cloned().collect(), counts));
            out.extend(
                ".ok_or(KernelError::Invalid)"
                    .parse::<TokenStream>()
                    .unwrap(),
            );
            counts.sites += 1;
            counts.lazy += 1;
            i += 3;
            continue;
        }
        out.extend([match &input[i] {
            TokenTree::Group(g) => {
                TokenTree::Group(Group::new(g.delimiter(), rewrite(g.stream(), counts)))
            }
            // Rustfmt may print `x()?.field` across lines; '?' has no compound
            // Rust operator. Normalize only its non-semantic joint spacing.
            TokenTree::Punct(p) if p.as_char() == '?' => {
                TokenTree::Punct(Punct::new('?', Spacing::Alone))
            }
            token => token.clone(),
        }]);
        i += 1;
    }
    out
}
fn non_test_source(source: &str) -> &str {
    let position = source
        .rfind("\n#[cfg(test)]\n")
        .expect("fixed trailing unit-test module");
    let trailing =
        syn::parse_file(&source[position..]).expect("complete trailing cfg(test) module");
    assert!(
        matches!(trailing.items.as_slice(), [syn::Item::Mod(module)] if module.ident == "tests"),
        "no ordinary items may hide after the cfg(test) module"
    );
    &source[..position]
}
fn stripped_header(source: &str) -> String {
    let start = source
        .find("// BEGIN TEST ONLY KERNEL REFUSAL TRACE")
        .unwrap();
    let marker = "// END TEST ONLY KERNEL REFUSAL TRACE";
    let end = source.find(marker).unwrap();
    let header = &source[start + "// BEGIN TEST ONLY KERNEL REFUSAL TRACE".len()..end];
    assert_eq!(
        header.parse::<TokenStream>().unwrap().to_string(),
        HEADER.parse::<TokenStream>().unwrap().to_string()
    );
    let mut text = source.to_owned();
    text.replace_range(start..end + marker.len(), "");
    text
}

#[test]
fn kernel_refusal_test_instrumentation_preserves_every_non_test_guard() {
    let c = super::contract("diagnostic-kernel-refusal-attribution-contract.toml");
    assert_eq!(c["instrumented_invalid_sites"].as_integer(), Some(32));
    assert_eq!(c["test_only_lazy_option_sites"].as_integer(), Some(3));
    assert_eq!(c["packet_hold_seconds"].as_integer(), Some(60));
    for field in [
        "capture_module_cfg_test_only",
        "scoped_thread_local_first_event_only",
        "nested_unwind_and_thread_isolation_required",
        "only_static_sites_phases_and_numeric_clock_facts",
        "successful_checks_have_no_error_event",
        "non_test_guard_tokens_equivalent_to_baseline",
        "changed_guard_and_deadline_canaries_required",
    ] {
        assert_eq!(c[field].as_bool(), Some(true), "{field}");
    }
    for field in [
        "historical_invalid_root_cause_proved",
        "binary_identity_or_performance_proved",
        "automatic_retry_or_time_fallback",
        "raw_packet_token_credentials_or_key_retained",
        "runtime_journal_or_admission_api_added",
        "server_workflow_product_or_qualification_allowed",
        "production_start_or_all_writer_cessation",
    ] {
        assert_eq!(c[field].as_bool(), Some(false), "{field}");
    }
    let mut total = Counts::default();
    for name in [
        "diagnostic_kernel_admission.rs",
        "diagnostic_kernel_origin.rs",
    ] {
        let path = format!("tools/long-run-supervisor-074/src/{name}");
        let baseline = std::process::Command::new("git")
            .args(["show", &format!("{BASELINE}:{path}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(baseline.status.success());
        let baseline = String::from_utf8(baseline.stdout).unwrap();
        let current = std::fs::read_to_string(super::root().join(path)).unwrap();
        let current = if name == "diagnostic_kernel_admission.rs" {
            stripped_header(&current)
        } else {
            current
        };
        let expected = rewrite(
            non_test_source(&baseline).parse().unwrap(),
            &mut Counts::default(),
        )
        .to_string();
        let normalized =
            rewrite(non_test_source(&current).parse().unwrap(), &mut total).to_string();
        assert_eq!(
            normalized, expected,
            "unexpected guard, ordering, budget or API change in {name}"
        );
        let mutation = if name == "diagnostic_kernel_admission.rs" {
            current.replacen("60_000_000_000", "61_000_000_000", 1)
        } else {
            current.replacen("stat.f_type != magic", "stat.f_type == magic", 1)
        };
        assert_ne!(
            rewrite(
                non_test_source(&mutation).parse().unwrap(),
                &mut Counts::default()
            )
            .to_string(),
            expected,
            "normalization must not erase a changed deadline or guard"
        );
        assert!(
            std::panic::catch_unwind(|| {
                let _ = non_test_source(&format!("{current}\nfn unguarded_tail() {{}}\n"));
            })
            .is_err(),
            "non-test tail must not be ignored"
        );
    }
    assert_eq!((total.sites, total.lazy), (32, 3));
    let trace = std::fs::read_to_string(
        super::root().join("tools/long-run-supervisor-074/src/diagnostic_kernel_refusal_trace.rs"),
    )
    .unwrap();
    for forbidden in [
        "KernelPacket",
        "DiagnosticRequest",
        "SigningKey",
        "fs::read",
        "clock_gettime",
        "pub fn",
    ] {
        assert!(!trace.contains(forbidden), "{forbidden}");
    }
    println!("kernel_refusal_equivalence baseline={BASELINE} sites={} lazy={} guards_unchanged=true historical_cause_proved=false", total.sites, total.lazy);
}
