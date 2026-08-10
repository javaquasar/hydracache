fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    let mut prost = tonic_prost_build::Config::new();
    prost.protoc_executable(protoc);
    prost.enum_attribute(
        ".hydracache.client.v2alpha.StableErrorCode",
        "#[allow(clippy::enum_variant_names)]",
    );
    let descriptor =
        std::path::PathBuf::from(std::env::var("OUT_DIR")?).join("hc2_contract_descriptor.bin");
    tonic_prost_build::configure()
        .file_descriptor_set_path(descriptor)
        .compile_with_config(
            prost,
            &[
                "proto/hc2_spike.proto",
                "../hydracache-client-hc2/proto/hc2_contract.proto",
            ],
            &["proto", "../hydracache-client-hc2/proto"],
        )?;
    Ok(())
}
