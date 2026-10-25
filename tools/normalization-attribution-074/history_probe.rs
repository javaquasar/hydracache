//! Opt-in prefix-chain diagnostic; no replacement of any normalizer.
pub const PATHS: [&str; 3] = [
    "tools/long-run-supervisor-074/src/diagnostic_authority_registry.rs",
    "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs",
    "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs",
];
pub const OPERATIONS: [&str; 4] = [
    "authority-context-prefix",
    "authority-hex-prefix",
    "remaining-preparation-prefix",
    "document-scratch-prefix",
];
pub const ITERATIONS: usize = 16;
pub fn order(block: usize) -> Vec<&'static str> {
    let mut operations = OPERATIONS.to_vec();
    if !block.is_multiple_of(2) {
        operations.reverse();
    }
    operations
}
pub fn normalize(operation: &str, path: &str, raw: &str) -> String {
    match operation {
        "authority-context-prefix" => crate::hc074_authority_context_view!(path, raw),
        "authority-hex-prefix" => crate::hc074_authority_hex_view!(path, raw),
        "remaining-preparation-prefix" => crate::hc074_remaining_preparation_view!(path, raw),
        "document-scratch-prefix" => crate::guard::normalize(path, raw),
        _ => panic!("unknown fixed prefix"),
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
        let [mut created, mut exited, mut kernel, mut user] = [FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        }; 4];
        // SAFETY: current process pseudo-handle and distinct valid output pointers.
        assert_ne!(
            unsafe {
                GetProcessTimes(
                    GetCurrentProcess(),
                    &mut created,
                    &mut exited,
                    &mut kernel,
                    &mut user,
                )
            },
            0
        );
        let ticks = |v: FILETIME| (u64::from(v.dwHighDateTime) << 32) | u64::from(v.dwLowDateTime);
        (ticks(kernel) + ticks(user)) * 100
    }
    fn digest(raw: &[u8]) -> String {
        Sha256::digest(raw)
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect()
    }
    let request: Value = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    assert_eq!(request["production_authority"], false);
    assert_eq!(request["paths"].as_array().unwrap().len(), PATHS.len());
    let mut cases = vec![];
    for (index, path) in PATHS.iter().enumerate() {
        assert_eq!(request["paths"][index]["path"], *path);
        let raw = std::fs::read(crate::runtime::root().join(path)).unwrap();
        assert_eq!(request["paths"][index]["sha256"], digest(&raw));
        // This is exactly the existing reader's input to the historical prefix.
        // No prefix warmup or self-derived expected output is prepared here.
        let start = Instant::now();
        let prepared = crate::runtime_io::read_to_string(crate::runtime::root().join(path))
            .unwrap()
            .replace('\r', "");
        println!(
            "\nHC074_HISTORY_PREP {}",
            json!({"path":path,
            "wall_ns":start.elapsed().as_nanos(),"input_bytes":prepared.len(),
            "input_sha256":digest(prepared.as_bytes()),"prefix_warmup":false})
        );
        cases.push((path, prepared));
    }
    let mut outputs = std::collections::BTreeMap::new();
    for block in 0..6 {
        for (path, raw) in &cases {
            for operation in order(block) {
                let before = cpu();
                let start = Instant::now();
                let mut output = String::new();
                for _ in 0..ITERATIONS {
                    output = normalize(operation, black_box(path), black_box(raw));
                    black_box(&output);
                }
                let wall = start.elapsed().as_nanos();
                let parent_cpu = cpu() - before;
                // Hash/consistency checking is outside the measured interval.
                let sha = digest(output.as_bytes());
                let key = (*path, operation);
                let expected = outputs.entry(key).or_insert_with(|| sha.clone());
                assert_eq!(&sha, expected);
                println!(
                    "HC074_HISTORY {}",
                    json!({"block":block,"path":path,
                    "operation":operation,"iterations":ITERATIONS,"wall_ns":wall,
                    "parent_cpu_ns":parent_cpu,"input_bytes":raw.len(),
                    "output_bytes":output.len(),"output_sha256":sha})
                );
            }
        }
    }
    println!(
        "HC074_HISTORY_DONE {}",
        json!({"samples":72,"blocks":6,
        "iterations":ITERATIONS,"prefix_warmup":false,"disjoint_intervals":false,
        "full_fixed_slice_chain":false,"product":false,"package_replay":false})
    );
}
#[cfg(not(windows))]
pub fn run(_: &std::path::Path) {
    panic!("fixed diagnostic requires Windows");
}
