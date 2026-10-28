//! Explicit local inventory only. Never executes or signs the prepared bytes.
#[cfg(target_os = "linux")]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    use hydracache_long_run_supervisor_074::{
        diagnostic_artifacts::ArtifactDigest, diagnostic_prebuilt_copy::prepare_asserted_copy,
    };
    use std::{fs::OpenOptions, os::unix::fs::OpenOptionsExt};
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 5 {
        return Err("usage: prepare_unsigned_prebuilt SOURCE PRIVATE_PARENT ATTEMPT ASSERTED_SHA256 ASSERTED_BYTES".into());
    }
    let attempt = args[2].to_str().ok_or("invalid attempt")?;
    let digest = ArtifactDigest {
        sha256: args[3].to_str().ok_or("invalid hash")?.to_owned(),
        bytes: args[4].to_str().ok_or("invalid size")?.parse()?,
    };
    // Leaf symlinks refuse. Ancestor/account/runtime trust is NOT established.
    let source = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(&args[0])?;
    let parent = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(&args[1])?;
    let mut copy = prepare_asserted_copy(&source, &parent, attempt, digest)?;
    let checked = copy.check_original()?;
    println!("unsigned_copy_sha256={} bytes={} copy_inventory_only=true authenticated_custody=false executed=false signed=false admission=false", checked.sha256, checked.bytes);
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        run()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err("unsigned copy preparation requires nonroot Linux".into())
    }
}
