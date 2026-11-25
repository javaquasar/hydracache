use super::*;

#[test]
fn package_metadata_requires_one_exact_sdk_identity() {
    assert_eq!(
        package_id_from_metadata(br#"{"packages":[{"name":"unrelated","id":"ignore"},{"name":"hydracache-client-hc2","id":"exact-sdk"}]}"#).unwrap(),
        "exact-sdk"
    );
    for bytes in [
        &br#"{"packages":[]}"#[..],
        &br#"{"packages":[{"name":"other","id":"wrong"}]}"#[..],
        &br#"{"packages":[{"name":"hydracache-client-hc2","id":"a"},{"name":"hydracache-client-hc2","id":"b"}]}"#[..],
        &br#"{"packages":[{"name":"hydracache-client-hc2","id":7}]}"#[..],
        &b"not-json"[..],
    ] {
        assert!(package_id_from_metadata(bytes).is_err());
    }
}

#[test]
fn rust_output_must_be_a_canonical_descendant_of_the_requested_target() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target");
    let child = target.join("build/out");
    let outside = root.path().join("outside");
    fs::create_dir_all(&child).unwrap();
    fs::create_dir(&outside).unwrap();
    assert_eq!(
        checked_rust_output(&child, &target).unwrap(),
        child.canonicalize().unwrap()
    );
    for path in [&target, &outside] {
        assert!(checked_rust_output(path, &target)
            .unwrap_err()
            .to_string()
            .contains("outside its target"));
    }
    assert!(checked_rust_output(&root.path().join("absent"), &target).is_err());
    assert!(checked_rust_output(&child, &root.path().join("absent")).is_err());
}

fn fixture(root: &Path, case: &str) {
    let sdk = root.join("crates/hydracache-client-hc2");
    fs::create_dir_all(sdk.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/hydracache-client-hc2\"]\nresolver = \"2\"\n",
    )
    .unwrap();
    fs::write(
        root.join("Cargo.lock"),
        "version = 3\n\n[[package]]\nname = \"hydracache-client-hc2\"\nversion = \"0.0.0\"\n",
    )
    .unwrap();
    fs::write(
        sdk.join("Cargo.toml"),
        "[package]\nname = \"hydracache-client-hc2\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(
        sdk.join("src/lib.rs"),
        "include!(concat!(env!(\"OUT_DIR\"), \"/wire.rs\"));\n",
    )
    .unwrap();
    fs::write(
        sdk.join("build.rs"),
        r#"
fn main() {
    let case = std::env::var("HYDRACACHE_GENERATION_CASE").unwrap();
    assert_ne!(case, "rust-fail", "intentional generator refusal");
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let value = if case == "rust-diff" && out.to_string_lossy().contains("rust-b") { 2 } else { 1 };
    std::fs::write(out.join("wire.rs"), format!("pub const GENERATED: u8 = {value};\n")).unwrap();
}
"#,
    )
    .unwrap();
    let java = root.join("sdks/java/hydracache-client-hc2");
    fs::create_dir_all(java.join("fixture-source")).unwrap();
    fs::write(
        java.join("fixture-source/Wire.java"),
        "final class Wire {}\n",
    )
    .unwrap();
    fs::write(java.join("pom.xml"), r#"<project xmlns="http://maven.apache.org/POM/4.0.0">
  <modelVersion>4.0.0</modelVersion><groupId>io.hydracache.test</groupId>
  <artifactId>generation-process-probe</artifactId><version>0.0.0</version>
  <build><plugins>
    <plugin><groupId>org.apache.maven.plugins</groupId><artifactId>maven-clean-plugin</artifactId><version>3.2.0</version></plugin>
    <plugin><groupId>org.apache.maven.plugins</groupId><artifactId>maven-resources-plugin</artifactId><version>3.3.1</version>
      <executions><execution><id>fixture</id><phase>generate-sources</phase><goals><goal>copy-resources</goal></goals>
        <configuration><outputDirectory>${project.build.directory}/generated-sources/java</outputDirectory>
          <resources><resource><directory>fixture-source</directory><filtering>false</filtering></resource></resources>
        </configuration>
      </execution></executions>
    </plugin>
  </plugins></build>
</project>"#).unwrap();
    let owned = root.join("target/hc2-generation-check");
    fs::create_dir_all(&owned).unwrap();
    fs::write(owned.join("stale"), "must be removed before generation").unwrap();
    if case == "metadata-fail" {
        fs::write(root.join("Cargo.toml"), "not valid TOML").unwrap();
    } else if case == "java-fail" {
        fs::write(java.join("pom.xml"), "not valid XML").unwrap();
    }
}

#[test]
fn clean_generation_uses_real_cargo_and_maven_and_refuses_failed_or_changed_output() {
    for case in ["ok", "metadata-fail", "rust-fail", "rust-diff", "java-fail"] {
        let root = tempfile::tempdir().unwrap();
        fixture(root.path(), case);
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "client_plane_generation::process_tests::native_generation_child",
                "--nocapture",
            ])
            .env("HYDRACACHE_GENERATION_ROOT", root.path())
            .env("HYDRACACHE_GENERATION_CASE", case)
            // Instrument this test executable, but keep disposable build-script
            // binaries outside the registered coverage object/profile set.
            .env_remove("CARGO_ENCODED_RUSTFLAGS")
            .env_remove("RUSTFLAGS")
            .env_remove("CARGO_TARGET_DIR")
            .env("CARGO_BUILD_JOBS", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{case}: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if case == "ok" {
            assert_eq!(
                String::from_utf8_lossy(&output.stdout)
                    .matches("BUILD SUCCESS")
                    .count(),
                2,
                "both independent Java generations must finish"
            );
        }
    }
}

#[test]
fn native_generation_child() {
    let Some(root) = std::env::var_os("HYDRACACHE_GENERATION_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let case = std::env::var("HYDRACACHE_GENERATION_CASE").unwrap();
    let result = check_at_root(&root);
    assert!(!root.join("target/hc2-generation-check/stale").exists());
    if case == "ok" {
        result.unwrap();
        assert!(root.join("target/hc2-generation-check/rust-a").is_dir());
        assert!(root.join("target/hc2-generation-check/rust-b").is_dir());
        assert_eq!(
            fs::read(
                root.join(
                    "sdks/java/hydracache-client-hc2/target/generated-sources/java/Wire.java"
                )
            )
            .unwrap(),
            b"final class Wire {}\n"
        );
    } else {
        let expected = match case.as_str() {
            "metadata-fail" => "Rust HC/2 package metadata failed",
            "rust-fail" => "clean Rust HC/2 generation failed",
            "rust-diff" => "Rust generation is not byte-identical",
            "java-fail" => "clean Java HC/2 generation failed",
            _ => panic!("unknown generation test case"),
        };
        assert!(result.unwrap_err().to_string().contains(expected), "{case}");
        assert!(!root
            .join("sdks/java/hydracache-client-hc2/target/generated-sources")
            .exists());
    }
}
