use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use bytes::Bytes;
use futures_util::stream::{FuturesUnordered, StreamExt};
use hydracache_client_hc2::{ClientConfig, GrpcMtlsAdapter, GrpcMtlsConfig, Hc2Client};
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, ExtendedKeyUsagePurpose, IsCa, KeyPair,
};
use serde_json::json;

fn arguments() -> Result<(String, BTreeMap<String, String>), String> {
    let mut values = std::env::args().skip(1);
    let command = values.next().ok_or_else(|| "missing command".to_owned())?;
    let mut parsed = BTreeMap::new();
    while let Some(flag) = values.next() {
        if !flag.starts_with("--") {
            return Err(format!("unexpected argument: {flag}"));
        }
        let value = values
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        parsed.insert(flag, value);
    }
    Ok((command, parsed))
}

fn required(values: &BTreeMap<String, String>, name: &str) -> Result<String, String> {
    values
        .get(name)
        .cloned()
        .ok_or_else(|| format!("missing {name}"))
}

fn write(path: &Path, value: impl AsRef<[u8]>) -> Result<(), String> {
    std::fs::write(path, value).map_err(|error| format!("write {}: {error}", path.display()))
}

fn generate_pki(output: &Path) -> Result<(), String> {
    std::fs::create_dir_all(output)
        .map_err(|error| format!("create {}: {error}", output.display()))?;
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).map_err(|e| e.to_string())?;
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let ca =
        CertifiedIssuer::self_signed(ca_params, KeyPair::generate().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let server_key = KeyPair::generate().map_err(|e| e.to_string())?;
    let mut server_params =
        CertificateParams::new(vec!["localhost".to_owned()]).map_err(|e| e.to_string())?;
    server_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    let server_cert = server_params
        .signed_by(&server_key, &ca)
        .map_err(|e| e.to_string())?;
    let client_key = KeyPair::generate().map_err(|e| e.to_string())?;
    let mut client_params =
        CertificateParams::new(vec!["memory-hc2-client".to_owned()]).map_err(|e| e.to_string())?;
    client_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
    let client_cert = client_params
        .signed_by(&client_key, &ca)
        .map_err(|e| e.to_string())?;
    write(&output.join("ca.pem"), ca.pem())?;
    write(&output.join("server.pem"), server_cert.pem())?;
    write(&output.join("server.key"), server_key.serialize_pem())?;
    write(&output.join("client.pem"), client_cert.pem())?;
    write(&output.join("client.key"), client_key.serialize_pem())?;
    Ok(())
}

async fn hold(values: &BTreeMap<String, String>) -> Result<(), String> {
    let endpoint = required(values, "--endpoint")?;
    let ca = std::fs::read(required(values, "--ca")?).map_err(|e| e.to_string())?;
    let cert = std::fs::read(required(values, "--cert")?).map_err(|e| e.to_string())?;
    let key = std::fs::read(required(values, "--key")?).map_err(|e| e.to_string())?;
    let connections = required(values, "--connections")?
        .parse::<usize>()
        .map_err(|e| e.to_string())?;
    let slow_consumers = required(values, "--slow-consumers")?
        .parse::<usize>()
        .map_err(|e| e.to_string())?;
    if connections == 0 || slow_consumers > connections {
        return Err(
            "connections must be positive and slow consumers cannot exceed them".to_owned(),
        );
    }
    let ready = PathBuf::from(required(values, "--ready")?);
    let adapter = GrpcMtlsAdapter::new(
        GrpcMtlsConfig::new(endpoint, "localhost", ca, cert, key).map_err(|e| e.to_string())?,
    );
    let mut pending = FuturesUnordered::new();
    for index in 0..connections {
        let adapter = adapter.clone();
        pending.push(async move {
            Hc2Client::connect(
                &adapter,
                ClientConfig::new(format!("memory-m6-{index:04}"), "memory-campaign-071"),
            )
            .await
            .map_err(|e| e.to_string())
        });
    }
    let mut clients = Vec::with_capacity(connections);
    while let Some(client) = pending.next().await {
        clients.push(client?);
    }
    let mut subscriptions = Vec::with_capacity(slow_consumers);
    for client in clients.iter().take(slow_consumers) {
        subscriptions.push(
            client
                .subscribe(Bytes::from_static(b"memory:"), 0)
                .await
                .map_err(|e| e.to_string())?,
        );
    }
    if slow_consumers > 0 {
        let producer = &clients[0];
        for index in 0..1_100_u32 {
            producer
                .put(
                    Bytes::from_static(b"memory:0000000000000000"),
                    Bytes::from(index.to_be_bytes().to_vec()),
                    None,
                    None,
                )
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    write(
        &ready,
        serde_json::to_vec_pretty(&json!({
            "schema_version": 1,
            "release": "0.71",
            "connections": clients.len(),
            "slow_consumers": subscriptions.len(),
            "pressure_events": if subscriptions.is_empty() { 0 } else { 1_100 },
            "transport": "grpc-bidirectional-mtls",
            "ready": true
        }))
        .map_err(|e| e.to_string())?,
    )?;
    tokio::task::spawn_blocking(|| {
        let mut buffer = [0_u8; 1];
        let _ = std::io::stdin().read(&mut buffer);
    })
    .await
    .map_err(|e| e.to_string())?;
    drop(subscriptions);
    drop(clients);
    Ok(())
}

#[tokio::main]
async fn main() {
    let result = async {
        let (command, values) = arguments()?;
        match command.as_str() {
            "pki" => generate_pki(Path::new(&required(&values, "--output")?)),
            "hold" => hold(&values).await,
            _ => Err(format!("unknown command: {command}")),
        }
    }
    .await;
    if let Err(error) = result {
        eprintln!("memory HC/2 connections 0.71: {error}");
        std::process::exit(2);
    }
}
