// Copyright 2019 TiKV Project Authors. Licensed under Apache-2.0.

use protobuf_build::Builder;

fn main() {
    #[cfg(feature = "prost-codec")]
    {
        println!("cargo:rerun-if-env-changed=PROTOC");
        println!("cargo:rerun-if-changed=proto");
        if std::env::var_os("PROTOC").is_none() {
            let protoc = protoc_bin_vendored::protoc_bin_path()
                .expect("find the vendored protoc for this build host");
            std::env::set_var("PROTOC", protoc);
        }
    }
    let base = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let mut builder = Builder::new();
    builder
        .search_dir_for_protos(&format!("{}/proto", base))
        .includes(&[format!("{}/include", base), format!("{}/proto", base)])
        .include_google_protos();
    #[cfg(feature = "prost-codec")]
    builder.wrapper_options(protobuf_build::GenOpt::all() - protobuf_build::GenOpt::MESSAGE);
    builder.generate()
}
