use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener as StdTcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use futures_util::Stream;
use rcgen::{
    date_time_ymd, BasicConstraints, CertificateParams, CertifiedIssuer, ExtendedKeyUsagePurpose,
    IsCa, KeyPair,
};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::{ReceiverStream, TcpListenerStream};
use tonic::transport::{Certificate, Identity, Server, ServerTlsConfig};
use tonic::{Request, Response, Status, Streaming};

use hydracache_client_plane_spike::fixture_receipt::ProductionDaemonReadyReceipt;

mod wire {
    tonic::include_proto!("hydracache.client.v2alpha");
}

use wire::batch_item;
use wire::client_envelope;
use wire::client_plane_alpha_server::{ClientPlaneAlpha, ClientPlaneAlphaServer};
use wire::invocation_response;
use wire::server_envelope;
use wire::{
    BatchResult, CacheEvent, ClientEnvelope, HandshakeAck, InvocationResponse, LockOwnershipResult,
    LockResult, MutationResult, NodeEndpoint, ResponseMeta, RetryDirective, ServerEnvelope,
    SessionHeartbeat, StableErrorCode, SubscriptionAck, TopologyUpdate, ValueResult,
};

const GENERATION: u32 = 6;
const CONNECTION_GENERATION: u64 = 1;

type ResponseStream = Pin<Box<dyn Stream<Item = Result<ServerEnvelope, Status>> + Send>>;

#[derive(Debug, Default)]
struct Store {
    values: BTreeMap<Vec<u8>, Vec<u8>>,
    locks: BTreeMap<Vec<u8>, u64>,
    next_fence: u64,
}

#[derive(Debug, Default)]
struct ClosedReceipt {
    invocations: u64,
    events: u64,
    active_subscriptions: usize,
    active_sessions: usize,
}

#[derive(Clone)]
struct InteropService {
    accepted: Arc<AtomicBool>,
    store: Arc<Mutex<Store>>,
    invocations: Arc<AtomicU64>,
    events: Arc<AtomicU64>,
    closed: Arc<Mutex<Option<oneshot::Sender<ClosedReceipt>>>>,
}

#[tonic::async_trait]
impl ClientPlaneAlpha for InteropService {
    type OpenStream = ResponseStream;

    async fn open(
        &self,
        request: Request<Streaming<ClientEnvelope>>,
    ) -> Result<Response<Self::OpenStream>, Status> {
        if self.accepted.swap(true, Ordering::SeqCst) {
            return Err(Status::resource_exhausted(
                "interop server accepts exactly one connection",
            ));
        }
        let mut inbound = request.into_inner();
        let (tx, rx) = mpsc::channel(256);
        let service = self.clone();
        tokio::spawn(async move {
            let mut subscriptions = BTreeMap::<u64, (Vec<u8>, u64)>::new();
            let mut sessions = BTreeSet::<Vec<u8>>::new();
            let mut cancelled = BTreeSet::<u64>::new();
            let mut handshaken = false;
            let mut negotiated_generation = None;

            loop {
                let envelope = match inbound.message().await {
                    Ok(Some(envelope)) => envelope,
                    Ok(None) => break,
                    Err(_) => break,
                };
                if !(5..=GENERATION).contains(&envelope.generation)
                    || negotiated_generation
                        .is_some_and(|generation| generation != envelope.generation)
                    || envelope.connection_generation != CONNECTION_GENERATION
                {
                    let _ = tx
                        .send(Err(Status::failed_precondition(
                            "generation identity mismatch",
                        )))
                        .await;
                    break;
                }
                match envelope.message {
                    Some(client_envelope::Message::Handshake(handshake)) if !handshaken => {
                        if handshake.generation != envelope.generation {
                            let _ = tx
                                .send(Err(Status::failed_precondition(
                                    "handshake generation mismatch",
                                )))
                                .await;
                            break;
                        }
                        handshaken = true;
                        negotiated_generation = Some(envelope.generation);
                        let accepted = handshake.requested;
                        let ack = HandshakeAck {
                            generation: envelope.generation,
                            cluster_id: "hc2-java-interop".to_owned(),
                            accepted,
                            topology_epoch: 1,
                            connection_generation: CONNECTION_GENERATION,
                            minimum_generation: 5,
                            preferred_generation: GENERATION,
                            negotiated_generation_deprecated: envelope.generation < GENERATION,
                        };
                        if send(
                            &tx,
                            envelope.generation,
                            envelope.correlation_id,
                            server_envelope::Message::Handshake(ack),
                        )
                        .await
                        .is_err()
                        {
                            break;
                        }
                        let topology = TopologyUpdate {
                            epoch: 1,
                            nodes: vec![NodeEndpoint {
                                node_id: "interop-node".to_owned(),
                                node_epoch: 1,
                                endpoint_uri: "hc2+grpc://localhost".to_owned(),
                                server_name: "localhost".to_owned(),
                            }],
                        };
                        let _ = send(
                            &tx,
                            envelope.generation,
                            0,
                            server_envelope::Message::Topology(topology),
                        )
                        .await;
                    }
                    Some(client_envelope::Message::Handshake(_)) | None if !handshaken => {
                        let _ = tx
                            .send(Err(Status::failed_precondition("handshake must be first")))
                            .await;
                        break;
                    }
                    Some(client_envelope::Message::Invocation(invocation)) if handshaken => {
                        service.invocations.fetch_add(1, Ordering::SeqCst);
                        if is_delayed_probe(&invocation) {
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        }
                        let response = apply_invocation(&service.store, invocation);
                        if send(
                            &tx,
                            envelope.generation,
                            envelope.correlation_id,
                            server_envelope::Message::Invocation(response),
                        )
                        .await
                        .is_err()
                        {
                            break;
                        }
                        emit_matching_events(
                            &service,
                            &tx,
                            envelope.generation,
                            &mut subscriptions,
                        )
                        .await;
                    }
                    Some(client_envelope::Message::Cancel(cancel)) if handshaken => {
                        cancelled.insert(cancel.correlation_id);
                    }
                    Some(client_envelope::Message::Subscribe(subscribe)) if handshaken => {
                        subscriptions.insert(
                            subscribe.subscription_id,
                            (subscribe.key_prefix, subscribe.resume_watermark),
                        );
                        let ack = SubscriptionAck {
                            subscription_id: subscribe.subscription_id,
                            watermark: subscribe.resume_watermark,
                        };
                        let _ = send(
                            &tx,
                            envelope.generation,
                            envelope.correlation_id,
                            server_envelope::Message::Subscribed(ack),
                        )
                        .await;
                    }
                    Some(client_envelope::Message::Unsubscribe(unsubscribe)) if handshaken => {
                        subscriptions.remove(&unsubscribe.subscription_id);
                    }
                    Some(client_envelope::Message::SessionOpen(_)) if handshaken => {
                        let session_id = envelope.correlation_id.to_be_bytes().to_vec();
                        sessions.insert(session_id.clone());
                        let heartbeat = SessionHeartbeat {
                            session_id,
                            fence: 1,
                        };
                        let _ = send(
                            &tx,
                            envelope.generation,
                            envelope.correlation_id,
                            server_envelope::Message::SessionHeartbeat(heartbeat),
                        )
                        .await;
                    }
                    Some(client_envelope::Message::SessionHeartbeat(heartbeat)) if handshaken => {
                        if sessions.contains(&heartbeat.session_id) {
                            let _ = send(
                                &tx,
                                envelope.generation,
                                envelope.correlation_id,
                                server_envelope::Message::SessionHeartbeat(heartbeat),
                            )
                            .await;
                        }
                    }
                    Some(client_envelope::Message::SessionClose(close)) if handshaken => {
                        sessions.remove(&close.session_id);
                    }
                    _ => {
                        let _ = tx
                            .send(Err(Status::failed_precondition(
                                "unexpected or duplicate HC/2 message",
                            )))
                            .await;
                        break;
                    }
                }
            }
            drop(tx);
            if let Some(closed) = service.closed.lock().expect("closed receipt lock").take() {
                let _ = closed.send(ClosedReceipt {
                    invocations: service.invocations.load(Ordering::SeqCst),
                    events: service.events.load(Ordering::SeqCst),
                    active_subscriptions: subscriptions.len(),
                    active_sessions: sessions.len(),
                });
            }
            drop(cancelled);
        });
        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }
}

fn is_delayed_probe(invocation: &wire::InvocationRequest) -> bool {
    matches!(
        invocation.operation.as_ref(),
        Some(wire::invocation_request::Operation::Get(get))
            if get.key == b"__hc2_delay__"
    )
}

async fn send(
    tx: &mpsc::Sender<Result<ServerEnvelope, Status>>,
    generation: u32,
    correlation_id: u64,
    message: server_envelope::Message,
) -> Result<(), mpsc::error::SendError<Result<ServerEnvelope, Status>>> {
    tx.send(Ok(ServerEnvelope {
        generation,
        connection_generation: CONNECTION_GENERATION,
        correlation_id,
        message: Some(message),
    }))
    .await
}

fn success_meta() -> ResponseMeta {
    ResponseMeta {
        error: StableErrorCode::StableErrorUnspecified as i32,
        retry: RetryDirective::Never as i32,
        safe_detail: String::new(),
        topology_epoch: 1,
    }
}

fn apply_invocation(
    store: &Mutex<Store>,
    invocation: wire::InvocationRequest,
) -> InvocationResponse {
    use wire::invocation_request::Operation;
    let mut store = store.lock().expect("interop store lock");
    match invocation.operation {
        Some(Operation::Get(get)) => get_result(&store, &get.key),
        Some(Operation::Put(put)) => {
            store.values.insert(put.key, put.value);
            mutation_result(true)
        }
        Some(Operation::Delete(delete)) => {
            let applied = store.values.remove(&delete.key).is_some();
            mutation_result(applied)
        }
        Some(Operation::CompareAndSet(compare)) => {
            let applied = store
                .values
                .get(&compare.key)
                .is_some_and(|current| current == &compare.expected);
            if applied {
                store.values.insert(compare.key, compare.replacement);
            }
            mutation_result(applied)
        }
        Some(Operation::RemoveIfValue(remove)) => {
            let applied = store
                .values
                .get(&remove.key)
                .is_some_and(|current| current == &remove.expected);
            if applied {
                store.values.remove(&remove.key);
            }
            mutation_result(applied)
        }
        Some(Operation::TryLock(lock)) => {
            if store.locks.contains_key(&lock.key) {
                lock_result(false, 0)
            } else {
                store.next_fence = store.next_fence.saturating_add(1).max(1);
                let fence = store.next_fence;
                store.locks.insert(lock.key, fence);
                lock_result(true, fence)
            }
        }
        Some(Operation::Unlock(unlock)) => {
            let applied = store.locks.get(&unlock.key).copied() == Some(unlock.fence);
            if applied {
                store.locks.remove(&unlock.key);
            }
            mutation_result(applied)
        }
        Some(Operation::RenewLock(renew)) => {
            mutation_result(store.locks.get(&renew.key).copied() == Some(renew.fence))
        }
        Some(Operation::LockOwnership(ownership)) => {
            let fence = store.locks.get(&ownership.key).copied();
            InvocationResponse {
                meta: Some(success_meta()),
                result: Some(invocation_response::Result::LockOwnership(
                    LockOwnershipResult {
                        locked: fence.is_some(),
                        fence: fence.unwrap_or_default(),
                    },
                )),
            }
        }
        Some(Operation::Batch(batch)) => {
            let items = batch
                .items
                .into_iter()
                .map(|item| match item.operation {
                    Some(batch_item::Operation::Get(get)) => get_result(&store, &get.key),
                    Some(batch_item::Operation::Put(put)) => {
                        store.values.insert(put.key, put.value);
                        mutation_result(true)
                    }
                    Some(batch_item::Operation::Delete(delete)) => {
                        mutation_result(store.values.remove(&delete.key).is_some())
                    }
                    Some(batch_item::Operation::CompareAndSet(compare)) => {
                        let applied = store
                            .values
                            .get(&compare.key)
                            .is_some_and(|current| current == &compare.expected);
                        if applied {
                            store.values.insert(compare.key, compare.replacement);
                        }
                        mutation_result(applied)
                    }
                    None => error_result(
                        StableErrorCode::StableErrorInvalidRequest,
                        "batch item omitted operation",
                    ),
                })
                .collect();
            InvocationResponse {
                meta: Some(success_meta()),
                result: Some(invocation_response::Result::Batch(BatchResult { items })),
            }
        }
        None => error_result(
            StableErrorCode::StableErrorInvalidRequest,
            "invocation omitted operation",
        ),
    }
}

fn get_result(store: &Store, key: &[u8]) -> InvocationResponse {
    let value = store.values.get(key);
    InvocationResponse {
        meta: Some(success_meta()),
        result: Some(invocation_response::Result::Value(ValueResult {
            found: value.is_some(),
            value: value.cloned().unwrap_or_default(),
            expires_at_unix_ms: 0,
        })),
    }
}

fn mutation_result(applied: bool) -> InvocationResponse {
    InvocationResponse {
        meta: Some(success_meta()),
        result: Some(invocation_response::Result::Mutation(MutationResult {
            applied,
        })),
    }
}

fn lock_result(acquired: bool, fence: u64) -> InvocationResponse {
    InvocationResponse {
        meta: Some(success_meta()),
        result: Some(invocation_response::Result::Lock(LockResult {
            acquired,
            fence,
        })),
    }
}

fn error_result(code: StableErrorCode, detail: &str) -> InvocationResponse {
    InvocationResponse {
        meta: Some(ResponseMeta {
            error: code as i32,
            retry: RetryDirective::Never as i32,
            safe_detail: detail.to_owned(),
            topology_epoch: 1,
        }),
        result: None,
    }
}

async fn emit_matching_events(
    service: &InteropService,
    tx: &mpsc::Sender<Result<ServerEnvelope, Status>>,
    generation: u32,
    subscriptions: &mut BTreeMap<u64, (Vec<u8>, u64)>,
) {
    let latest = service
        .store
        .lock()
        .expect("interop store lock")
        .values
        .iter()
        .next_back()
        .map(|(key, value)| (key.clone(), value.clone()));
    let Some((key, value)) = latest else { return };
    for (subscription_id, (prefix, watermark)) in subscriptions.iter_mut() {
        if !key.starts_with(prefix) {
            continue;
        }
        *watermark = watermark.saturating_add(1);
        let event = CacheEvent {
            subscription_id: *subscription_id,
            watermark: *watermark,
            key: key.clone(),
            value: value.clone(),
            removed: false,
        };
        if send(tx, generation, 0, server_envelope::Message::Event(event))
            .await
            .is_ok()
        {
            service.events.fetch_add(1, Ordering::SeqCst);
        }
    }
}

struct Pki {
    ca_pem: String,
    server_cert_pem: String,
    server_key_pem: String,
    client_cert_pem: String,
    client_key_pem: String,
    second_client_cert_pem: String,
    second_client_key_pem: String,
}

struct ClientCredentialPaths {
    ca: PathBuf,
    first_cert: PathBuf,
    first_key: PathBuf,
    second_cert: PathBuf,
    second_key: PathBuf,
}

#[derive(Clone, Copy)]
enum PkiProfile {
    Normal,
    ExpiredServer,
    NotYetServer,
    WrongServerEku,
    ExpiredClient,
    NotYetClient,
    WrongClientEku,
    UntrustedClient,
}

impl PkiProfile {
    fn parse(value: &str) -> Result<Self, Box<dyn Error>> {
        match value {
            "normal" => Ok(Self::Normal),
            "expired-server" => Ok(Self::ExpiredServer),
            "not-yet-server" => Ok(Self::NotYetServer),
            "wrong-server-eku" => Ok(Self::WrongServerEku),
            "expired-client" => Ok(Self::ExpiredClient),
            "not-yet-client" => Ok(Self::NotYetClient),
            "wrong-client-eku" => Ok(Self::WrongClientEku),
            "untrusted-client" => Ok(Self::UntrustedClient),
            _ => Err(format!("unsupported PKI profile: {value}").into()),
        }
    }
}

fn generate_pki(profile: PkiProfile) -> Result<Pki, Box<dyn Error>> {
    let mut ca_params = CertificateParams::new(Vec::<String>::new())?;
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let ca = CertifiedIssuer::self_signed(ca_params, KeyPair::generate()?)?;

    let server_key = KeyPair::generate()?;
    let mut server_params = CertificateParams::new(vec!["localhost".to_owned()])?;
    if matches!(profile, PkiProfile::ExpiredServer) {
        server_params.not_before = date_time_ymd(2000, 1, 1);
        server_params.not_after = date_time_ymd(2001, 1, 1);
    } else if matches!(profile, PkiProfile::NotYetServer) {
        server_params.not_before = date_time_ymd(4090, 1, 1);
        server_params.not_after = date_time_ymd(4091, 1, 1);
    }
    server_params.extended_key_usages = vec![if matches!(profile, PkiProfile::WrongServerEku) {
        ExtendedKeyUsagePurpose::ClientAuth
    } else {
        ExtendedKeyUsagePurpose::ServerAuth
    }];
    let server_cert = server_params.signed_by(&server_key, &ca)?;

    let client_key = KeyPair::generate()?;
    let mut client_params = CertificateParams::new(vec!["hc2-java-client".to_owned()])?;
    if matches!(profile, PkiProfile::ExpiredClient) {
        client_params.not_before = date_time_ymd(2000, 1, 1);
        client_params.not_after = date_time_ymd(2001, 1, 1);
    } else if matches!(profile, PkiProfile::NotYetClient) {
        client_params.not_before = date_time_ymd(4090, 1, 1);
        client_params.not_after = date_time_ymd(4091, 1, 1);
    }
    client_params.extended_key_usages = vec![if matches!(profile, PkiProfile::WrongClientEku) {
        ExtendedKeyUsagePurpose::ServerAuth
    } else {
        ExtendedKeyUsagePurpose::ClientAuth
    }];
    let client_cert = if matches!(profile, PkiProfile::UntrustedClient) {
        let mut foreign_params = CertificateParams::new(Vec::<String>::new())?;
        foreign_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let foreign = CertifiedIssuer::self_signed(foreign_params, KeyPair::generate()?)?;
        client_params.signed_by(&client_key, &foreign)?
    } else {
        client_params.signed_by(&client_key, &ca)?
    };
    let second_client_key = KeyPair::generate()?;
    let mut second_client_params =
        CertificateParams::new(vec!["hc2-java-client-second".to_owned()])?;
    second_client_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
    let second_client_cert = second_client_params.signed_by(&second_client_key, &ca)?;

    Ok(Pki {
        ca_pem: ca.pem(),
        server_cert_pem: server_cert.pem(),
        server_key_pem: server_key.serialize_pem(),
        client_cert_pem: client_cert.pem(),
        client_key_pem: client_key.serialize_pem(),
        second_client_cert_pem: second_client_cert.pem(),
        second_client_key_pem: second_client_key.serialize_pem(),
    })
}

fn configuration() -> Result<(PathBuf, PkiProfile, Option<PathBuf>), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mut credentials = None;
    let mut profile = PkiProfile::Normal;
    let mut daemon = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--credentials-dir" if credentials.is_none() => {
                credentials = Some(PathBuf::from(
                    args.next().ok_or("missing credentials directory")?,
                ));
            }
            "--profile" => {
                profile = PkiProfile::parse(&args.next().ok_or("missing PKI profile")?)?;
            }
            "--daemon" if daemon.is_none() => {
                daemon = Some(PathBuf::from(args.next().ok_or("missing daemon path")?));
            }
            _ => return Err(format!("unsupported or duplicate argument: {argument}").into()),
        }
    }
    let path = credentials.ok_or(
        "usage: hc2_java_interop_server --credentials-dir <empty-directory> [--profile <name>] [--daemon <path>]",
    )?;
    if !path.is_dir() || path.read_dir()?.next().is_some() {
        return Err("credentials directory must exist and be empty".into());
    }
    if daemon.is_some() && !matches!(profile, PkiProfile::Normal) {
        return Err("production daemon fixture accepts only the normal PKI profile".into());
    }
    Ok((path, profile, daemon))
}

fn write_credential(path: &Path, name: &str, contents: &str) -> Result<PathBuf, Box<dyn Error>> {
    let target = path.join(name);
    std::fs::write(&target, contents.as_bytes())?;
    Ok(target)
}

fn reserve_addrs<const N: usize>() -> Result<([SocketAddr; N], Vec<StdTcpListener>), Box<dyn Error>>
{
    let listeners = (0..N)
        .map(|_| StdTcpListener::bind("127.0.0.1:0"))
        .collect::<Result<Vec<_>, _>>()?;
    let addrs = listeners
        .iter()
        .map(StdTcpListener::local_addr)
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|_| "reservation count must match the requested address count")?;
    Ok((addrs, listeners))
}

fn request_daemon_drain(admin: SocketAddr) -> Result<(), Box<dyn Error>> {
    let mut stream = TcpStream::connect_timeout(&admin, std::time::Duration::from_secs(5))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
    write!(
        stream,
        "POST /admin/drain HTTP/1.1\r\nHost: {admin}\r\nx-hydracache-client-id: java-daemon-fixture\r\nx-hydracache-tenant: system\r\nx-hydracache-admin: true\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )?;
    stream.flush()?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    let status = response.lines().next().unwrap_or_default();
    if !status.contains(" 200 ") {
        return Err(format!("production daemon drain failed: {status}").into());
    }
    Ok(())
}

fn run_daemon_fixture(
    daemon: &Path,
    credentials: &Path,
    pki: &Pki,
    client_paths: &ClientCredentialPaths,
) -> Result<(), Box<dyn Error>> {
    if !daemon.is_file() {
        return Err(format!("production daemon binary is missing: {}", daemon.display()).into());
    }
    let server_cert = write_credential(credentials, "server.pem", &pki.server_cert_pem)?;
    let server_key = write_credential(credentials, "server.key", &pki.server_key_pem)?;
    let client_ca = write_credential(credentials, "clients.pem", &pki.ca_pem)?;
    let ([client_addr, hc2_addr, admin_addr], reservations) = reserve_addrs::<3>()?;
    drop(reservations);
    let mut child = Command::new(daemon)
        .env("HYDRACACHE_CLIENT_API_ENABLED", "true")
        .env("HYDRACACHE_LISTEN_ADDR", client_addr.to_string())
        .env("HYDRACACHE_ADMIN_API_ENABLED", "true")
        .env("HYDRACACHE_ADMIN_ADDR", admin_addr.to_string())
        .env("HYDRACACHE_HC2_ENABLED", "true")
        .env("HYDRACACHE_HC2_ADDR", hc2_addr.to_string())
        .env("HYDRACACHE_HC2_CLUSTER_ID", "daemon-proof")
        .env("HYDRACACHE_TLS_ENABLED", "true")
        .env("HYDRACACHE_TLS_CERT_PATH", &server_cert)
        .env("HYDRACACHE_TLS_KEY_PATH", &server_key)
        .env("HYDRACACHE_TLS_CA_PATH", &client_ca)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let result = (|| -> Result<(), Box<dyn Error>> {
        let stdout = child
            .stdout
            .take()
            .ok_or("production daemon stdout is unavailable")?;
        let mut stdout = BufReader::new(stdout);
        let mut ready = String::new();
        stdout.read_line(&mut ready)?;
        if ready.trim() != r#"{"status":"ok"}"# {
            let _ = child.kill();
            let status = child.wait()?;
            let mut stderr = String::new();
            child
                .stderr
                .take()
                .ok_or("production daemon stderr is unavailable")?
                .read_to_string(&mut stderr)?;
            return Err(format!(
                "production daemon failed readiness: status={status}, receipt={:?}, stderr={stderr:?}",
                ready.trim()
            )
            .into());
        }
        let ready_receipt = ProductionDaemonReadyReceipt::new(
            hc2_addr.port(),
            admin_addr.port(),
            &client_paths.ca,
            &client_paths.first_cert,
            &client_paths.first_key,
            &client_paths.second_cert,
            &client_paths.second_key,
        )?;
        println!("{}", ready_receipt.encode());
        let mut command = String::new();
        std::io::stdin().lock().read_line(&mut command)?;
        if command.trim() != "DRAIN" {
            return Err("production daemon fixture expected DRAIN".into());
        }
        request_daemon_drain(admin_addr)?;
        let status = child.wait()?;
        if !status.success() {
            return Err(format!("production daemon failed after drain: {status}").into());
        }
        println!("CLOSED_DAEMON\tstatus=ok");
        Ok(())
    })();
    if result.is_err() && child.try_wait()?.is_none() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let (credentials, profile, daemon) = configuration()?;
    let pki = generate_pki(profile)?;
    let ca = write_credential(&credentials, "ca.pem", &pki.ca_pem)?;
    let client_cert = write_credential(&credentials, "client.pem", &pki.client_cert_pem)?;
    let client_key = write_credential(&credentials, "client.key", &pki.client_key_pem)?;
    let second_client_cert = write_credential(
        &credentials,
        "client-second.pem",
        &pki.second_client_cert_pem,
    )?;
    let second_client_key = write_credential(
        &credentials,
        "client-second.key",
        &pki.second_client_key_pem,
    )?;
    let client_paths = ClientCredentialPaths {
        ca,
        first_cert: client_cert,
        first_key: client_key,
        second_cert: second_client_cert,
        second_key: second_client_key,
    };

    if let Some(daemon) = daemon {
        return run_daemon_fixture(&daemon, &credentials, &pki, &client_paths);
    }

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let incoming = TcpListenerStream::new(listener);
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let (closed_tx, closed_rx) = oneshot::channel();
    let service = InteropService {
        accepted: Arc::new(AtomicBool::new(false)),
        store: Arc::new(Mutex::new(Store::default())),
        invocations: Arc::new(AtomicU64::new(0)),
        events: Arc::new(AtomicU64::new(0)),
        closed: Arc::new(Mutex::new(Some(closed_tx))),
    };
    let identity = Identity::from_pem(pki.server_cert_pem, pki.server_key_pem);
    let client_ca = Certificate::from_pem(pki.ca_pem);
    let server = tokio::spawn(async move {
        Server::builder()
            .tls_config(
                ServerTlsConfig::new()
                    .identity(identity)
                    .client_ca_root(client_ca),
            )?
            .add_service(ClientPlaneAlphaServer::new(service))
            .serve_with_incoming_shutdown(incoming, async {
                let _ = shutdown_rx.await;
            })
            .await
    });

    println!(
        "READY\t{port}\t{}\t{}\t{}",
        client_paths.ca.display(),
        client_paths.first_cert.display(),
        client_paths.first_key.display()
    );
    let receipt = closed_rx.await?;
    let _ = shutdown_tx.send(());
    server.await??;
    println!(
        "CLOSED\tinvocations={}\tevents={}\tactive_subscriptions={}\tactive_sessions={}",
        receipt.invocations, receipt.events, receipt.active_subscriptions, receipt.active_sessions
    );
    if receipt.active_subscriptions != 0 || receipt.active_sessions != 0 {
        return Err("HC/2 interop server retained resources at shutdown".into());
    }
    Ok(())
}
