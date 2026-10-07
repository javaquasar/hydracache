use std::process::Command;

fn git(args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .output()
        .expect("Git required");
    assert!(output.status.success(), "Git identity unavailable");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn main() {
    println!(
        "cargo:rustc-env=SCREEN_SOURCE_SHA={}",
        git(&["rev-parse", "HEAD"])
    );
    for name in ["HEAD", "index"] {
        println!(
            "cargo:rerun-if-changed={}",
            git(&["rev-parse", "--git-path", name])
        );
    }
    let reference = git(&["symbolic-ref", "HEAD"]);
    println!(
        "cargo:rerun-if-changed={}",
        git(&["rev-parse", "--git-path", &reference])
    );
    println!("cargo:rerun-if-changed=build.rs");
}
