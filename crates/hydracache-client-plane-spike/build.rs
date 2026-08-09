fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    let mut prost = tonic_prost_build::Config::new();
    prost.protoc_executable(protoc);
    tonic_prost_build::configure().compile_with_config(
        prost,
        &["proto/hc2_spike.proto"],
        &["proto"],
    )?;
    Ok(())
}
