use hydracache_long_run_supervisor_074::verify_journal;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    ExitCode::from(run())
}

fn run() -> u8 {
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next(), args.next()) {
        (Some("verify"), Some(path), None) => {
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
        (Some("serve"), Some(config), None) => serve(&PathBuf::from(config)),
        #[cfg(target_os = "linux")]
        (Some("request"), Some(socket), Some(request_path)) => {
            request(&PathBuf::from(socket), &PathBuf::from(request_path))
        }
        _ => {
            eprintln!(
                "usage: hydracache-long-run-supervisor-074 verify <checkpoints.jsonl> | serve <config.toml> | request <socket> <request.json>"
            );
            2
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
