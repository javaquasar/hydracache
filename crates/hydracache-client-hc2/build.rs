fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/hc2_contract.proto");
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    let mut prost = tonic_prost_build::Config::new();
    prost.protoc_executable(protoc);
    prost.bytes(["."]);
    prost.enum_attribute(
        ".hydracache.client.v2alpha.StableErrorCode",
        "#[allow(clippy::enum_variant_names)]",
    );
    tonic_prost_build::configure().compile_with_config(
        prost,
        &["proto/hc2_contract.proto"],
        &["proto"],
    )?;
    Ok(())
}
