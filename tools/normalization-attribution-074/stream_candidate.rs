//! Isolated canonical-output candidate; never used by the installed source gate.
use proc_macro2::TokenStream;
use std::fmt::Write;

struct Matcher<'a> {
    expected: &'a [u8],
    offset: usize,
    equal: bool,
}
impl std::fmt::Write for Matcher<'_> {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        if let Some(end) = self.offset.checked_add(value.len()) {
            self.equal &= self.expected.get(self.offset..end) == Some(value.as_bytes());
            self.offset = end;
        } else {
            self.equal = false;
            self.offset = usize::MAX;
        }
        // Continue rendering after mismatch: no partial-output success or early
        // termination of the actual TokenStream display traversal.
        Ok(())
    }
}
fn matches(tokens: &TokenStream, expected: &str) -> bool {
    let mut matcher = Matcher {
        expected: expected.as_bytes(),
        offset: 0,
        equal: true,
    };
    write!(&mut matcher, "{tokens}").is_ok() && matcher.equal && matcher.offset == expected.len()
}
pub fn accepts(expected: &str, actual: &str) -> bool {
    if syn::parse_file(actual).is_err() {
        return false;
    }
    // Preserve the old unwrap point before the actual lexical parse, including
    // panic payloads for invalid expected input after valid actual syntax.
    let expected = expected.parse::<TokenStream>().unwrap().to_string();
    actual
        .parse::<TokenStream>()
        .map(|tokens| matches(&tokens, &expected))
        .unwrap_or_else(|_| expected.is_empty())
}

#[test]
fn streaming_matcher_checks_chunks_final_length_unicode_and_sticky_mismatch() {
    for (expected, chunks, answer) in [
        ("ab", vec!["a", "b"], true),
        ("ab", vec!["a"], false),
        ("a", vec!["a", "b"], false),
        ("é界", vec!["é", "界"], true),
        ("e", vec!["é"], false),
        ("", vec![""], true),
        ("same", vec!["bad", "same"], false),
    ] {
        let mut matcher = Matcher {
            expected: expected.as_bytes(),
            offset: 0,
            equal: true,
        };
        let length: usize = chunks.iter().map(|v| v.len()).sum();
        for chunk in chunks {
            matcher.write_str(chunk).unwrap();
        }
        assert_eq!(matcher.offset, length);
        assert_eq!(matcher.equal && matcher.offset == expected.len(), answer);
    }
    let mut overflow = Matcher {
        expected: b"",
        offset: usize::MAX,
        equal: true,
    };
    overflow.write_str("a").unwrap();
    assert!(!overflow.equal);
    assert_eq!(overflow.offset, usize::MAX);
}
#[test]
fn streaming_candidate_retains_syntax_expected_panic_and_default_empty_order() {
    fn outcome(f: impl FnOnce() -> bool + std::panic::UnwindSafe) -> Result<bool, (bool, String)> {
        std::panic::catch_unwind(f).map_err(|payload| {
            let borrowed = payload.downcast_ref::<&str>();
            (
                borrowed.is_some(),
                borrowed
                    .map(|v| (*v).to_owned())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .expect("old string panic API"),
            )
        })
    }
    for (expected, actual) in [
        ("(", "fn broken("),
        ("(", ""),
        ("", "#!\0\n"),
        ("", "\u{feff}"),
        ("fn f() {}", "// comment\nfn f(){}"),
        ("fn f() {}", "fn f(){"),
        ("", ""),
        ("a", "fn f() {}"),
    ] {
        assert_eq!(
            outcome(|| crate::storage_contract::accepts(expected, actual)),
            outcome(|| accepts(expected, actual)),
            "{expected:?}/{actual:?}"
        );
    }
}
#[test]
fn streaming_candidate_matches_seeded_tokens_comments_and_literal_mutations() {
    let mut seed = 0x074c_a110_u64;
    for case in 0..1024 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let expected =
            format!("#[doc=\"é界{case}\"] fn f_{case}(){{let n={seed}u64; assert!(n>0);}}");
        for actual in [
            expected.clone(),
            format!("// {case}\n{expected}"),
            expected.replace("n>0", "n>=0"),
            expected.replacen("u64", "u32", 1),
            format!("{expected} const EXTRA: bool = true;"),
            format!("{expected}{{"),
        ] {
            assert_eq!(
                crate::storage_contract::accepts(&expected, &actual),
                accepts(&expected, &actual)
            );
        }
    }
}
#[test]
fn streaming_candidate_keeps_real_selected_source_and_fault_results() {
    for path in super::normalization_profile::PATHS {
        let actual = std::fs::read_to_string(crate::runtime::root().join(path))
            .unwrap()
            .replace("\r\n", "\n");
        let expected = crate::runtime::expected(path).unwrap();
        for changed in [
            actual.clone(),
            format!("{actual}\n// comment\n"),
            actual.replace("assert_eq!", "assert_ne!"),
            format!("{actual}\nconst EXTRA: bool=true;"),
            format!("{actual}{{"),
        ] {
            assert_eq!(
                crate::storage_contract::accepts(&expected, &changed),
                accepts(&expected, &changed)
            );
        }
    }
}

#[cfg(windows)]
pub fn run(input: &std::path::Path) {
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};
    use std::hint::black_box;
    use std::time::Instant;
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
    fn cpu() -> u64 {
        let [mut a, mut b, mut k, mut u] = [FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        }; 4];
        // SAFETY: current pseudo-handle and distinct valid output pointers.
        assert_ne!(
            unsafe { GetProcessTimes(GetCurrentProcess(), &mut a, &mut b, &mut k, &mut u) },
            0
        );
        let ticks = |v: FILETIME| (u64::from(v.dwHighDateTime) << 32) | u64::from(v.dwLowDateTime);
        (ticks(k) + ticks(u)) * 100
    }
    let request: Value = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    assert_eq!(request["production_authority"], false);
    assert_eq!(request["paths"].as_array().unwrap().len(), 3);
    let mut cases = vec![];
    for (i, path) in super::normalization_profile::PATHS.iter().enumerate() {
        assert_eq!(request["paths"][i]["path"], *path);
        let raw = std::fs::read(crate::runtime::root().join(path)).unwrap();
        let digest: String = Sha256::digest(&raw)
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect();
        assert_eq!(request["paths"][i]["sha256"], digest);
        cases.push((
            path,
            String::from_utf8(raw).unwrap().replace("\r\n", "\n"),
            crate::runtime::expected(path).unwrap(),
        ));
    }
    for block in 0_usize..6 {
        let order = if block.is_multiple_of(2) {
            [false, true, true, false]
        } else {
            [true, false, false, true]
        };
        for (path, actual, expected) in &cases {
            for (slot, selected) in order.into_iter().enumerate() {
                let before = cpu();
                let start = Instant::now();
                for _ in 0..32 {
                    let valid = if selected {
                        accepts(black_box(expected), black_box(actual))
                    } else {
                        crate::storage_contract::accepts(black_box(expected), black_box(actual))
                    };
                    assert!(black_box(valid));
                }
                let wall = start.elapsed().as_nanos();
                let parent_cpu = cpu() - before;
                println!(
                    "\nHC074_CANONICAL {}",
                    json!({"block":block,"slot":slot,"path":path,"candidate":selected,
                    "iterations":32,"wall_ns":wall,"parent_cpu_ns":parent_cpu,"accepted":32})
                );
            }
        }
    }
    println!(
        "HC074_CANONICAL_DONE {}",
        json!({"blocks":6,"samples":72,"warmup":false,
        "allocator_interposition":false,"installed":false,"product":false,"package_replay":false})
    );
}
#[cfg(not(windows))]
pub fn run(_: &std::path::Path) {
    panic!("fixed diagnostic requires Windows");
}
