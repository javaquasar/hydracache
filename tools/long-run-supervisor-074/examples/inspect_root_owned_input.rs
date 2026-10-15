//! Read-only local inventory, no execution, signing or host selection.
#[cfg(target_os = "linux")]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    use hydracache_long_run_supervisor_074::{
        diagnostic_artifacts::ArtifactDigest, diagnostic_root_owned_input::inspect_root_owned_input,
    };
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: inspect_root_owned_input PATH ASSERTED_SHA256 ASSERTED_BYTES".into());
    }
    let expected = ArtifactDigest {
        sha256: args[1].to_str().ok_or("invalid hash")?.into(),
        bytes: args[2].to_str().ok_or("invalid size")?.parse()?,
    };
    let mut read = inspect_root_owned_input(std::path::Path::new(&args[0]), expected)?;
    let digest = read.check_original()?;
    println!("root_owned_input_sha256={} bytes={} original_reader_context=matched inventory_only=true authenticated_custody=false executed=false signed=false admission=false",digest.sha256,digest.bytes);
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        run()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err("root-owned input inventory requires Linux".into())
    }
}
