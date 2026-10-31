//! Bounded observation of the current local daemon. No distributed backend is admitted.
use std::error::Error;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use hydracache_cluster_testkit::current_daemon_wire_075::{
    ClientErrorCode, ClientFrame, ClientRequest, ClientRequestEnvelope, ClientResponse,
    ClientWireMessage, Namespace, StructuredKey,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::imap_proof_process_075::{execute_observed, ProcessBounds};

const MAX_RESPONSE: usize = 64 * 1024;
pub const SCENARIOS: &[&str] = &[
    "readiness",
    "anonymous_refusal",
    "hc1_binary_put",
    "resp_observes_hc1",
    "resp_empty_key_and_value",
    "hc1_observes_resp",
    "tenant_isolation",
    "zero_deadline_refusal",
    "refusal_preserves_value",
    "redacted_metrics",
    "graceful_drain",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exchange {
    pub scenario: String,
    pub request: Vec<u8>,
    pub response: Vec<u8>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DaemonProbe {
    pub scope: String,
    pub binary_sha256: String,
    pub config_sha256: String,
    pub exchanges: Vec<Exchange>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub containment: String,
    pub root_reaped: bool,
    pub remaining_live_members: usize,
    pub released_listeners: bool,
    pub cluster_backend: String,
}

pub fn binary_digest(path: &Path) -> Result<String, Box<dyn Error>> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > 512 * 1024 * 1024 {
        return Err("daemon binary type/size bound".into());
    }
    let mut input = std::fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut block = [0u8; 32768];
    let mut total = 0;
    loop {
        let count = input.read(&mut block)?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > metadata.len() {
            return Err("daemon changed during hashing".into());
        }
        hash.update(&block[..count]);
    }
    if total != metadata.len() {
        return Err("daemon truncated during hashing".into());
    }
    Ok(hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn roundtrip(addr: SocketAddr, request: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(500))?;
    stream.set_read_timeout(Some(Duration::from_millis(500)))?;
    stream.set_write_timeout(Some(Duration::from_millis(500)))?;
    stream.write_all(request)?;
    let mut bytes = Vec::new();
    stream
        .take(MAX_RESPONSE as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_RESPONSE {
        return Err("daemon response bound".into());
    }
    Ok(bytes)
}

fn http_request(method: &str, path: &str, body: &[u8], identity: Option<&str>) -> Vec<u8> {
    let headers = identity.map(|tenant| format!("x-hydracache-client-id: imap075-observer\r\nx-hydracache-tenant: {tenant}\r\nx-hydracache-admin: true\r\n")).unwrap_or_default();
    let mut request = format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: {}\r\n{headers}\r\n", body.len()).into_bytes();
    request.extend_from_slice(body);
    request
}

pub fn http_response(raw: &[u8]) -> Result<(u16, &[u8]), Box<dyn Error>> {
    let split = raw
        .windows(4)
        .position(|part| part == b"\r\n\r\n")
        .ok_or("incomplete HTTP response")?;
    if split > 8192 || raw.len() > MAX_RESPONSE {
        return Err("HTTP response bound".into());
    }
    let headers = std::str::from_utf8(&raw[..split])?;
    let mut lines = headers.split("\r\n");
    let status = lines.next().ok_or("missing status")?;
    let fields = status.split(' ').collect::<Vec<_>>();
    if fields.len() < 2 || fields[0] != "HTTP/1.1" {
        return Err("unsupported HTTP status".into());
    }
    let status = fields[1].parse::<u16>()?;
    let mut length = None;
    for line in lines {
        let (name, value) = line.split_once(':').ok_or("invalid HTTP header")?;
        if name.eq_ignore_ascii_case("transfer-encoding") {
            return Err("unexpected transfer encoding".into());
        }
        if name.eq_ignore_ascii_case("content-length") {
            if length.is_some() {
                return Err("duplicate HTTP content-length".into());
            }
            length = Some(value.trim().parse::<usize>()?);
        }
    }
    let body = &raw[split + 4..];
    if length != Some(body.len()) {
        return Err("HTTP content-length mismatch".into());
    }
    Ok((status, body))
}

fn key(raw: &[u8]) -> StructuredKey {
    let name = if raw.is_empty() {
        "redis-binary-v1-empty".into()
    } else {
        format!(
            "redis-binary-v1-{}",
            raw.iter().map(|b| format!("{b:02x}")).collect::<String>()
        )
    };
    StructuredKey::new(vec![name]).expect("fixed development fixture key")
}
fn envelope(id: &str, request: ClientRequest) -> Result<Vec<u8>, Box<dyn Error>> {
    let envelope = ClientRequestEnvelope::new(id, request);
    let envelope = if id == "expired" {
        envelope.with_deadline_ms(0)
    } else {
        envelope
    };
    Ok(
        ClientFrame::from_message(&ClientWireMessage::Request(envelope))?
            .encode()?
            .to_vec(),
    )
}
fn decode(
    body: &[u8],
) -> Result<
    Result<
        ClientResponse,
        hydracache_cluster_testkit::current_daemon_wire_075::ClientErrorEnvelope,
    >,
    Box<dyn Error>,
> {
    match ClientFrame::decode(body, MAX_RESPONSE)?.decode_message()? {
        ClientWireMessage::Response(response) => Ok(response.result),
        _ => Err("non-response frame".into()),
    }
}
fn require(value: bool, message: &str) -> Result<(), Box<dyn Error>> {
    if value {
        Ok(())
    } else {
        Err(message.into())
    }
}

fn step(index: usize, addrs: &[SocketAddr; 3]) -> Result<Exchange, Box<dyn Error>> {
    let ns = Namespace::new("redis")?;
    let get = || ClientRequest::Get {
        ns: ns.clone(),
        key: key(&[0, 127, 255]),
    };
    let (addr, request) = match index {
        0 => (addrs[1], http_request("GET", "/readyz", &[], None)),
        1 => (
            addrs[0],
            http_request(
                "POST",
                "/client/v1/data",
                &envelope("anonymous", get())?,
                None,
            ),
        ),
        2 | 7 => (
            addrs[0],
            http_request(
                "POST",
                "/client/v1/data",
                &envelope(
                    if index == 2 { "binary-put" } else { "expired" },
                    ClientRequest::Put {
                        ns: ns.clone(),
                        key: key(&[0, 127, 255]),
                        value: if index == 2 {
                            vec![0, 128, 255]
                        } else {
                            vec![42]
                        },
                        ttl_ms: None,
                        dimensions: Vec::new(),
                    },
                )?,
                Some("redis"),
            ),
        ),
        3 => (
            addrs[2],
            b"*2\r\n$3\r\nGET\r\n$3\r\n\0\x7f\xff\r\n*1\r\n$4\r\nQUIT\r\n".to_vec(),
        ),
        4 => (
            addrs[2],
            b"*3\r\n$3\r\nSET\r\n$0\r\n\r\n$0\r\n\r\n*1\r\n$4\r\nQUIT\r\n".to_vec(),
        ),
        5 => (
            addrs[0],
            http_request(
                "POST",
                "/client/v1/data",
                &envelope(
                    "empty-get",
                    ClientRequest::Get {
                        ns: ns.clone(),
                        key: key(&[]),
                    },
                )?,
                Some("redis"),
            ),
        ),
        6 | 8 => (
            addrs[0],
            http_request(
                "POST",
                "/client/v1/data",
                &envelope(
                    if index == 6 {
                        "foreign-get"
                    } else {
                        "read-after-refusal"
                    },
                    get(),
                )?,
                Some(if index == 6 {
                    "foreign-diagnostic-tenant"
                } else {
                    "redis"
                }),
            ),
        ),
        9 => (addrs[1], http_request("GET", "/metrics", &[], None)),
        10 => (
            addrs[1],
            http_request("POST", "/admin/drain", &[], Some("system")),
        ),
        _ => return Err("unknown daemon scenario".into()),
    };
    let response = roundtrip(addr, &request)?;
    verify_exchange(index, &response)?;
    Ok(Exchange {
        scenario: SCENARIOS[index].into(),
        request,
        response,
    })
}

pub fn verify_exchange(index: usize, response: &[u8]) -> Result<(), Box<dyn Error>> {
    if index == 3 {
        return require(
            response == b"$3\r\n\0\x80\xff\r\n+OK\r\n",
            "RESP did not observe HC1 binary value",
        );
    }
    if index == 4 {
        return require(response == b"+OK\r\n+OK\r\n", "RESP empty SET refused");
    }
    let (status, body) = http_response(response)?;
    if index == 1 {
        return require(status == 401, "anonymous request accepted");
    }
    require(status == 200, "HTTP scenario refused")?;
    match index {
        0 => require(
            serde_json::from_slice::<serde_json::Value>(body)?["ready"] == true,
            "false readiness",
        ),
        2 => require(
            matches!(decode(body)?, Ok(ClientResponse::Stored)),
            "HC1 put failed",
        ),
        5 => require(
            matches!(decode(body)?, Ok(ClientResponse::Value { value: Some(v) }) if v.is_empty()),
            "HC1 did not observe RESP empty value",
        ),
        6 => require(
            matches!(decode(body)?, Ok(ClientResponse::Value { value: None })),
            "tenant data crossed boundary",
        ),
        7 => require(
            matches!(decode(body)?, Err(error) if error.code == ClientErrorCode::DeadlineExceeded),
            "expired mutation was accepted",
        ),
        8 => require(
            matches!(decode(body)?, Ok(ClientResponse::Value { value: Some(v) }) if v == [0,128,255]),
            "refused mutation changed value",
        ),
        9 => {
            let metrics = std::str::from_utf8(body)?;
            require(
                !metrics.contains("imap075-observer")
                    && !metrics.contains("foreign-diagnostic-tenant"),
                "unbounded identity in metrics",
            )
        }
        10 => require(
            serde_json::from_slice::<serde_json::Value>(body)?.is_object(),
            "drain result missing",
        ),
        _ => Err("unknown daemon scenario".into()),
    }
}

pub fn probe(binary: &Path, scratch: &Path) -> Result<DaemonProbe, Box<dyn Error>> {
    let before = binary_digest(binary)?;
    let temp = tempfile::tempdir_in(scratch)?;
    let sockets = [
        TcpListener::bind("127.0.0.1:0")?,
        TcpListener::bind("127.0.0.1:0")?,
        TcpListener::bind("127.0.0.1:0")?,
    ];
    let addrs = [
        sockets[0].local_addr()?,
        sockets[1].local_addr()?,
        sockets[2].local_addr()?,
    ];
    let mut command = Command::new(binary);
    command.env_clear();
    for name in ["SystemRoot", "TEMP", "TMP", "PATH"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let config = [
        ("HYDRACACHE_ROLE", "local".to_owned()),
        ("HYDRACACHE_CLIENT_API_ENABLED", "true".into()),
        ("HYDRACACHE_LISTEN_ADDR", addrs[0].to_string()),
        ("HYDRACACHE_ADMIN_API_ENABLED", "true".into()),
        ("HYDRACACHE_ADMIN_ADDR", addrs[1].to_string()),
        ("HYDRACACHE_REDIS_API_ENABLED", "true".into()),
        ("HYDRACACHE_REDIS_ADDR", addrs[2].to_string()),
        (
            "HYDRACACHE_STORAGE_DIR",
            temp.path().join("data").to_string_lossy().into_owned(),
        ),
    ];
    command.envs(config.iter().map(|(key, value)| (*key, value)));
    let config_sha256 = crate::imap_qualification_075::digest(b"current-daemon-local;hc1=true;redis=true;admin=true;hc2=false;tls=false;replicas=0;ephemeral-paths");
    drop(sockets);
    let mut exchanges = Vec::new();
    let output = execute_observed(
        command,
        &[],
        ProcessBounds {
            timeout: Duration::from_secs(20),
            stdout_bytes: 65536,
            stderr_bytes: 65536,
            ..ProcessBounds::default()
        },
        |observation| {
            if (observation.stdout.starts_with(b"{\"status\":\"ok\"}\n")
                || observation.stdout.starts_with(b"{\"status\":\"ok\"}\r\n"))
                && exchanges.len() < SCENARIOS.len()
                && !observation.root_exited
            {
                exchanges.push(step(exchanges.len(), &addrs).map_err(|e| e.to_string())?);
            }
            Ok(())
        },
    )?;
    require(
        exchanges.len() == SCENARIOS.len(),
        "daemon exited before all observations",
    )?;
    require(
        binary_digest(binary)? == before,
        "daemon binary changed during probe",
    )?;
    require(
        addrs.iter().all(|addr| TcpListener::bind(addr).is_ok()),
        "daemon listener survived cleanup",
    )?;
    Ok(DaemonProbe {
        scope: "current_daemon_local_characterization".into(),
        binary_sha256: before,
        config_sha256,
        exchanges,
        stdout: output.stdout,
        stderr: output.stderr,
        containment: output.scope.into(),
        root_reaped: output.direct_child_reaped,
        remaining_live_members: output.remaining_live_members,
        released_listeners: true,
        cluster_backend: "disabled_blocked_by_074".into(),
    })
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args != ["--release", "0.75"] {
        return Err("requires exactly --release 0.75".into());
    }
    let root = crate::doc_check::find_repo_root()?;
    let scratch = root.join("target/imap-075");
    std::fs::create_dir_all(&scratch)?;
    let binary = root.join("target/debug").join(if cfg!(windows) {
        "hydracache-server.exe"
    } else {
        "hydracache-server"
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&probe(&binary, &scratch)?)?
    );
    Ok(())
}

pub(crate) struct SoakDaemon {
    pub command: Command,
    pub addrs: [SocketAddr; 3],
    pub storage: tempfile::TempDir,
}
pub(crate) fn soak_command(binary: &Path, scratch: &Path) -> Result<SoakDaemon, Box<dyn Error>> {
    let storage = tempfile::tempdir_in(scratch)?;
    let sockets = [
        TcpListener::bind("127.0.0.1:0")?,
        TcpListener::bind("127.0.0.1:0")?,
        TcpListener::bind("127.0.0.1:0")?,
    ];
    let addrs = [
        sockets[0].local_addr()?,
        sockets[1].local_addr()?,
        sockets[2].local_addr()?,
    ];
    let mut command = Command::new(binary);
    command.env_clear();
    for name in ["SystemRoot", "TEMP", "TMP", "PATH"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    command
        .env("HYDRACACHE_ROLE", "local")
        .env("HYDRACACHE_CLIENT_API_ENABLED", "true")
        .env("HYDRACACHE_LISTEN_ADDR", addrs[0].to_string())
        .env("HYDRACACHE_ADMIN_API_ENABLED", "true")
        .env("HYDRACACHE_ADMIN_ADDR", addrs[1].to_string())
        .env("HYDRACACHE_REDIS_API_ENABLED", "true")
        .env("HYDRACACHE_REDIS_ADDR", addrs[2].to_string())
        .env("HYDRACACHE_STORAGE_DIR", storage.path().join("data"));
    drop(sockets);
    Ok(SoakDaemon {
        command,
        addrs,
        storage,
    })
}
pub(crate) fn soak_ready(addrs: &[SocketAddr; 3]) -> Result<(), Box<dyn Error>> {
    step(0, addrs).map(|_| ())
}
pub(crate) fn soak_drain(addrs: &[SocketAddr; 3]) -> Result<(), Box<dyn Error>> {
    step(10, addrs).map(|_| ())
}
pub fn soak_round(addrs: &[SocketAddr; 3], seed: u64, position: u64) -> Result<(), Box<dyn Error>> {
    soak_round_using(addrs, seed, position, roundtrip)
}
fn soak_round_using<T>(
    addrs: &[SocketAddr; 3],
    seed: u64,
    position: u64,
    mut transport: T,
) -> Result<(), Box<dyn Error>>
where
    T: FnMut(SocketAddr, &[u8]) -> Result<Vec<u8>, Box<dyn Error>>,
{
    let ns = Namespace::new("redis")?;
    let first = (seed ^ position).to_be_bytes();
    let second = (seed ^ position ^ 0x8000_0000_0000_0000).to_be_bytes();
    let request = |transport: &mut T, id: &str, body: ClientRequest| -> Result<_, Box<dyn Error>> {
        let raw = transport(
            addrs[0],
            &http_request(
                "POST",
                "/client/v1/data",
                &envelope(id, body)?,
                Some("redis"),
            ),
        )?;
        let (status, body) = http_response(&raw)?;
        require(status == 200, "soak HTTP request refused")?;
        decode(body)
    };
    require(
        matches!(
            request(
                &mut transport,
                "soak-put",
                ClientRequest::Put {
                    ns: ns.clone(),
                    key: key(b"a"),
                    value: first.to_vec(),
                    ttl_ms: None,
                    dimensions: vec![]
                }
            )?,
            Ok(ClientResponse::Stored)
        ),
        "soak HC1 put failed",
    )?;
    let response = transport(
        addrs[2],
        b"*2\r\n$3\r\nGET\r\n$1\r\na\r\n*1\r\n$4\r\nQUIT\r\n",
    )?;
    let mut expected = b"$8\r\n".to_vec();
    expected.extend_from_slice(&first);
    expected.extend_from_slice(b"\r\n+OK\r\n");
    require(response == expected, "soak RESP did not observe HC1 value")?;
    let mut set = b"*3\r\n$3\r\nSET\r\n$1\r\na\r\n$8\r\n".to_vec();
    set.extend_from_slice(&second);
    set.extend_from_slice(b"\r\n*1\r\n$4\r\nQUIT\r\n");
    require(
        transport(addrs[2], &set)? == b"+OK\r\n+OK\r\n",
        "soak RESP set failed",
    )?;
    require(
        matches!(request(&mut transport, "soak-get",ClientRequest::Get{ns:ns.clone(),key:key(b"a")})?,Ok(ClientResponse::Value{value:Some(value)}) if value==second),
        "soak HC1 did not observe RESP value",
    )?;
    require(
        matches!(
            request(
                &mut transport,
                "soak-remove",
                ClientRequest::Invalidate {
                    ns: ns.clone(),
                    key: key(b"a")
                }
            )?,
            Ok(ClientResponse::Invalidated)
        ),
        "soak invalidation failed",
    )?;
    require(
        matches!(
            request(
                &mut transport,
                "soak-absent",
                ClientRequest::Get { ns, key: key(b"a") }
            )?,
            Ok(ClientResponse::Value { value: None })
        ),
        "soak quiescent key remains",
    )
}

#[cfg(test)]
#[path = "../tests/support/imap_daemon_soak_transport_075.rs"]
mod soak_transport_tests;
