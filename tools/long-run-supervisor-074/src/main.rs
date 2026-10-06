use hydracache_long_run_supervisor_074::verify_journal;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    ExitCode::from(run())
}

fn run() -> u8 {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [command, path] if command == "verify" => {
            match verify_journal(&PathBuf::from(path))
                .and_then(|report| serde_json::to_string(&report).map_err(Into::into))
            {
                Ok(report) => {
                    println!("{report}");
                    0
                }
                Err(error) => {
                    eprintln!("{error}");
                    9
                }
            }
        }
        #[cfg(target_os = "linux")]
        [command, config] if command == "serve" => serve(&PathBuf::from(config)),
        #[cfg(target_os = "linux")]
        [command, socket, request_path] if command == "request" => {
            request(&PathBuf::from(socket), &PathBuf::from(request_path))
        }
        #[cfg(target_os = "linux")]
        [command, socket, request_path, bundle_directory] if command == "request-start" => {
            request_start(
                &PathBuf::from(socket),
                &PathBuf::from(request_path),
                &PathBuf::from(bundle_directory),
            )
        }
        #[cfg(target_os = "linux")]
        [command, campaign_directory] if command == "collect-host-receipt" => {
            collect_host_receipt(&PathBuf::from(campaign_directory))
        }
        [command, request, key, issued, expires, output] if command == "build-request" => {
            build_request(
                &PathBuf::from(request),
                &PathBuf::from(key),
                issued,
                expires,
                &PathBuf::from(output),
            )
        }
        [command, key] if command == "derive-verification-key" => {
            derive_verification_key(&PathBuf::from(key))
        }
        [command, manifest, key, output] if command == "sign-provisioning-manifest" => {
            sign_provisioning_manifest(
                &PathBuf::from(manifest),
                &PathBuf::from(key),
                &PathBuf::from(output),
            )
        }
        [command, manifest, key, signature] if command == "verify-provisioning-manifest" => {
            verify_provisioning_manifest(
                &PathBuf::from(manifest),
                &PathBuf::from(key),
                &PathBuf::from(signature),
            )
        }
        #[cfg(target_os = "linux")]
        [command, config] if command == "validate-production-config" => {
            validate_production_config(&PathBuf::from(config))
        }
        #[cfg(target_os = "linux")]
        [command] if command == "systemd-smoke" => systemd_smoke(),
        #[cfg(target_os = "linux")]
        [command] if command == "controller-loss-smoke-start" => controller_loss_smoke_start(),
        #[cfg(target_os = "linux")]
        [command] if command == "controller-loss-smoke-resume" => controller_loss_smoke_resume(),
        #[cfg(target_os = "linux")]
        [command] if command == "campaign-lifecycle-smoke-start" => {
            campaign_lifecycle_smoke_start()
        }
        #[cfg(target_os = "linux")]
        [command] if command == "campaign-lifecycle-smoke-resume" => {
            campaign_lifecycle_smoke_resume()
        }
        [command] if command == "campaign-fixture-harness" => campaign_fixture_harness(),
        #[cfg(target_os = "linux")]
        [command] if command == "campaign-fixture-daemon" => campaign_fixture_daemon(),
        #[cfg(target_os = "linux")]
        [command] if command == "campaign-start-rehearsal-harness" => {
            campaign_start_rehearsal_harness()
        }
        #[cfg(target_os = "linux")]
        [command] if command == "campaign-start-rehearsal-daemon" => {
            campaign_start_rehearsal_daemon()
        }
        _ => {
            eprintln!(
                "usage: hydracache-long-run-supervisor-074 verify <checkpoints.jsonl> | derive-verification-key <signing-key-file> | sign-provisioning-manifest <manifest> <signing-key-file> <signature-output> | verify-provisioning-manifest <manifest> <verification-key-file> <signature-file> | build-request <request.json> <signing-key-file> <issued-unix-seconds> <expires-unix-seconds> <output.json> | validate-production-config <config.toml> | systemd-smoke | controller-loss-smoke-start | controller-loss-smoke-resume | campaign-lifecycle-smoke-start | campaign-lifecycle-smoke-resume | serve <config.toml> | request <socket> <request.json> | request-start <socket> <request.json> <bundle-directory> | collect-host-receipt <campaign-directory>"
            );
            2
        }
    }
}

#[cfg(target_os = "linux")]
fn campaign_lifecycle_smoke_start() -> u8 {
    print_json_result(
        hydracache_long_run_supervisor_074::campaign_smoke::start_campaign_lifecycle_smoke(),
    )
}

#[cfg(target_os = "linux")]
fn campaign_lifecycle_smoke_resume() -> u8 {
    print_json_result(
        hydracache_long_run_supervisor_074::campaign_smoke::resume_campaign_lifecycle_smoke(),
    )
}

#[cfg(target_os = "linux")]
fn campaign_fixture_harness() -> u8 {
    match hydracache_long_run_supervisor_074::campaign_smoke::run_fixture_harness() {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            9
        }
    }
}

#[cfg(target_os = "linux")]
fn campaign_fixture_daemon() -> u8 {
    match hydracache_long_run_supervisor_074::campaign_smoke::run_fixture_daemon() {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            9
        }
    }
}

#[cfg(target_os = "linux")]
fn campaign_start_rehearsal_harness() -> u8 {
    match hydracache_long_run_supervisor_074::campaign_smoke::run_start_rehearsal_harness() {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            9
        }
    }
}

#[cfg(target_os = "linux")]
fn campaign_start_rehearsal_daemon() -> u8 {
    match hydracache_long_run_supervisor_074::campaign_smoke::run_start_rehearsal_daemon() {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            9
        }
    }
}

#[cfg(target_os = "linux")]
fn systemd_smoke() -> u8 {
    match hydracache_long_run_supervisor_074::systemd_smoke::run_systemd_smoke()
        .map_err(|error| error.to_string())
        .and_then(|receipt| serde_json::to_string(&receipt).map_err(|error| error.to_string()))
    {
        Ok(receipt) => {
            println!("{receipt}");
            0
        }
        Err(error) => {
            eprintln!("{error}");
            9
        }
    }
}

#[cfg(target_os = "linux")]
fn controller_loss_smoke_start() -> u8 {
    print_json_result(
        hydracache_long_run_supervisor_074::systemd_smoke::start_controller_loss_smoke(),
    )
}

#[cfg(target_os = "linux")]
fn controller_loss_smoke_resume() -> u8 {
    print_json_result(
        hydracache_long_run_supervisor_074::systemd_smoke::resume_controller_loss_smoke(),
    )
}

#[cfg(target_os = "linux")]
fn print_json_result<T: serde::Serialize, E: std::fmt::Display>(result: Result<T, E>) -> u8 {
    match result
        .map_err(|error| error.to_string())
        .and_then(|receipt| serde_json::to_string(&receipt).map_err(|error| error.to_string()))
    {
        Ok(receipt) => {
            println!("{receipt}");
            0
        }
        Err(error) => {
            eprintln!("{error}");
            9
        }
    }
}

fn sign_provisioning_manifest(
    manifest: &std::path::Path,
    signing_key: &std::path::Path,
    output: &std::path::Path,
) -> u8 {
    match hydracache_long_run_supervisor_074::request_builder::sign_provisioning_manifest(
        manifest,
        signing_key,
        output,
    ) {
        Ok(digest) => {
            println!("{digest}");
            0
        }
        Err(error) => {
            eprintln!("{error}");
            2
        }
    }
}

fn verify_provisioning_manifest(
    manifest: &std::path::Path,
    verification_key: &std::path::Path,
    signature: &std::path::Path,
) -> u8 {
    match hydracache_long_run_supervisor_074::request_builder::verify_provisioning_manifest(
        manifest,
        verification_key,
        signature,
    ) {
        Ok(digest) => {
            println!("{digest}");
            0
        }
        Err(error) => {
            eprintln!("{error}");
            9
        }
    }
}

fn derive_verification_key(signing_key: &std::path::Path) -> u8 {
    match hydracache_long_run_supervisor_074::request_builder::derive_verification_key(signing_key)
    {
        Ok(key) => {
            println!("{key}");
            0
        }
        Err(error) => {
            eprintln!("{error}");
            2
        }
    }
}

#[cfg(target_os = "linux")]
fn validate_production_config(config: &std::path::Path) -> u8 {
    use hydracache_long_run_supervisor_074::config::ServerConfig;

    match std::fs::read(config)
        .map_err(|error| error.to_string())
        .and_then(|bytes| ServerConfig::parse(&bytes, true).map_err(|error| error.to_string()))
        .and_then(|parsed| {
            parsed
                .policy()
                .map(|_| ())
                .map_err(|error| error.to_string())
        }) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            9
        }
    }
}

#[cfg(target_os = "linux")]
fn request_start(
    socket: &std::path::Path,
    request_path: &std::path::Path,
    bundle_directory: &std::path::Path,
) -> u8 {
    use hydracache_long_run_supervisor_074::client::exchange_start_with_evidence;
    use hydracache_long_run_supervisor_074::protocol::parse_wire_request;
    use hydracache_long_run_supervisor_074::start_evidence::load_start_transport_inputs;
    use std::time::{SystemTime, UNIX_EPOCH};

    let result = std::fs::read(request_path)
        .map_err(|error| error.to_string())
        .and_then(|packet| {
            let wire = parse_wire_request(&packet).map_err(|error| error.to_string())?;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| error.to_string())?
                .as_secs();
            let (manifest, host_receipt) =
                load_start_transport_inputs(bundle_directory, &wire.request, now)
                    .map_err(|error| error.to_string())?;
            exchange_start_with_evidence(socket, &packet, &manifest, &host_receipt)
                .map_err(|error| error.to_string())
        });
    match result {
        Ok(response) => {
            let code = response.body.error_code.unwrap_or(0);
            match serde_json::to_string(&response) {
                Ok(encoded) => println!("{encoded}"),
                Err(error) => {
                    eprintln!("{error}");
                    return 11;
                }
            }
            u8::try_from(code).unwrap_or(11)
        }
        Err(error) => {
            eprintln!("{error}");
            11
        }
    }
}

fn build_request(
    request: &std::path::Path,
    signing_key: &std::path::Path,
    issued: &str,
    expires: &str,
    output: &std::path::Path,
) -> u8 {
    let result = issued
        .parse::<u64>()
        .ok()
        .zip(expires.parse::<u64>().ok())
        .ok_or_else(|| "authorization times must be unsigned integers".to_owned())
        .and_then(|(issued, expires)| {
            hydracache_long_run_supervisor_074::request_builder::build_signed_request(
                request,
                signing_key,
                issued,
                expires,
                output,
            )
            .map_err(|error| error.to_string())
        });
    match result {
        Ok(digest) => {
            println!("{digest}");
            0
        }
        Err(error) => {
            eprintln!("{error}");
            2
        }
    }
}

#[cfg(target_os = "linux")]
fn collect_host_receipt(campaign_directory: &std::path::Path) -> u8 {
    use hydracache_long_run_supervisor_074::host_receipt::{
        collect_host_observation, write_receipt_for_admission,
    };

    match collect_host_observation(campaign_directory)
        .and_then(|receipt| write_receipt_for_admission(&receipt, campaign_directory))
    {
        Ok(digest) => {
            println!("{digest}");
            0
        }
        Err(error) => {
            eprintln!("{error}");
            9
        }
    }
}

#[cfg(target_os = "linux")]
fn serve(config_path: &std::path::Path) -> u8 {
    use hydracache_long_run_supervisor_074::config::ServerConfig;
    use hydracache_long_run_supervisor_074::server::SupervisorServer;

    let result = std::fs::read(config_path)
        .map_err(|error| error.to_string())
        .and_then(|bytes| ServerConfig::parse(&bytes, true).map_err(|error| error.to_string()))
        .and_then(|config| SupervisorServer::bind(config).map_err(|error| error.to_string()))
        .and_then(|server| server.serve().map_err(|error| error.to_string()));
    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            11
        }
    }
}

#[cfg(target_os = "linux")]
fn request(socket: &std::path::Path, request_path: &std::path::Path) -> u8 {
    use hydracache_long_run_supervisor_074::client::exchange;

    let result = std::fs::read(request_path)
        .map_err(|error| error.to_string())
        .and_then(|packet| exchange(socket, &packet).map_err(|error| error.to_string()));
    match result {
        Ok(response) => {
            let code = response.body.error_code.unwrap_or(0);
            match serde_json::to_string(&response) {
                Ok(encoded) => println!("{encoded}"),
                Err(error) => {
                    eprintln!("{error}");
                    return 11;
                }
            }
            u8::try_from(code).unwrap_or(11)
        }
        Err(error) => {
            eprintln!("{error}");
            11
        }
    }
}
