//! Opt-in source-reader attribution; ordinary package execution only checks the protocol.
#[path = "support/census_provider_input_legacy.rs"]
mod caller;
#[path = "../../../tools/historical-blob-batch-074/mod.rs"]
pub mod census_runtime;
#[path = "support/census_provider_input.rs"]
mod integrated;

const OPERATIONS: [&str; 5] = ["original", "accepts", "view", "mismatch", "unrelated"];
const BLOCKS: usize = 6;

fn order(block: usize) -> [&'static str; 4] {
    if block.is_multiple_of(2) {
        ["legacy", "current", "current", "legacy"]
    } else {
        ["current", "legacy", "legacy", "current"]
    }
}

#[test]
fn attribution_order_is_balanced_and_not_a_package_measurement() {
    for block in 0..BLOCKS {
        let row = order(block);
        assert_eq!(row.iter().filter(|v| **v == "legacy").count(), 2);
        assert_eq!(row.iter().filter(|v| **v == "current").count(), 2);
        assert_ne!(row[0], row[1]);
        assert_eq!(row[0], row[3]);
    }
    assert_eq!(OPERATIONS.len(), 5);
    // Keep the old public surface compiled even outside the opt-in Windows runner.
    let _: fn() -> String = caller::original;
    let _: fn(&str, &str) -> bool = caller::accepts;
    let _: fn(&str, &str) -> String = caller::view;
    let _: fn() -> String = integrated::original;
    let _: fn(&str, &str) -> bool = integrated::accepts;
    let _: fn(&str, &str) -> String = integrated::view;
}

#[test]
fn census_cost_child() {
    let Some(input) = std::env::var_os("HC074_CENSUS_COST_INPUT") else {
        return;
    };
    #[cfg(windows)]
    windows::run(std::path::Path::new(&input));
    #[cfg(not(windows))]
    panic!("Windows attribution only: {input:?}");
}

#[cfg(windows)]
mod windows {
    use super::{caller, census_runtime, integrated, order, BLOCKS, OPERATIONS};
    use census_runtime::{census_reference as reference, transport};
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};
    use std::hint::black_box;
    use std::path::Path;
    use std::process::Command;
    use std::time::Instant;
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

    fn cpu() -> u64 {
        let [mut created, mut exited, mut kernel, mut user] = [FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        }; 4];
        // SAFETY: the pseudo-handle is valid for this process and every out-pointer
        // addresses a distinct, initialized FILETIME for the duration of the call.
        let result = unsafe {
            GetProcessTimes(
                GetCurrentProcess(),
                &mut created,
                &mut exited,
                &mut kernel,
                &mut user,
            )
        };
        assert_ne!(result, 0, "GetProcessTimes refused");
        let ticks = |v: &FILETIME| (u64::from(v.dwHighDateTime) << 32) | u64::from(v.dwLowDateTime);
        (ticks(&kernel) + ticks(&user)) * 100
    }

    fn measure<T>(call: impl FnOnce() -> T) -> (T, u128, u64) {
        let before = cpu();
        let start = Instant::now();
        let value = black_box(call());
        let wall = start.elapsed().as_nanos();
        (value, wall, cpu() - before)
    }

    fn raw(case: &Value) -> Vec<u8> {
        let value = case["stdout_hex"].as_str().unwrap();
        assert_eq!(value.len() % 2, 0);
        (0..value.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&value[i..i + 2], 16).unwrap())
            .collect()
    }

    fn invoke(variant: &str, operation: &str, base: &str, reviewed: &str) -> String {
        match (variant, operation) {
            ("legacy", "original") => caller::original(),
            ("current", "original") => integrated::original(),
            ("legacy", "accepts") => caller::accepts(base, reviewed).to_string(),
            ("current", "accepts") => integrated::accepts(base, reviewed).to_string(),
            ("legacy", "view") => caller::view("diagnostic_artifacts_linux.rs", reviewed),
            ("current", "view") => integrated::view("diagnostic_artifacts_linux.rs", reviewed),
            ("legacy", "mismatch") => caller::accepts(base, "mismatch").to_string(),
            ("current", "mismatch") => integrated::accepts(base, "mismatch").to_string(),
            ("legacy", "unrelated") => caller::view("other.rs", "untouched"),
            ("current", "unrelated") => integrated::view("other.rs", "untouched"),
            _ => panic!("unknown fixed scenario"),
        }
    }

    pub fn run(input: &Path) {
        let document: Value = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
        assert_eq!(document["test_fixture_pin_integrity"], true);
        assert_eq!(document["production_pin_authority"], false);
        let cases = document["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 2);
        let mut bodies = vec![];
        for (index, revision) in [reference::BASE, reference::REVIEWED].iter().enumerate() {
            let bytes = raw(&cases[index]);
            assert_eq!(
                cases[index]["spec"],
                format!("{revision}:{}", reference::SOURCE)
            );
            assert_eq!(bytes.len(), reference::ROWS[index].1);
            let digest: String = Sha256::digest(&bytes)
                .iter()
                .map(|v| format!("{v:02x}"))
                .collect();
            assert_eq!(digest, reference::ROWS[index].2);
            let body = String::from_utf8(bytes).unwrap();
            assert!(!body.contains("\r\n"));
            bodies.push(body);
        }
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let helper = std::fs::read_to_string(root.join(integrated::contract::CONTROL)).unwrap();
        assert!(integrated::contract::helper_matches(&helper));
        assert!(integrated::contract::accepts(&bodies[0], &bodies[1]));
        let mut samples = vec![];
        for block in 0..BLOCKS {
            for operation in OPERATIONS {
                for (slot, variant) in order(block).iter().enumerate() {
                    let (value, wall, parent_cpu) =
                        measure(|| invoke(variant, operation, &bodies[0], &bodies[1]));
                    let expected = match operation {
                        "original" | "view" => &bodies[0],
                        "accepts" => "true",
                        "mismatch" => "false",
                        "unrelated" => "untouched",
                        _ => unreachable!(),
                    };
                    assert_eq!(value, expected);
                    println!(
                        "HC074_COST {}",
                        json!({"phase":"e2e","block":block,"slot":slot,
                        "operation":operation,"variant":variant,"iterations":1,"wall_ns":wall,"parent_cpu_ns":parent_cpu,
                        "result_bytes":value.len()})
                    );
                    samples.push(wall);
                }
            }
        }
        assert_eq!(samples.len(), BLOCKS * OPERATIONS.len() * 4);
        // Separate component diagnostics, NOT subtracted phases of a full call.
        for block in 0..BLOCKS {
            for component in [
                "observer",
                "reference",
                "sha256",
                "interpret",
                "helper-read-hash",
                "body-predicate",
                "pipe",
                "owned",
            ] {
                let iterations = match component {
                    "pipe" | "owned" => 1,
                    "body-predicate" => 4,
                    _ => 128,
                };
                let (_, wall, parent_cpu) = measure(|| {
                    for _ in 0..iterations {
                        match component {
                            "observer" => {
                                black_box(&bodies[0]);
                            }
                            "reference" => {
                                black_box(
                                    reference::reference(reference::BASE, reference::SOURCE)
                                        .unwrap(),
                                );
                            }
                            "sha256" => {
                                black_box(Sha256::digest(black_box(bodies[0].as_bytes())));
                            }
                            "interpret" => {
                                let output = transport::Output {
                                    code: Some(0),
                                    stdout: bodies[0].as_bytes().to_vec(),
                                    stderr: vec![],
                                };
                                black_box(
                                    census_runtime::census_caller::interpret(output).unwrap(),
                                );
                            }
                            "helper-read-hash" => {
                                let current = std::fs::read_to_string(
                                    root.join(integrated::contract::CONTROL),
                                )
                                .unwrap();
                                assert!(integrated::contract::helper_matches(&current));
                            }
                            "body-predicate" => {
                                assert!(integrated::contract::accepts(&bodies[0], &bodies[1]));
                            }
                            "pipe" | "owned" => {
                                let mut command = Command::new("git");
                                command
                                    .arg("--no-replace-objects")
                                    .args([
                                        "show",
                                        &format!("{}:{}", reference::BASE, reference::SOURCE),
                                    ])
                                    .current_dir(&root);
                                let value = if component == "pipe" {
                                    let value = command.output().unwrap();
                                    transport::Output {
                                        code: value.status.code(),
                                        stdout: value.stdout,
                                        stderr: value.stderr,
                                    }
                                } else {
                                    transport::run_owned(
                                        &mut command,
                                        &[],
                                        Instant::now() + transport::BOUND,
                                    )
                                    .unwrap()
                                };
                                assert_eq!(value.code, Some(0));
                                assert_eq!(value.stdout, bodies[0].as_bytes());
                            }
                            _ => unreachable!(),
                        }
                    }
                });
                println!(
                    "HC074_COST {}",
                    json!({"phase":"component","block":block,"component":component,
                    "iterations":iterations,"wall_ns":wall,"parent_cpu_ns":parent_cpu})
                );
            }
        }
        println!(
            "HC074_COST_DONE {}",
            json!({"e2e_samples":samples.len(),"component_samples":BLOCKS*8,
            "git_reads_expected_e2e":BLOCKS*4*5,"git_reads_expected_components":BLOCKS*2,
            "first_samples_retained":true,"warmup":false,"package_replay":false,"production_authority":false})
        );
    }
}
