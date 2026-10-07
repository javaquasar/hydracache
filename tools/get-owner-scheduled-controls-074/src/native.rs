//! Independent loopback native controls. No daemon, retries, or state clock overrides.
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use hydracache_client_hc2::{
    BatchItemResult, BatchOperation, ClientConfig, ClientError, ErrorCode, GrpcMtlsAdapter,
    GrpcMtlsConfig, Hc2Client,
};
use hydracache_client_protocol::{
    BatchPutEntry, ClientFrame, ClientRequest, ClientRequestEnvelope, ClientResponse,
    ClientWireMessage, Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{
    AxumClientSurface, ClientIdentity, ClientSurfaceLimits, ClientSurfaceState, CLIENT_DATA_PATH,
    HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
};
use hydracache_server::{serve_hc2_listener, Hc2ClientPlaneService, Hc2ListenerTls, TlsConfig};
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, ExtendedKeyUsagePurpose, IsCa, KeyPair,
};
use sha2::{Digest, Sha256};
use tokio::sync::{watch, Mutex};
use tokio::task::JoinHandle;

use crate::target::{PreloadOutcome, Target, TargetError, TargetOutcome, TargetRequest};

pub const SEED: u64 = 740074;
const MAX_BATCH_LOGICAL_BYTES: usize = 1_048_576;
type Result<T> = std::result::Result<T, String>;
type RequestResult<T> = std::result::Result<T, RequestFailure>;
#[derive(Debug, thiserror::Error)]
#[error("{detail}")]
struct RequestFailure {
    detail: String,
    outcome: TargetOutcome,
}
impl From<String> for RequestFailure {
    fn from(detail: String) -> Self {
        Self {
            detail,
            outcome: TargetOutcome::Error,
        }
    }
}
fn hc2_failure(error: ClientError) -> RequestFailure {
    let outcome = match error.code() {
        ErrorCode::DeadlineExceeded => TargetOutcome::Timeout,
        ErrorCode::QuotaExceeded => TargetOutcome::Rejected,
        _ => TargetOutcome::Error,
    };
    RequestFailure {
        detail: error.to_string(),
        outcome,
    }
}
fn http_failure(error: reqwest::Error) -> RequestFailure {
    RequestFailure {
        outcome: if error.is_timeout() {
            TargetOutcome::Timeout
        } else {
            TargetOutcome::Error
        },
        detail: error.to_string(),
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Surface {
    DirectClientSurface,
    Hc1Http,
    Hc2GrpcMtls,
}
#[derive(Clone, Copy, Debug)]
pub enum Operation {
    Get,
    Put,
    BatchGet { batch_size: usize },
    BatchPut { batch_size: usize },
}
impl Operation {
    pub fn batch_size(self) -> usize {
        match self {
            Self::Get | Self::Put => 1,
            Self::BatchGet { batch_size } | Self::BatchPut { batch_size } => batch_size,
        }
    }
    fn validate(self, dataset: &Dataset) -> Result<()> {
        if matches!(self, Self::BatchGet { .. } | Self::BatchPut { .. }) {
            let count = self.batch_size();
            if ![1, 8, 32, 128].contains(&count) {
                return Err("unsupported native batch size".to_owned());
            }
            // Both request PUT bytes and GET reply bytes are bounded before
            // constructing repeated values, PKI, state or transport owners.
            // Account for HC1/direct canonical hex keys (32 bytes each).
            let total = dataset
                .value
                .len()
                .checked_add(32)
                .and_then(|per_item| per_item.checked_mul(count));
            if total.is_none_or(|bytes| bytes > MAX_BATCH_LOGICAL_BYTES) {
                return Err("native batch logical byte budget exceeded".to_owned());
            }
        }
        Ok(())
    }
}

fn validate_surface_batch(response: ClientResponse, expected: &[Option<Bytes>]) -> Result<()> {
    let ClientResponse::Batch { items } = response else {
        return Err("native batch response kind mismatch".to_owned());
    };
    if items.len() != expected.len() {
        return Err("native batch response count mismatch".to_owned());
    }
    for (index, (item, value)) in items.iter().zip(expected).enumerate() {
        if item.index != index
            || item
                .result
                .as_ref()
                .ok()
                .is_none_or(|actual| actual.as_deref() != value.as_deref())
        {
            return Err("native batch response position/value/error mismatch".to_owned());
        }
    }
    Ok(())
}

fn validate_hc2_batch(
    items: &[BatchItemResult],
    expected: &[Option<Bytes>],
    put: bool,
) -> Result<()> {
    if items.len() != expected.len() || (put && expected.iter().any(Option::is_some)) {
        return Err("HC2 batch response count mismatch".to_owned());
    }
    for (index, (item, value)) in items.iter().zip(expected).enumerate() {
        let valid = if put {
            item.value.is_none() && item.mutation.is_some_and(|mutation| mutation.applied)
        } else {
            item.mutation.is_none()
                && item.value.as_ref().map(|v| &v.value) == value.as_ref()
                && item.value.as_ref().is_none_or(|v| v.expires_at.is_none())
        };
        if item.item_id as usize != index + 1 || !valid {
            return Err("HC2 batch response position/value/mutation mismatch".to_owned());
        }
    }
    Ok(())
}

/// Same binary dataset for either transport; security contexts are NOT equivalent.
pub struct Dataset {
    keys: Vec<Bytes>,
    value: Bytes,
}
impl Dataset {
    pub fn new(keyspace: usize, payload_bytes: usize) -> Result<Self> {
        if !(1..=16).contains(&keyspace) || !(1..=1_048_576).contains(&payload_bytes) {
            return Err("unsupported native dataset size".to_owned());
        }
        let keys = (0..keyspace)
            .map(|i| {
                let mut key = SEED.to_le_bytes().to_vec();
                key.extend_from_slice(&(i as u64).to_le_bytes());
                Bytes::from(key)
            })
            .collect();
        let value = Bytes::from(
            (0..payload_bytes)
                .map(|i| (i as u64 ^ SEED) as u8)
                .collect::<Vec<_>>(),
        );
        Ok(Self { keys, value })
    }
    pub(crate) fn entries(&self) -> Vec<(Bytes, Bytes)> {
        self.keys
            .iter()
            .map(|key| (key.clone(), self.value.clone()))
            .collect()
    }
    pub fn digest(&self) -> String {
        let mut hash = Sha256::new();
        hash.update(SEED.to_le_bytes());
        hash.update((self.keys.len() as u64).to_le_bytes());
        hash.update((self.value.len() as u64).to_le_bytes());
        for key in &self.keys {
            hash.update(key);
            hash.update(&self.value);
        }
        format!("{:x}", hash.finalize())
    }
}

/// Tool-owned ephemeral PKI. Intentionally neither Debug nor Serialize.
pub struct MtlsFixture {
    pub(crate) ca: String,
    pub(crate) server_cert: String,
    pub(crate) server_key: String,
    pub(crate) client_cert: String,
    pub(crate) client_key: String,
}
type Pki = MtlsFixture;
pub(crate) fn pki() -> Result<Pki> {
    let generate = || -> std::result::Result<Pki, rcgen::Error> {
        let mut ca_params = CertificateParams::new(Vec::<String>::new())?;
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let ca = CertifiedIssuer::self_signed(ca_params, KeyPair::generate()?)?;
        let server_key = KeyPair::generate()?;
        let mut server_params = CertificateParams::new(vec!["localhost".to_owned()])?;
        server_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let server_cert = server_params.signed_by(&server_key, &ca)?;
        let client_key = KeyPair::generate()?;
        let mut client_params = CertificateParams::new(vec!["scheduled-client".to_owned()])?;
        client_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
        let client_cert = client_params.signed_by(&client_key, &ca)?;
        Ok(Pki {
            ca: ca.pem(),
            server_cert: server_cert.pem(),
            server_key: server_key.serialize_pem(),
            client_cert: client_cert.pem(),
            client_key: client_key.serialize_pem(),
        })
    };
    generate().map_err(|e| e.to_string())
}
fn client_config(index: usize, tenant: &str) -> ClientConfig {
    let mut config = ClientConfig::new(format!("scheduled-{index}"), tenant);
    config.connect_timeout = Duration::from_secs(2);
    config.default_request_timeout = Duration::from_secs(2);
    config
}
fn adapter(endpoint: &str, trust: &Pki, identity: &Pki) -> Result<GrpcMtlsAdapter> {
    Ok(GrpcMtlsAdapter::new(
        GrpcMtlsConfig::new(
            endpoint,
            "localhost",
            trust.ca.as_bytes(),
            identity.client_cert.as_bytes(),
            identity.client_key.as_bytes(),
        )
        .map_err(|e| e.to_string())?,
    ))
}

enum Client {
    Direct(ClientIdentity),
    Hc1(reqwest::Client),
    Hc2(Hc2Client),
}
/// Owns a dynamically bound listener and all clients. Drop aborts its listener;
/// successful explicit shutdown additionally joins and checks HC2 accounting.
pub struct NativeControl {
    state: Arc<ClientSurfaceState>,
    endpoint: String,
    clients: Vec<Mutex<Client>>,
    dataset: Dataset,
    operation: Operation,
    shutdown: watch::Sender<bool>,
    listener: Option<JoinHandle<Result<()>>>,
    hc2: Option<Hc2ClientPlaneService>,
    _pki_files: Option<tempfile::TempDir>,
    transport_security: Option<crate::security::TransportReceipt>,
}
impl Drop for NativeControl {
    fn drop(&mut self) {
        for client in &mut self.clients {
            if let Client::Hc2(client) = client.get_mut() {
                client.close();
            }
        }
        let _ = self.shutdown.send(true);
        if let Some(listener) = self.listener.take() {
            listener.abort();
        }
    }
}
impl NativeControl {
    pub async fn start(
        surface: Surface,
        slots: usize,
        dataset: Dataset,
        operation: Operation,
    ) -> Result<Self> {
        Self::start_inner(surface, slots, dataset, operation, None).await
    }
    pub async fn start_hc2_mtls(
        slots: usize,
        dataset: Dataset,
        operation: Operation,
        fixture: &MtlsFixture,
    ) -> Result<Self> {
        Self::start_inner(
            Surface::Hc2GrpcMtls,
            slots,
            dataset,
            operation,
            Some(fixture),
        )
        .await
    }
    async fn start_inner(
        surface: Surface,
        slots: usize,
        dataset: Dataset,
        operation: Operation,
        fixture: Option<&MtlsFixture>,
    ) -> Result<Self> {
        if ![1, 8, 32, 128].contains(&slots) {
            return Err("unsupported client slot count".to_owned());
        }
        operation.validate(&dataset)?;
        let state = Arc::new(
            ClientSurfaceState::new(ClientSurfaceLimits {
                max_frame_bytes: 8 * 1024 * 1024,
                ..Default::default()
            })
            .map_err(|e| e.to_string())?,
        );
        state.set_profile_instrumentation_enabled(false);
        if matches!(surface, Surface::DirectClientSurface) {
            let (shutdown, _) = watch::channel(false);
            let clients = (0..slots)
                .map(|index| {
                    ClientIdentity::new(format!("scheduled-{index}"), "tenant-a")
                        .map(|identity| Mutex::new(Client::Direct(identity)))
                        .map_err(|e| e.to_string())
                })
                .collect::<Result<Vec<_>>>()?;
            let control = Self {
                state,
                endpoint: String::new(),
                clients,
                dataset,
                operation,
                shutdown,
                listener: None,
                hc2: None,
                _pki_files: None,
                transport_security: None,
            };
            control.preload().await.map_err(|e| e.to_string())?;
            return Ok(control);
        }
        let tcp = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| e.to_string())?;
        let address = tcp.local_addr().map_err(|e| e.to_string())?;
        let (shutdown, receiver) = watch::channel(false);
        let mut control = Self {
            state,
            endpoint: String::new(),
            clients: Vec::new(),
            dataset,
            operation,
            shutdown,
            listener: None,
            hc2: None,
            _pki_files: None,
            transport_security: None,
        };
        match surface {
            Surface::DirectClientSurface => unreachable!("direct control creates no listener"),
            Surface::Hc1Http => {
                control.endpoint = format!("http://{address}{CLIENT_DATA_PATH}");
                let router = AxumClientSurface::from_state(Arc::clone(&control.state)).routes();
                control.listener = Some(tokio::spawn(async move {
                    let mut receiver = receiver;
                    axum::serve(tcp, router)
                        .with_graceful_shutdown(async move {
                            while !*receiver.borrow() {
                                if receiver.changed().await.is_err() {
                                    break;
                                }
                            }
                        })
                        .await
                        .map_err(|e| e.to_string())
                }));
                for _ in 0..slots {
                    let client = reqwest::Client::builder()
                        .http1_only()
                        .no_proxy()
                        .pool_max_idle_per_host(1)
                        .timeout(Duration::from_secs(2))
                        .build()
                        .map_err(|e| e.to_string())?;
                    control.clients.push(Mutex::new(Client::Hc1(client)));
                }
            }
            Surface::Hc2GrpcMtls => {
                let generated;
                let material = if let Some(fixture) = fixture {
                    fixture
                } else {
                    generated = pki()?;
                    &generated
                };
                control.transport_security = Some(material.receipt());
                let files = tempfile::tempdir().map_err(|e| e.to_string())?;
                let cert = files.path().join("server.pem");
                let key = files.path().join("server.key");
                let ca = files.path().join("clients.pem");
                for (path, contents) in [
                    (&cert, &material.server_cert),
                    (&key, &material.server_key),
                    (&ca, &material.ca),
                ] {
                    std::fs::write(path, contents).map_err(|e| e.to_string())?;
                }
                let tls = Hc2ListenerTls::from_server_config(&TlsConfig {
                    enabled: true,
                    cert_path: Some(cert),
                    key_path: Some(key),
                    ca_path: Some(ca),
                    acknowledge_insecure: false,
                })
                .map_err(|e| e.to_string())?;
                control._pki_files = Some(files);
                control.endpoint = format!("https://{address}");
                let service =
                    Hc2ClientPlaneService::new(Arc::clone(&control.state), "scheduled-controls");
                control.hc2 = Some(service.clone());
                control.listener = Some(tokio::spawn(async move {
                    serve_hc2_listener(tcp, service, tls, receiver)
                        .await
                        .map_err(|e| e.to_string())
                }));
                let adapter = adapter(&control.endpoint, material, material)?;
                for index in 0..slots {
                    control.clients.push(Mutex::new(Client::Hc2(
                        Hc2Client::connect(&adapter, client_config(index, "tenant-a"))
                            .await
                            .map_err(|e| e.to_string())?,
                    )));
                }
            }
        }
        control.preload().await.map_err(|e| e.to_string())?;
        Ok(control)
    }
    pub fn dataset_digest(&self) -> String {
        self.dataset.digest()
    }
    pub fn transport_security(&self) -> Option<&crate::security::TransportReceipt> {
        self.transport_security.as_ref()
    }
    /// Allocation diagnostic workload, not scheduled latency measurement.
    pub async fn diagnostic_round(&self, write: bool, sequence: u64) -> Result<()> {
        let key = &self.dataset.keys[sequence as usize % self.dataset.keys.len()];
        if write {
            self.put(key, sequence).await.map_err(|e| e.to_string())
        } else {
            let value = self.get(key, sequence).await.map_err(|e| e.to_string())?;
            if value.as_ref() == Some(&self.dataset.value) {
                Ok(())
            } else {
                Err("diagnostic GET byte drift".to_owned())
            }
        }
    }
    pub async fn refill_dataset(&self) -> Result<()> {
        self.preload().await.map(|_| ()).map_err(|e| e.to_string())
    }
    pub async fn delete_dataset(&self) -> Result<()> {
        for (sequence, key) in self.dataset.keys.iter().enumerate() {
            let client = self.clients[sequence % self.clients.len()].lock().await;
            match &*client {
                Client::Direct(_) | Client::Hc1(_) => {
                    let (ns, key) = Self::hc1_key(key)?;
                    let response = self
                        .surface_request(
                            &client,
                            sequence as u64,
                            ClientRequest::Invalidate { ns, key },
                        )
                        .await
                        .map_err(|e| e.to_string())?;
                    if !matches!(response, ClientResponse::Invalidated) {
                        return Err("DELETE reply mismatch".to_owned());
                    }
                }
                Client::Hc2(client) => {
                    if !client
                        .delete(key.clone(), None)
                        .await
                        .map_err(|e| e.to_string())?
                        .applied
                    {
                        return Err("HC2 DELETE was not applied".to_owned());
                    }
                }
            }
        }
        for (sequence, key) in self.dataset.keys.iter().enumerate() {
            if self
                .get(key, sequence as u64)
                .await
                .map_err(|e| e.to_string())?
                .is_some()
            {
                return Err("deleted key remained visible".to_owned());
            }
        }
        if self.retained_entries() != 0 || self.retained_value_bytes() != 0 {
            return Err("deleted logical values retained".to_owned());
        }
        Ok(())
    }
    pub fn dispatch_attempts(&self) -> u64 {
        self.state.dispatch_attempts()
    }

    async fn surface_request(
        &self,
        client: &Client,
        sequence: u64,
        request: ClientRequest,
    ) -> RequestResult<ClientResponse> {
        match client {
            Client::Hc1(client) => self.hc1(client, sequence, "tenant-a", request).await,
            Client::Direct(identity) => {
                let envelope = ClientRequestEnvelope::new(sequence.to_string(), request);
                let version = envelope.protocol_version;
                let response = self.state.dispatch_verified_request(identity, envelope);
                if response.request_id != sequence.to_string()
                    || response.protocol_version != version
                {
                    return Err("direct response identity drift".to_owned().into());
                }
                response
                    .result
                    .map_err(|e| format!("direct error: {e:?}").into())
            }
            Client::Hc2(_) => Err("HC2 cannot use HC1/direct envelope".to_owned().into()),
        }
    }

    async fn batch(
        &self,
        keys: &[Bytes],
        values: Option<&[Bytes]>,
        expected: &[Option<Bytes>],
        sequence: u64,
    ) -> RequestResult<()> {
        if keys.is_empty()
            || keys.len() != expected.len()
            || values.is_some_and(|values| values.len() != keys.len())
        {
            return Err("invalid observer batch shape".to_owned().into());
        }
        let client = self.clients[sequence as usize % self.clients.len()]
            .lock()
            .await;
        match &*client {
            Client::Hc1(_) | Client::Direct(_) => {
                let ns = Namespace::new("hc2").map_err(|e| e.to_string())?;
                let keys = keys
                    .iter()
                    .map(Self::hc1_key)
                    .map(|result| result.map(|(_, key)| key))
                    .collect::<Result<Vec<_>>>()?;
                let request = if let Some(values) = values {
                    ClientRequest::BatchPut {
                        ns,
                        entries: keys
                            .into_iter()
                            .zip(values)
                            .map(|(key, value)| BatchPutEntry {
                                key,
                                value: value.to_vec(),
                            })
                            .collect(),
                    }
                } else {
                    ClientRequest::BatchGet { ns, keys }
                };
                validate_surface_batch(
                    self.surface_request(&client, sequence, request).await?,
                    expected,
                )
                .map_err(Into::into)
            }
            Client::Hc2(client) => {
                let operations = keys
                    .iter()
                    .enumerate()
                    .map(|(index, key)| {
                        if let Some(values) = values {
                            BatchOperation::Put {
                                key: key.clone(),
                                value: values[index].clone(),
                                ttl: None,
                            }
                        } else {
                            BatchOperation::Get { key: key.clone() }
                        }
                    })
                    .collect();
                let items = client.batch(operations, None).await.map_err(hc2_failure)?;
                validate_hc2_batch(&items, expected, values.is_some()).map_err(Into::into)
            }
        }
    }
    pub fn retained_entries(&self) -> usize {
        self.state.retained_state_for_diagnostics().store_entries
    }
    pub fn client_slots(&self) -> usize {
        self.clients.len()
    }
    pub fn hc2_active_connections(&self) -> Option<u64> {
        self.hc2
            .as_ref()
            .map(|service| service.accounting().active_connections)
    }
    pub fn retained_value_bytes(&self) -> usize {
        self.state.retained_state_for_diagnostics().value_bytes
    }

    async fn hc1(
        &self,
        client: &reqwest::Client,
        sequence: u64,
        tenant: &str,
        request: ClientRequest,
    ) -> RequestResult<ClientResponse> {
        let request_id = sequence.to_string();
        let envelope = ClientRequestEnvelope::new(request_id.clone(), request);
        let version = envelope.protocol_version;
        let frame = ClientFrame::from_message(&ClientWireMessage::Request(envelope))
            .map_err(|e| e.to_string())?
            .encode()
            .map_err(|e| e.to_string())?;
        let response = client
            .post(&self.endpoint)
            .header(
                HYDRACACHE_CLIENT_ID_HEADER,
                format!("scheduled-{}", sequence as usize % self.clients.len()),
            )
            .header(HYDRACACHE_TENANT_HEADER, tenant)
            .body(frame.to_vec())
            .send()
            .await
            .map_err(http_failure)?;
        if !response.status().is_success() {
            return Err(format!("HC1 HTTP {}", response.status()).into());
        }
        let bytes = response.bytes().await.map_err(http_failure)?;
        let ClientWireMessage::Response(envelope) = ClientFrame::decode(&bytes, 8 * 1024 * 1024)
            .map_err(|e| e.to_string())?
            .decode_message()
            .map_err(|e| e.to_string())?
        else {
            return Err("HC1 response frame expected".to_owned().into());
        };
        if envelope.request_id != request_id || envelope.protocol_version != version {
            return Err("HC1 response identity drift".to_owned().into());
        }
        envelope
            .result
            .map_err(|e| format!("HC1 error: {e:?}").into())
    }
    fn hc1_key(key: &Bytes) -> Result<(Namespace, StructuredKey)> {
        let hex: String = key.iter().map(|byte| format!("{byte:02x}")).collect();
        Ok((
            Namespace::new("hc2").map_err(|e| e.to_string())?,
            StructuredKey::new(vec![hex]).map_err(|e| e.to_string())?,
        ))
    }
    async fn get(&self, key: &Bytes, sequence: u64) -> RequestResult<Option<Bytes>> {
        let client = self.clients[sequence as usize % self.clients.len()]
            .lock()
            .await;
        match &*client {
            Client::Hc1(_) | Client::Direct(_) => {
                let (ns, key) = Self::hc1_key(key)?;
                match self
                    .surface_request(&client, sequence, ClientRequest::Get { ns, key })
                    .await?
                {
                    ClientResponse::Value { value } => Ok(value.map(Bytes::from)),
                    _ => Err("HC1 GET response mismatch".to_owned().into()),
                }
            }
            Client::Hc2(client) => client
                .get(key.clone(), None)
                .await
                .map(|v| v.map(|v| v.value))
                .map_err(hc2_failure),
        }
    }
    async fn put(&self, key: &Bytes, sequence: u64) -> RequestResult<()> {
        let client = self.clients[sequence as usize % self.clients.len()]
            .lock()
            .await;
        match &*client {
            Client::Hc1(_) | Client::Direct(_) => {
                let (ns, key) = Self::hc1_key(key)?;
                match self
                    .surface_request(
                        &client,
                        sequence,
                        ClientRequest::Put {
                            ns,
                            key,
                            value: self.dataset.value.to_vec(),
                            ttl_ms: None,
                            dimensions: vec![],
                        },
                    )
                    .await?
                {
                    ClientResponse::Stored => Ok(()),
                    _ => Err("HC1 PUT response mismatch".to_owned().into()),
                }
            }
            Client::Hc2(client) => {
                let result = client
                    .put(key.clone(), self.dataset.value.clone(), None, None)
                    .await
                    .map_err(hc2_failure)?;
                if result.applied {
                    Ok(())
                } else {
                    Err("HC2 PUT not applied".to_owned().into())
                }
            }
        }
    }
    pub async fn verify(&self) -> Result<String> {
        for (index, key) in self.dataset.keys.iter().enumerate() {
            if self
                .get(key, index as u64)
                .await
                .map_err(|e| e.to_string())?
                .as_ref()
                != Some(&self.dataset.value)
            {
                return Err("native dataset drift".to_owned());
            }
        }
        if self
            .get(&Bytes::from_static(b"missing-key"), 0)
            .await
            .map_err(|e| e.to_string())?
            .is_some()
        {
            return Err("native missing GET became a hit".to_owned());
        }
        let retained = self.state.retained_state_for_diagnostics();
        if self.state.profile_metrics() != Default::default() {
            return Err("product instrumentation unexpectedly active".to_owned());
        }
        if retained.store_entries != self.dataset.keys.len()
            || retained.value_bytes != self.dataset.keys.len() * self.dataset.value.len()
        {
            return Err("native retained dataset drift".to_owned());
        }
        Ok(self.dataset.digest())
    }
    pub async fn shutdown(mut self) -> Result<()> {
        for client in &mut self.clients {
            if let Client::Hc2(client) = client.get_mut() {
                client.close();
                let retained = client.retained_state();
                if !retained.closed
                    || retained.pending_invocations != 0
                    || retained.outbound_buffered_items != 0
                {
                    return Err("HC2 client owners not released".to_owned());
                }
            }
        }
        self.clients.clear();
        if self.listener.is_some() {
            self.shutdown.send(true).map_err(|e| e.to_string())?;
        }
        if let Some(mut listener) = self.listener.take() {
            match tokio::time::timeout(Duration::from_secs(5), &mut listener).await {
                Ok(result) => result.map_err(|e| e.to_string())??,
                Err(_) => {
                    listener.abort();
                    let _ = listener.await;
                    return Err("native listener drain timed out".to_owned());
                }
            }
        }
        if self
            .hc2
            .as_ref()
            .is_some_and(|service| !service.accounting().live_resources_zero())
        {
            return Err("HC2 server owners not released".to_owned());
        }
        Ok(())
    }
}
#[async_trait]
impl Target for NativeControl {
    // A control never flushes/reset-mutates the store to hide incorrect state.
    async fn reset(&self) -> std::result::Result<String, TargetError> {
        self.verify().await.map_err(TargetError::Reset)
    }
    async fn preload(&self) -> std::result::Result<PreloadOutcome, TargetError> {
        for (index, key) in self.dataset.keys.iter().enumerate() {
            self.put(key, index as u64)
                .await
                .map_err(|e| TargetError::Preload(e.to_string()))?;
        }
        Ok(PreloadOutcome {
            operations: self.dataset.keys.len() as u64,
            state_digest: self.verify().await.map_err(TargetError::Preload)?,
        })
    }
    async fn state_digest(&self) -> std::result::Result<String, TargetError> {
        self.verify().await.map_err(TargetError::Measurement)
    }
    async fn execute(&self, request: TargetRequest) -> TargetOutcome {
        let key = &self.dataset.keys[request.sequence as usize % self.dataset.keys.len()];
        let result = match self.operation {
            Operation::Get => self.get(key, request.sequence).await.and_then(|v| {
                if v.as_ref() == Some(&self.dataset.value) {
                    Ok(())
                } else {
                    Err("native GET value mismatch".to_owned().into())
                }
            }),
            Operation::Put => self.put(key, request.sequence).await,
            Operation::BatchGet { batch_size } | Operation::BatchPut { batch_size } => {
                let keys = (0..batch_size)
                    .map(|position| {
                        self.dataset.keys[(request.sequence as usize % self.dataset.keys.len()
                            + position)
                            % self.dataset.keys.len()]
                        .clone()
                    })
                    .collect::<Vec<_>>();
                let put = matches!(self.operation, Operation::BatchPut { .. });
                let values = put.then(|| vec![self.dataset.value.clone(); batch_size]);
                let expected = vec![
                    if put {
                        None
                    } else {
                        Some(self.dataset.value.clone())
                    };
                    batch_size
                ];
                self.batch(&keys, values.as_deref(), &expected, request.sequence)
                    .await
            }
        };
        result.map_or_else(|error| error.outcome, |()| TargetOutcome::Success)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydracache_client_hc2::{CacheValue, MutationResult};
    use hydracache_client_protocol::{BatchItemStatus, ClientErrorCode, ClientErrorEnvelope};

    #[test]
    fn batch_oracles_reject_count_order_error_value_and_mutation_drift() {
        let expected = vec![
            Some(Bytes::from_static(b"\0\xff")),
            None,
            Some(Bytes::new()),
        ];
        let items = expected
            .iter()
            .enumerate()
            .map(|(index, value)| BatchItemStatus {
                index,
                result: Ok(value.as_ref().map(|v| v.to_vec())),
            })
            .collect::<Vec<_>>();
        assert!(validate_surface_batch(
            ClientResponse::Batch {
                items: items.clone()
            },
            &expected
        )
        .is_ok());
        assert!(validate_surface_batch(ClientResponse::Stored, &expected).is_err());
        assert!(
            validate_surface_batch(ClientResponse::Batch { items: vec![] }, &expected).is_err()
        );
        for defect in 0..3 {
            let mut broken = items.clone();
            match defect {
                0 => broken[1].index = 0,
                1 => broken[1].result = Ok(Some(vec![])),
                _ => {
                    broken[1].result = Err(ClientErrorEnvelope::new(
                        ClientErrorCode::TooLarge,
                        false,
                        "fixture-only rejection",
                    ))
                }
            }
            assert!(
                validate_surface_batch(ClientResponse::Batch { items: broken }, &expected).is_err()
            );
        }
        let items = expected
            .iter()
            .enumerate()
            .map(|(index, value)| BatchItemResult {
                item_id: (index + 1) as u32,
                value: value.as_ref().map(|value| CacheValue {
                    value: value.clone(),
                    expires_at: None,
                }),
                mutation: None,
            })
            .collect::<Vec<_>>();
        assert!(validate_hc2_batch(&items, &expected, false).is_ok());
        assert!(validate_hc2_batch(&[], &expected, false).is_err());
        for defect in 0..4 {
            let mut broken = items.clone();
            match defect {
                0 => broken[0].item_id = 2,
                1 => broken[0].value.as_mut().unwrap().value = Bytes::new(),
                2 => broken[0].value.as_mut().unwrap().expires_at = Some(std::time::UNIX_EPOCH),
                _ => broken[0].mutation = Some(MutationResult { applied: true }),
            }
            assert!(validate_hc2_batch(&broken, &expected, false).is_err());
        }
        let put = BatchItemResult {
            item_id: 1,
            value: None,
            mutation: Some(MutationResult { applied: true }),
        };
        assert!(validate_hc2_batch(std::slice::from_ref(&put), &[None], true).is_ok());
        assert!(validate_hc2_batch(std::slice::from_ref(&put), &expected[..1], true).is_err());
        for broken in [
            BatchItemResult {
                mutation: None,
                ..put.clone()
            },
            BatchItemResult {
                mutation: Some(MutationResult { applied: false }),
                ..put.clone()
            },
            BatchItemResult {
                value: items[0].value.clone(),
                ..put
            },
        ] {
            assert!(validate_hc2_batch(&[broken], &[None], true).is_err());
        }
    }

    #[test]
    fn native_batch_budget_boundaries_are_checked_without_overflow() {
        for batch_size in [1, 8, 32, 128] {
            let ceiling = MAX_BATCH_LOGICAL_BYTES / batch_size - 32;
            for operation in [
                Operation::BatchGet { batch_size },
                Operation::BatchPut { batch_size },
            ] {
                assert!(operation
                    .validate(&Dataset::new(1, ceiling).unwrap())
                    .is_ok());
                assert!(operation
                    .validate(&Dataset::new(1, ceiling + 1).unwrap())
                    .is_err());
            }
        }
        assert_eq!(Operation::Get.batch_size(), 1);
        assert_eq!(Operation::Put.batch_size(), 1);
    }

    #[tokio::test]
    async fn independent_batch_transports_preserve_duplicates_misses_empty_and_binary_values() {
        for surface in [
            Surface::DirectClientSurface,
            Surface::Hc1Http,
            Surface::Hc2GrpcMtls,
        ] {
            let control =
                NativeControl::start(surface, 1, Dataset::new(2, 256).unwrap(), Operation::Get)
                    .await
                    .unwrap();
            let a = control.dataset.keys[0].clone();
            let b = control.dataset.keys[1].clone();
            let binary = Bytes::from_static(b"\0\xff\r\n");
            control
                .batch(
                    &[a.clone(), b.clone(), a.clone()],
                    Some(&[Bytes::from_static(b"old"), binary.clone(), Bytes::new()]),
                    &[None, None, None],
                    90,
                )
                .await
                .unwrap();
            control
                .batch(
                    &[
                        a.clone(),
                        Bytes::from_static(b"absent\0\xff"),
                        b.clone(),
                        a.clone(),
                    ],
                    None,
                    &[Some(Bytes::new()), None, Some(binary), Some(Bytes::new())],
                    91,
                )
                .await
                .unwrap();
            assert!(control.batch(&[], None, &[], 92).await.is_err());
            assert!(control
                .batch(std::slice::from_ref(&a), Some(&[]), &[None], 93)
                .await
                .is_err());
            assert!(control
                .batch(std::slice::from_ref(&a), None, &[], 94)
                .await
                .is_err());
            control.preload().await.unwrap();
            control.verify().await.unwrap();
            control.shutdown().await.unwrap();
        }
    }

    #[tokio::test]
    async fn direct_and_hc1_oversized_batch_put_reject_without_partial_mutation() {
        for surface in [Surface::DirectClientSurface, Surface::Hc1Http] {
            let control =
                NativeControl::start(surface, 1, Dataset::new(2, 256).unwrap(), Operation::Get)
                    .await
                    .unwrap();
            let keys = (0..256)
                .map(|index| {
                    if index == 0 {
                        control.dataset.keys[0].clone()
                    } else {
                        Bytes::from(format!("oversized-native-{index}"))
                    }
                })
                .collect::<Vec<_>>();
            let values = vec![Bytes::from_static(b"must-not-commit"); 256];
            let error = control
                .batch(&keys, Some(&values), &vec![None; 256], 100)
                .await
                .unwrap_err();
            assert!(error.detail.contains("TooLarge"), "{error}");
            let mut expected = vec![None; 256];
            expected[0] = Some(control.dataset.value.clone());
            for (keys, expected) in keys.chunks(128).zip(expected.chunks(128)) {
                control.batch(keys, None, expected, 101).await.unwrap();
            }
            control.verify().await.unwrap();
            control.shutdown().await.unwrap();
        }
    }

    #[tokio::test]
    async fn hc2_batch_limits_and_unapplied_item_are_not_atomic_surface_batch_semantics() {
        let control = NativeControl::start(
            Surface::Hc2GrpcMtls,
            1,
            Dataset::new(2, 256).unwrap(),
            Operation::Get,
        )
        .await
        .unwrap();
        let keys = vec![control.dataset.keys[0].clone(); 256];
        let before = control.dispatch_attempts();
        control
            .batch(
                &keys,
                None,
                &vec![Some(control.dataset.value.clone()); 256],
                100,
            )
            .await
            .unwrap();
        assert_eq!(control.dispatch_attempts() - before, 256);
        let before = control.dispatch_attempts();
        assert!(control
            .batch(
                &vec![keys[0].clone(); 1025],
                None,
                &vec![Some(control.dataset.value.clone()); 1025],
                101
            )
            .await
            .is_err());
        assert_eq!(control.dispatch_attempts(), before);
        {
            let slot = control.clients[0].lock().await;
            let Client::Hc2(client) = &*slot else {
                unreachable!()
            };
            let items = client
                .batch(
                    vec![
                        BatchOperation::Put {
                            key: control.dataset.keys[0].clone(),
                            value: Bytes::from_static(b"committed-first"),
                            ttl: None,
                        },
                        BatchOperation::CompareAndSet {
                            key: control.dataset.keys[1].clone(),
                            expected: Bytes::from_static(b"does-not-match"),
                            replacement: Bytes::from_static(b"must-not-apply"),
                            ttl: None,
                        },
                    ],
                    None,
                )
                .await
                .unwrap();
            assert!(items[0].mutation.unwrap().applied);
            assert!(!items[1].mutation.unwrap().applied);
        }
        assert_eq!(
            control.get(&control.dataset.keys[0], 102).await.unwrap(),
            Some(Bytes::from_static(b"committed-first"))
        );
        assert_eq!(
            control.get(&control.dataset.keys[1], 103).await.unwrap(),
            Some(control.dataset.value.clone())
        );
        control.preload().await.unwrap();
        control.shutdown().await.unwrap();
    }
    #[test]
    fn sdk_failure_codes_are_not_retried_or_conflated_with_success() {
        for (code, expected) in [
            (ErrorCode::DeadlineExceeded, TargetOutcome::Timeout),
            (ErrorCode::QuotaExceeded, TargetOutcome::Rejected),
            (ErrorCode::Unauthorized, TargetOutcome::Error),
        ] {
            let error = ClientError::new(
                code,
                hydracache_client_hc2::RetryAdvice::Never,
                "test-only failure",
            );
            assert_eq!(hc2_failure(error).outcome, expected);
        }
    }
    #[tokio::test]
    async fn hc1_rejects_anonymous_and_other_tenant_cannot_read_dataset() {
        let control = NativeControl::start(
            Surface::Hc1Http,
            1,
            Dataset::new(2, 256).unwrap(),
            Operation::Get,
        )
        .await
        .unwrap();
        {
            let slot = control.clients[0].lock().await;
            let Client::Hc1(client) = &*slot else {
                unreachable!()
            };
            let before = control.state.dispatch_attempts();
            let anonymous = client
                .post(&control.endpoint)
                .body(vec![0u8])
                .send()
                .await
                .unwrap();
            assert!(!anonymous.status().is_success());
            assert_eq!(control.state.dispatch_attempts(), before);
            let (ns, key) = NativeControl::hc1_key(&control.dataset.keys[0]).unwrap();
            assert_eq!(
                control
                    .hc1(client, 88, "tenant-b", ClientRequest::Get { ns, key })
                    .await
                    .unwrap(),
                ClientResponse::Value { value: None }
            );
        }
        control.verify().await.unwrap();
        control.shutdown().await.unwrap();
    }
    #[tokio::test]
    async fn hc2_rejects_foreign_ca_identity_before_dispatch() {
        let trust = pki().unwrap();
        let foreign = pki().unwrap();
        let files = tempfile::tempdir().unwrap();
        let cert = files.path().join("server.pem");
        let key = files.path().join("server.key");
        let ca = files.path().join("ca.pem");
        std::fs::write(&cert, &trust.server_cert).unwrap();
        std::fs::write(&key, &trust.server_key).unwrap();
        std::fs::write(&ca, &trust.ca).unwrap();
        let state = Arc::new(ClientSurfaceState::new(Default::default()).unwrap());
        let tcp = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("https://{}", tcp.local_addr().unwrap());
        let service = Hc2ClientPlaneService::new(Arc::clone(&state), "foreign-ca-test");
        let tls = Hc2ListenerTls::from_server_config(&TlsConfig {
            enabled: true,
            cert_path: Some(cert),
            key_path: Some(key),
            ca_path: Some(ca),
            acknowledge_insecure: false,
        })
        .unwrap();
        let (tx, rx) = watch::channel(false);
        let observed = service.clone();
        let listener = tokio::spawn(async move { serve_hc2_listener(tcp, service, tls, rx).await });
        let result = Hc2Client::connect(
            &adapter(&endpoint, &trust, &foreign).unwrap(),
            client_config(0, "tenant-a"),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(state.dispatch_attempts(), 0);
        let valid_adapter = adapter(&endpoint, &trust, &trust).unwrap();
        let tenant_a = Hc2Client::connect(&valid_adapter, client_config(0, "tenant-a"))
            .await
            .unwrap();
        let tenant_b = Hc2Client::connect(&valid_adapter, client_config(1, "tenant-b"))
            .await
            .unwrap();
        let key = Bytes::from_static(b"tenant-key");
        tenant_a
            .put(
                key.clone(),
                Bytes::from_static(b"tenant-a-value"),
                None,
                None,
            )
            .await
            .unwrap();
        assert!(tenant_b.get(key.clone(), None).await.unwrap().is_none());
        assert_eq!(
            tenant_a.get(key, None).await.unwrap().unwrap().value,
            Bytes::from_static(b"tenant-a-value")
        );
        tenant_a.close();
        tenant_b.close();
        tx.send(true).unwrap();
        tokio::time::timeout(Duration::from_secs(5), listener)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(observed.accounting().live_resources_zero());
    }
}
