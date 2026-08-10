use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use prost::Message;
use prost_types::{FileDescriptorProto, FileDescriptorSet, MethodDescriptorProto};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const CRATE: &str = "hydracache-client-plane-spike";
const FIXTURE: &str = "crates/hydracache-client-plane-spike/python-fixture";
const GENERATED_PACKAGE: &str = "src/hydracache_hc2_generated";
const WHEEL_MANIFEST: &str = "wheelhouse.lock.json";
const REQUIREMENTS: &str = "requirements.lock";
const GENERATOR_VERSION: &str = "hydracache-hc2-python-1";

pub fn run_generate(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args.as_slice() != ["--write"] {
        return Err("client-plane-python-generate requires exactly --write".into());
    }
    let root = workspace_root()?;
    let checked_in = root.join(FIXTURE).join(GENERATED_PACKAGE);
    generate_to(&root, &checked_in)?;
    println!(
        "client-plane-python-generate: wrote {}",
        checked_in.display()
    );
    Ok(())
}

pub fn run_check(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if !args.is_empty() {
        return Err("client-plane-python-check does not accept arguments".into());
    }
    check_at_root(&workspace_root()?)
}

pub(crate) fn check_at_root(root: &Path) -> Result<(), Box<dyn Error>> {
    check_generated(root)?;
    let fixture = root.join(FIXTURE);
    let runtime = detect_python_runtime(&fixture)?;
    verify_supported_runtime(&fixture, &runtime)?;
    verify_wheelhouse(&fixture)?;
    let python = prepare_offline_venv(root, &fixture)?;
    run_python_tests(&python, &fixture)?;
    println!(
        "client-plane-python-check: OK ({} {}.{} {}, offline wheelhouse)",
        runtime.os, runtime.major, runtime.minor, runtime.arch
    );
    Ok(())
}

fn check_generated(root: &Path) -> Result<(), Box<dyn Error>> {
    let scratch = root.join("target/hc2-python-generation");
    generate_to(root, &scratch)?;
    let checked_in = root.join(FIXTURE).join(GENERATED_PACKAGE);
    let expected = directory_files(&checked_in)?;
    let actual = directory_files(&scratch)?;
    if expected != actual {
        let missing = actual
            .keys()
            .filter(|path| !expected.contains_key(*path))
            .cloned()
            .collect::<Vec<_>>();
        let extra = expected
            .keys()
            .filter(|path| !actual.contains_key(*path))
            .cloned()
            .collect::<Vec<_>>();
        let changed = actual
            .iter()
            .filter_map(|(path, bytes)| {
                expected
                    .get(path)
                    .filter(|expected| *expected != bytes)
                    .map(|_| path.clone())
            })
            .collect::<Vec<_>>();
        return Err(format!(
            "checked-in Python generation is dirty; missing={missing:?} extra={extra:?} changed={changed:?}; run cargo xtask client-plane-python-generate --write"
        )
        .into());
    }
    Ok(())
}

fn generate_to(root: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
    if output.exists() {
        fs::remove_dir_all(output)?;
    }
    fs::create_dir_all(output)?;
    let proto_root = root.join("crates").join(CRATE).join("proto");
    let mut protos = fs::read_dir(&proto_root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension() == Some(OsStr::new("proto")))
        .collect::<Vec<_>>();
    protos.sort();
    if protos.is_empty() {
        return Err("HC/2 Python generation found no proto sources".into());
    }

    let descriptor_path = output.join("contract_descriptor.bin");
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    let mut command = Command::new(&protoc);
    command
        .arg(format!("--proto_path={}", proto_root.display()))
        .arg(format!("--python_out={}", output.display()))
        .arg(format!("--pyi_out={}", output.display()))
        .arg(format!(
            "--descriptor_set_out={}",
            descriptor_path.display()
        ))
        .arg("--include_imports");
    for proto in &protos {
        command.arg(proto);
    }
    run_output(&mut command, "vendored protoc Python generation")?;

    let descriptor = FileDescriptorSet::decode(fs::read(&descriptor_path)?.as_slice())?;
    fs::remove_file(&descriptor_path)?;
    let version = run_output(
        Command::new(&protoc).arg("--version"),
        "vendored protoc version",
    )?;
    let protoc_version = String::from_utf8(version.stdout)?.trim().to_owned();
    generate_grpc_stubs(output, &descriptor)?;
    generate_metadata(output, &descriptor, &protoc_version)?;
    fs::write(output.join("__init__.py"), package_init(&descriptor))?;
    fs::write(output.join("py.typed"), b"")?;
    Ok(())
}

fn generate_grpc_stubs(
    output: &Path,
    descriptor: &FileDescriptorSet,
) -> Result<(), Box<dyn Error>> {
    let mut message_modules = BTreeMap::new();
    for file in &descriptor.file {
        let package = file.package.as_deref().unwrap_or_default();
        let module = python_module(file)?;
        for message in &file.message_type {
            let name = message
                .name
                .as_deref()
                .ok_or("descriptor message has no name")?;
            message_modules.insert(qualified_type(package, name), module.clone());
        }
    }
    for file in &descriptor.file {
        if file.service.is_empty() {
            continue;
        }
        let module = python_module(file)?;
        let path = output.join(format!("{module}_grpc.py"));
        fs::write(path, grpc_stub(file, &message_modules)?)?;
    }
    Ok(())
}

fn grpc_stub(
    file: &FileDescriptorProto,
    message_modules: &BTreeMap<String, String>,
) -> Result<String, Box<dyn Error>> {
    let package = file.package.as_deref().unwrap_or_default();
    let mut imports = BTreeSet::new();
    for service in &file.service {
        for method in &service.method {
            imports.insert(module_for_type(
                method.input_type.as_deref(),
                message_modules,
            )?);
            imports.insert(module_for_type(
                method.output_type.as_deref(),
                message_modules,
            )?);
        }
    }
    let mut code = format!("# Generated by {GENERATOR_VERSION}. DO NOT EDIT.\n\nimport grpc\n");
    for module in imports {
        code.push_str(&format!(
            "from . import {module} as {}\n",
            module_alias(&module)
        ));
    }
    code.push('\n');
    for service in &file.service {
        let service_name = service.name.as_deref().ok_or("service has no name")?;
        let qualified_service = if package.is_empty() {
            service_name.to_owned()
        } else {
            format!("{package}.{service_name}")
        };
        code.push_str(&format!("class {service_name}Stub:\n"));
        code.push_str("    def __init__(self, channel):\n");
        for method in &service.method {
            let method_name = method.name.as_deref().ok_or("method has no name")?;
            let input = python_type(method.input_type.as_deref(), message_modules)?;
            let output = python_type(method.output_type.as_deref(), message_modules)?;
            let cardinality = grpc_cardinality(method);
            code.push_str(&format!(
                "        self.{method_name} = channel.{cardinality}(\n            '/{qualified_service}/{method_name}',\n            request_serializer={input}.SerializeToString,\n            response_deserializer={output}.FromString,\n            _registered_method=True,\n        )\n"
            ));
        }
        code.push('\n');
        code.push_str(&format!("class {service_name}Servicer:\n"));
        for method in &service.method {
            let method_name = method.name.as_deref().ok_or("method has no name")?;
            code.push_str(&format!(
                "    def {method_name}(self, request_or_iterator, context):\n        context.set_code(grpc.StatusCode.UNIMPLEMENTED)\n        context.set_details('Method not implemented')\n        raise NotImplementedError('Method not implemented')\n\n"
            ));
        }
        code.push_str(&format!(
            "def add_{service_name}Servicer_to_server(servicer, server):\n    rpc_method_handlers = {{\n"
        ));
        for method in &service.method {
            let method_name = method.name.as_deref().ok_or("method has no name")?;
            let input = python_type(method.input_type.as_deref(), message_modules)?;
            let output = python_type(method.output_type.as_deref(), message_modules)?;
            let cardinality = grpc_cardinality(method);
            code.push_str(&format!(
                "        '{method_name}': grpc.{cardinality}_rpc_method_handler(\n            servicer.{method_name},\n            request_deserializer={input}.FromString,\n            response_serializer={output}.SerializeToString,\n        ),\n"
            ));
        }
        code.push_str(&format!(
            "    }}\n    generic_handler = grpc.method_handlers_generic_handler(\n        '{qualified_service}', rpc_method_handlers\n    )\n    server.add_generic_rpc_handlers((generic_handler,))\n    server.add_registered_method_handlers('{qualified_service}', rpc_method_handlers)\n\n"
        ));
    }
    Ok(format!("{}\n", code.trim_end()))
}

fn grpc_cardinality(method: &MethodDescriptorProto) -> &'static str {
    match (
        method.client_streaming.unwrap_or(false),
        method.server_streaming.unwrap_or(false),
    ) {
        (false, false) => "unary_unary",
        (false, true) => "unary_stream",
        (true, false) => "stream_unary",
        (true, true) => "stream_stream",
    }
}

fn python_type(
    type_name: Option<&str>,
    modules: &BTreeMap<String, String>,
) -> Result<String, Box<dyn Error>> {
    let type_name = type_name.ok_or("method type is absent")?;
    let module = module_for_type(Some(type_name), modules)?;
    let name = type_name
        .rsplit('.')
        .next()
        .filter(|name| !name.is_empty())
        .ok_or("method type name is invalid")?;
    Ok(format!("{}.{}", module_alias(&module), name))
}

fn module_for_type(
    type_name: Option<&str>,
    modules: &BTreeMap<String, String>,
) -> Result<String, Box<dyn Error>> {
    let type_name = type_name.ok_or("method type is absent")?;
    modules
        .get(type_name)
        .cloned()
        .ok_or_else(|| format!("no Python module owns descriptor type {type_name}").into())
}

fn module_alias(module: &str) -> String {
    format!("{}__pb2", module.trim_end_matches("_pb2"))
}

fn python_module(file: &FileDescriptorProto) -> Result<String, Box<dyn Error>> {
    let name = file.name.as_deref().ok_or("descriptor file has no name")?;
    let stem = Path::new(name)
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or("descriptor file name is invalid")?;
    Ok(format!("{stem}_pb2"))
}

fn qualified_type(package: &str, name: &str) -> String {
    if package.is_empty() {
        format!(".{name}")
    } else {
        format!(".{package}.{name}")
    }
}

#[derive(Serialize)]
struct ContractMetadata {
    schema_version: u32,
    generator: &'static str,
    protoc: String,
    files: Vec<MetadataFile>,
}

#[derive(Serialize)]
struct MetadataFile {
    name: String,
    package: String,
    messages: Vec<MetadataMessage>,
    enums: Vec<MetadataEnum>,
    services: Vec<MetadataService>,
}

#[derive(Serialize)]
struct MetadataMessage {
    name: String,
    fields: Vec<MetadataField>,
}

#[derive(Serialize)]
struct MetadataField {
    name: String,
    number: i32,
    label: i32,
    kind: i32,
    type_name: String,
    oneof_index: Option<i32>,
}

#[derive(Serialize)]
struct MetadataEnum {
    name: String,
    values: Vec<MetadataEnumValue>,
}

#[derive(Serialize)]
struct MetadataEnumValue {
    name: String,
    number: i32,
}

#[derive(Serialize)]
struct MetadataService {
    name: String,
    methods: Vec<MetadataMethod>,
}

#[derive(Serialize)]
struct MetadataMethod {
    name: String,
    input_type: String,
    output_type: String,
    client_streaming: bool,
    server_streaming: bool,
}

fn generate_metadata(
    output: &Path,
    descriptor: &FileDescriptorSet,
    protoc_version: &str,
) -> Result<(), Box<dyn Error>> {
    let mut files = descriptor
        .file
        .iter()
        .map(metadata_file)
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    files.sort_by(|left, right| left.name.cmp(&right.name));
    let metadata = ContractMetadata {
        schema_version: 1,
        generator: GENERATOR_VERSION,
        protoc: protoc_version.to_owned(),
        files,
    };
    let mut encoded = serde_json::to_vec_pretty(&metadata)?;
    encoded.push(b'\n');
    fs::write(output.join("contract_metadata.json"), encoded)?;
    Ok(())
}

fn metadata_file(file: &FileDescriptorProto) -> Result<MetadataFile, Box<dyn Error>> {
    let mut messages = file
        .message_type
        .iter()
        .map(|message| {
            let mut fields = message
                .field
                .iter()
                .map(|field| MetadataField {
                    name: field.name.clone().unwrap_or_default(),
                    number: field.number.unwrap_or_default(),
                    label: field.label.unwrap_or_default(),
                    kind: field.r#type.unwrap_or_default(),
                    type_name: field.type_name.clone().unwrap_or_default(),
                    oneof_index: field.oneof_index,
                })
                .collect::<Vec<_>>();
            fields.sort_by_key(|field| field.number);
            MetadataMessage {
                name: message.name.clone().unwrap_or_default(),
                fields,
            }
        })
        .collect::<Vec<_>>();
    messages.sort_by(|left, right| left.name.cmp(&right.name));
    let mut enums = file
        .enum_type
        .iter()
        .map(|enumeration| {
            let mut values = enumeration
                .value
                .iter()
                .map(|value| MetadataEnumValue {
                    name: value.name.clone().unwrap_or_default(),
                    number: value.number.unwrap_or_default(),
                })
                .collect::<Vec<_>>();
            values.sort_by_key(|value| value.number);
            MetadataEnum {
                name: enumeration.name.clone().unwrap_or_default(),
                values,
            }
        })
        .collect::<Vec<_>>();
    enums.sort_by(|left, right| left.name.cmp(&right.name));
    let mut services = file
        .service
        .iter()
        .map(|service| {
            let mut methods = service
                .method
                .iter()
                .map(|method| MetadataMethod {
                    name: method.name.clone().unwrap_or_default(),
                    input_type: method.input_type.clone().unwrap_or_default(),
                    output_type: method.output_type.clone().unwrap_or_default(),
                    client_streaming: method.client_streaming.unwrap_or(false),
                    server_streaming: method.server_streaming.unwrap_or(false),
                })
                .collect::<Vec<_>>();
            methods.sort_by(|left, right| left.name.cmp(&right.name));
            MetadataService {
                name: service.name.clone().unwrap_or_default(),
                methods,
            }
        })
        .collect::<Vec<_>>();
    services.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(MetadataFile {
        name: file.name.clone().ok_or("descriptor file has no name")?,
        package: file.package.clone().unwrap_or_default(),
        messages,
        enums,
        services,
    })
}

fn package_init(descriptor: &FileDescriptorSet) -> Vec<u8> {
    let mut modules = descriptor
        .file
        .iter()
        .filter_map(|file| {
            python_module(file)
                .ok()
                .map(|module| (module, !file.service.is_empty()))
        })
        .collect::<Vec<_>>();
    modules.sort_by(|left, right| left.0.cmp(&right.0));
    let mut source = format!(
        "\"\"\"Generated non-production HC/2 Python contract.\"\"\"\n\nGENERATOR_VERSION = \"{GENERATOR_VERSION}\"\n\n"
    );
    for (module, has_service) in &modules {
        source.push_str(&format!("from . import {module}\n"));
        if *has_service {
            source.push_str(&format!("from . import {module}_grpc\n"));
        }
    }
    source.push_str("\n__all__ = [\n");
    for (module, has_service) in &modules {
        source.push_str(&format!("    \"{module}\",\n"));
        if *has_service {
            source.push_str(&format!("    \"{module}_grpc\",\n"));
        }
    }
    source.push_str("]\n");
    source.into_bytes()
}

fn directory_files(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, Box<dyn Error>> {
    let mut files = BTreeMap::new();
    collect_files(root, root, &mut files)?;
    Ok(files)
}

fn collect_files(
    root: &Path,
    current: &Path,
    files: &mut BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, files)?;
        } else {
            files.insert(path.strip_prefix(root)?.to_owned(), fs::read(path)?);
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct WheelhouseManifest {
    schema_version: u32,
    supported_runtimes: Vec<SupportedRuntime>,
    files: Vec<WheelFile>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct SupportedRuntime {
    os: String,
    arch: String,
    major: u8,
    minor: u8,
}

#[derive(Debug, Deserialize)]
struct WheelFile {
    name: String,
    bytes: u64,
    sha256: String,
}

#[derive(Debug, Deserialize)]
struct DetectedRuntime {
    os: String,
    arch: String,
    major: u8,
    minor: u8,
}

fn load_wheel_manifest(fixture: &Path) -> Result<WheelhouseManifest, Box<dyn Error>> {
    let manifest: WheelhouseManifest =
        serde_json::from_slice(&fs::read(fixture.join(WHEEL_MANIFEST))?)?;
    if manifest.schema_version != 1 || manifest.files.is_empty() {
        return Err("unsupported or empty HC/2 Python wheelhouse manifest".into());
    }
    Ok(manifest)
}

fn verify_wheelhouse(fixture: &Path) -> Result<(), Box<dyn Error>> {
    let manifest = load_wheel_manifest(fixture)?;
    let wheelhouse = fixture.join("wheelhouse");
    let expected = manifest
        .files
        .iter()
        .map(|file| file.name.as_str())
        .collect::<BTreeSet<_>>();
    if expected.len() != manifest.files.len() {
        return Err("wheelhouse manifest contains duplicate filenames".into());
    }
    let actual = fs::read_dir(&wheelhouse)?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect::<BTreeSet<_>>();
    if actual.iter().map(String::as_str).collect::<BTreeSet<_>>() != expected {
        return Err(format!(
            "wheelhouse file set differs from {WHEEL_MANIFEST}: expected={expected:?} actual={actual:?}"
        )
        .into());
    }
    for file in &manifest.files {
        if Path::new(&file.name).file_name() != Some(OsStr::new(&file.name)) {
            return Err(format!("invalid wheel manifest file name: {}", file.name).into());
        }
        let bytes = fs::read(wheelhouse.join(&file.name))?;
        let digest = Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if bytes.len() as u64 != file.bytes || digest != file.sha256 {
            return Err(format!("wheel integrity mismatch: {}", file.name).into());
        }
    }
    Ok(())
}

fn detect_python_runtime(fixture: &Path) -> Result<DetectedRuntime, Box<dyn Error>> {
    let output = run_output(
        Command::new(python_program()).current_dir(fixture).args([
            "-c",
            "import json,platform,sys; print(json.dumps({'os': 'windows' if sys.platform == 'win32' else sys.platform, 'arch': 'x86_64' if platform.machine().lower() in ('amd64','x86_64') else platform.machine().lower(), 'major': sys.version_info.major, 'minor': sys.version_info.minor}))",
        ]),
        "supported Python runtime detection",
    )?;
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn verify_supported_runtime(
    fixture: &Path,
    runtime: &DetectedRuntime,
) -> Result<(), Box<dyn Error>> {
    let manifest = load_wheel_manifest(fixture)?;
    let supported = manifest.supported_runtimes.iter().any(|candidate| {
        candidate.os == runtime.os
            && candidate.arch == runtime.arch
            && candidate.major == runtime.major
            && candidate.minor == runtime.minor
    });
    if !supported {
        return Err(format!(
            "unsupported required HC/2 Python runtime: {} {}.{} {}; supported={:?}",
            runtime.os, runtime.major, runtime.minor, runtime.arch, manifest.supported_runtimes
        )
        .into());
    }
    Ok(())
}

fn prepare_offline_venv(root: &Path, fixture: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let venv = root.join("target/hc2-python-venv");
    run_status(
        Command::new(python_program())
            .args(["-m", "venv", "--clear"])
            .arg(&venv),
        "HC/2 Python virtual environment",
    )?;
    let python = if cfg!(windows) {
        venv.join("Scripts/python.exe")
    } else {
        venv.join("bin/python")
    };
    run_status(
        Command::new(&python)
            .arg("-m")
            .arg("pip")
            .arg("install")
            .arg("--no-index")
            .arg("--disable-pip-version-check")
            .arg("--require-hashes")
            .arg("--find-links")
            .arg(fixture.join("wheelhouse"))
            .arg("-r")
            .arg(fixture.join(REQUIREMENTS))
            .env("PIP_NO_INDEX", "1")
            .env("PIP_DISABLE_PIP_VERSION_CHECK", "1"),
        "offline HC/2 Python dependency install",
    )?;
    run_status(
        Command::new(&python)
            .args(["-m", "pip", "check"])
            .env("PIP_NO_INDEX", "1"),
        "offline HC/2 Python dependency consistency",
    )?;
    Ok(python)
}

fn run_python_tests(python: &Path, fixture: &Path) -> Result<(), Box<dyn Error>> {
    let source = fixture.join("src");
    run_status(
        Command::new(python)
            .args(["-m", "unittest", "discover", "-s", "tests", "-v"])
            .current_dir(fixture)
            .env("PYTHONPATH", source)
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .env("PIP_NO_INDEX", "1"),
        "generated Python HC/2 tests",
    )
}

fn python_program() -> &'static str {
    if cfg!(windows) {
        "python"
    } else {
        "python3"
    }
}

fn run_status(command: &mut Command, label: &str) -> Result<(), Box<dyn Error>> {
    let status = command
        .status()
        .map_err(|error| format!("starting {label}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{label} failed with {status}").into())
    }
}

fn run_output(command: &mut Command, label: &str) -> Result<Output, Box<dyn Error>> {
    let output = command
        .output()
        .map_err(|error| format!("starting {label}: {error}"))?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(format!(
            "{label} failed with {}; stdout={}; stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into())
    }
}

fn workspace_root() -> Result<PathBuf, Box<dyn Error>> {
    let mut candidate = std::env::current_dir()?;
    loop {
        if candidate.join("Cargo.toml").is_file() && candidate.join("crates").join(CRATE).is_dir() {
            return Ok(candidate);
        }
        if !candidate.pop() {
            return Err("could not locate HydraCache workspace root".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_in_python_generation_is_clean() {
        check_generated(&workspace_root().unwrap()).unwrap();
    }

    #[test]
    fn wheelhouse_is_exact_and_hash_verified() {
        let fixture = workspace_root().unwrap().join(FIXTURE);
        verify_wheelhouse(&fixture).unwrap();
    }
}
