use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use std::{env, fs};

use hydracache_client_transport_axum::{ClientSurfaceLimits, ClientSurfaceState};
use hydracache_redis_compat::{
    RedisAuthConfig, RedisListenerConfig, RedisRespServer, DEFAULT_REDIS_NAMESPACE,
};
use redis::Value;
use rustls::pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio_rustls::TlsAcceptor;

const CLIENT_MATRIX_ENV: &str = "HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS";
const CLIENT_DOCKER_FORCE_ENV: &str = "HYDRACACHE_FORCE_REDIS_CLIENT_DOCKER";
const REDIS_ORACLE_REQUIRE_ENV: &str = "HYDRACACHE_REQUIRE_REDIS_ORACLE";
const CLIENT_RUNTIME_SKIP_EXIT: i32 = 42;
const PINNED_REDIS_IMAGES: [&str; 2] = ["redis:6.2.14", "redis:7.2.5"];
const DOCKER_HOST_GATEWAY: &str = "host.docker.internal";
const PYTHON_CLIENT_DOCKER_IMAGE: &str = "python:3.13.7-slim";
const PYTHON_CLIENT_DOCKER_PACKAGE: &str = "redis==5.2.1";
const NODE_CLIENT_DOCKER_IMAGE: &str = "node:24.6.0-bookworm-slim";
const NODE_CLIENT_DOCKER_PACKAGE: &str = "redis@4.7.0 redlock@5.0.0-beta.2";
const JVM_CLIENT_DOCKER_IMAGE: &str = "maven:3.9.11-eclipse-temurin-17";
const LOCK_RELEASE_SCRIPT: &str =
    "if redis.call('get', KEYS[1]) == ARGV[1] then return redis.call('del', KEYS[1]) else return 0 end";
const LOCK_EXTEND_SCRIPT: &str =
    "if redis.call('get', KEYS[1]) == ARGV[1] then return redis.call('pexpire', KEYS[1], ARGV[2]) else return 0 end";
const LOCK_EXTEND_SCRIPT_REDIS_PY: &str = r#"
        local token = redis.call('get', KEYS[1])
        if not token or token ~= ARGV[1] then
            return 0
        end
        local expiration = redis.call('pttl', KEYS[1])
        if not expiration then
            expiration = 0
        end
        if expiration < 0 then
            return 0
        end

        local newttl = ARGV[2]
        if ARGV[3] == "0" then
            newttl = ARGV[2] + expiration
        end
        redis.call('pexpire', KEYS[1], newttl)
        return 1
    "#;

#[test]
fn redis_client_gate_manifest_and_docs_are_wired() {
    let gates = include_str!("../../../docs/GATES.md");
    let testing = include_str!("../../../docs/TESTING.md");
    let manifest = include_str!("../../../docs/integrations/redis_compat_conformance.json");

    assert!(gates.contains(CLIENT_MATRIX_ENV));
    assert!(gates.contains("--test redis_clients"));
    assert!(testing.contains("--test redis_clients"));
    assert!(testing.contains(PYTHON_CLIENT_DOCKER_PACKAGE));
    assert!(testing.contains(NODE_CLIENT_DOCKER_PACKAGE));
    assert!(manifest.contains(r#""images": ["redis:6.2.14", "redis:7.2.5"]"#));
    assert!(manifest.contains("redis-py==5.2.1"));
    assert!(manifest.contains("redis@4.7.0"));
    assert!(manifest.contains("redlock@5.0.0-beta.2"));
    assert!(manifest.contains("redis_oracle_supported_subset_matches_real_redis"));
    assert!(manifest.contains("redis_oracle_del_exists_counts_match_real_redis"));
    assert!(manifest.contains("redis_oracle_mget_nil_and_order_match_real_redis"));
    assert!(manifest.contains("redis_oracle_mset_atomicity_matches_real_redis"));
    assert!(manifest.contains("redis_oracle_ttl_matches_real_redis_with_bounded_tolerance"));
    assert!(manifest.contains("SET NX PX/EX"));
    assert!(manifest.contains("SET NX without TTL, SET XX/GET/KEEPTTL"));
    assert!(manifest.contains("SET EXAT/PXAT"));
    assert!(manifest.contains("set_nx_px_acquires_missing_key_and_contention_returns_null"));
    assert!(manifest.contains("client_matrix_set_nx_px_lock_idiom_acquires_contends_and_releases"));
    assert!(manifest.contains("sha1_hex_matches_known_answer_vectors"));
    assert!(
        manifest.contains("lock_script_sha_fingerprints_are_frozen_for_reviewed_client_versions")
    );
    assert!(manifest.contains("eval_known_unlock_script_deletes_only_matching_token"));
    assert!(manifest.contains("eval_redis_py_release_and_reacquire_scripts_are_exact_allowlisted"));
    assert!(manifest.contains("script_load_exists_and_evalsha_are_allowlist_scoped"));
    assert!(manifest
        .contains("eval_extend_script_maps_keys1_token_and_ttl_without_mutating_on_bad_args"));
    assert!(manifest.contains("redis_oracle_ttl_edge_cases_match_real_redis"));
    assert!(manifest.contains("redis_oracle_set_options_are_documented_divergence"));
    assert!(manifest.contains("select_zero_is_supported_as_noop_for_single_database_contract"));
    assert!(manifest.contains("resp_listener_select_zero_ok_and_nonzero_keeps_default_database"));
    assert!(manifest.contains("info_returns_minimal_honest_facade_state"));
    assert!(manifest.contains("info_section_argument_does_not_fabricate_redis_keyspace_state"));
    assert!(
        manifest.contains("resp_listener_info_probe_does_not_fabricate_keyspace_or_cluster_state")
    );
    assert!(manifest.contains("type_reports_string_or_none_through_client_surface"));
    assert!(manifest.contains("resp_listener_type_reports_string_and_none"));
    assert!(manifest.contains("redis_oracle_unsupported_divergence_is_documented"));
    assert!(manifest.contains("redis_oracle_hc_extensions_are_hydracache_only"));
    assert!(manifest.contains("client_matrix_runs_hydracache_tag_extension_scenario"));
    assert!(manifest
        .contains("hc_tag_settags_and_invalidate_tag_use_edge_local_index_and_client_surface"));
    assert!(manifest.contains("CLUSTER SLOTS/NODES/INFO"));
    assert!(manifest.contains("cluster_commands_decode_as_unsupported_standalone_contract"));
    assert!(manifest
        .contains("cluster_mode_commands_fail_loud_over_resp_without_topology_or_redirects"));
    assert!(manifest.contains("client_matrix_runs_mset_and_ttl_commands"));
    assert!(manifest.contains("client_matrix_runs_resp3_negotiation_scenario"));
    assert!(manifest.contains("client_matrix_runs_auth_required_connection_scenario"));
    assert!(manifest.contains("client_matrix_runs_rediss_required_connection_scenario"));
    assert!(
        manifest.contains("nightly_python_node_go_jvm_clients_bootstrap_and_run_supported_subset")
    );
    for ecosystem in ["python", "node", "go", "jvm"] {
        assert!(testing.to_ascii_lowercase().contains(ecosystem));
    }
}

#[test]
fn redis_client_heavy_gate_is_executable_and_env_gated() {
    let source = include_str!("redis_clients.rs");
    let gates = include_str!("../../../docs/GATES.md");
    let testing = include_str!("../../../docs/TESTING.md");

    for test_name in [
        "mainstream_redis_client_can_talk_to_the_facade",
        "nightly_python_node_go_jvm_clients_bootstrap_and_run_supported_subset",
        "redis_oracle_supported_subset_matches_real_redis",
        "redis_oracle_del_exists_counts_match_real_redis",
        "redis_oracle_mget_nil_and_order_match_real_redis",
        "redis_oracle_mset_atomicity_matches_real_redis",
        "redis_oracle_ttl_matches_real_redis_with_bounded_tolerance",
        "redis_oracle_ttl_edge_cases_match_real_redis",
        "client_matrix_runs_mset_and_ttl_commands",
        "client_matrix_runs_resp3_negotiation_scenario",
        "client_matrix_set_nx_px_lock_idiom_acquires_contends_and_releases",
        "client_matrix_runs_auth_required_connection_scenario",
        "client_matrix_runs_rediss_required_connection_scenario",
        "client_matrix_runs_hydracache_tag_extension_scenario",
        "redis_oracle_set_nx_px_lock_acquire_matches_real_redis",
        "redis_oracle_lock_unlock_script_matches_real_redis",
        "redis_oracle_lock_extend_script_matches_real_redis_with_ttl_tolerance",
        "redis_oracle_set_options_are_documented_divergence",
        "redis_oracle_unsupported_divergence_is_documented",
        "redis_oracle_hc_extensions_are_hydracache_only",
    ] {
        assert!(
            source.contains(&format!("async fn {test_name}"))
                || source.contains(&format!("fn {test_name}"))
        );
        assert!(source.contains("#[ignore"));
    }
    for env_var in [
        CLIENT_MATRIX_ENV,
        CLIENT_DOCKER_FORCE_ENV,
        REDIS_ORACLE_REQUIRE_ENV,
        "HYDRACACHE_REQUIRE_REDIS_CLIENT_PYTHON",
        "HYDRACACHE_REQUIRE_REDIS_CLIENT_NODE",
        "HYDRACACHE_REQUIRE_REDIS_CLIENT_GO",
        "HYDRACACHE_REQUIRE_REDIS_CLIENT_JVM",
    ] {
        assert!(source.contains(env_var));
        assert!(testing.contains(env_var));
    }
    assert!(gates.contains("--test redis_clients"));
    for docker_image in [
        PYTHON_CLIENT_DOCKER_IMAGE,
        NODE_CLIENT_DOCKER_IMAGE,
        JVM_CLIENT_DOCKER_IMAGE,
    ] {
        assert!(source.contains(docker_image));
        assert!(testing.contains(docker_image));
    }
    assert!(source.contains(DOCKER_HOST_GATEWAY));
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1; uses mainstream redis-rs client path"]
async fn mainstream_redis_client_can_talk_to_the_facade() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping Redis client matrix; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }

    let (shutdown, addr, serving) = spawn_resp_facade().await;
    let client = redis::Client::open(format!("redis://{addr}/0")).unwrap();
    let mut connection = client.get_multiplexed_async_connection().await.unwrap();

    let pong: String = redis::cmd("PING")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(pong, "PONG");

    let info: String = redis::cmd("INFO")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert!(info.contains("redis_mode:standalone"));
    assert!(info.contains("hydracache_resp:RESP2+RESP3"));
    assert!(!info.contains("used_memory"));
    assert!(!info.contains("db0:"));

    let _: () = redis::cmd("SET")
        .arg("k")
        .arg("v")
        .query_async(&mut connection)
        .await
        .unwrap();
    let value: String = redis::cmd("GET")
        .arg("k")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(value, "v");

    let existing_type: String = redis::cmd("TYPE")
        .arg("k")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(existing_type, "string");
    let missing_type: String = redis::cmd("TYPE")
        .arg("missing")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(missing_type, "none");

    let values: Vec<Option<String>> = redis::cmd("MGET")
        .arg("k")
        .arg("missing")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(values, vec![Some("v".to_owned()), None]);

    run_redis_rs_mset_ttl_scenario(&mut connection, "rust")
        .await
        .unwrap();
    run_redis_rs_lock_scenario(&mut connection, "rust")
        .await
        .unwrap();

    let exists: i64 = redis::cmd("EXISTS")
        .arg("k")
        .arg("missing")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(exists, 1);

    let deleted: i64 = redis::cmd("DEL")
        .arg("k")
        .arg("missing")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(deleted, 1);

    drop(connection);
    drop(shutdown);
    serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1; proves MSET and TTL client commands"]
async fn client_matrix_runs_mset_and_ttl_commands() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping Redis client matrix; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }

    let (shutdown, addr, serving) = spawn_resp_facade().await;
    let client = redis::Client::open(format!("redis://{addr}/")).unwrap();
    let mut connection = client.get_multiplexed_async_connection().await.unwrap();

    run_redis_rs_mset_ttl_scenario(&mut connection, "matrix")
        .await
        .unwrap();

    drop(connection);
    drop(shutdown);
    serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1; proves SET NX PX lock acquire/contention/release"]
async fn client_matrix_set_nx_px_lock_idiom_acquires_contends_and_releases() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping Redis client matrix; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }

    let (shutdown, addr, serving) = spawn_resp_facade().await;
    let client = redis::Client::open(format!("redis://{addr}/")).unwrap();
    let mut connection = client.get_multiplexed_async_connection().await.unwrap();

    tokio::time::timeout(
        Duration::from_secs(2),
        run_redis_rs_lock_scenario(&mut connection, "rust-lock"),
    )
    .await
    .expect("SET NX PX lock scenario should finish promptly")
    .unwrap();

    drop(connection);
    drop(shutdown);
    serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1; proves redis-rs RESP3 negotiation"]
async fn client_matrix_runs_resp3_negotiation_scenario() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping Redis client matrix; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }

    let (shutdown, addr, serving) = spawn_resp_facade().await;
    let client = redis::Client::open(format!("redis://{addr}/?protocol=resp3")).unwrap();
    let mut connection = client.get_multiplexed_async_connection().await.unwrap();

    let pong: String = redis::cmd("PING")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(pong, "PONG");

    run_redis_rs_mset_ttl_scenario(&mut connection, "resp3")
        .await
        .unwrap();

    let _: () = redis::cmd("SET")
        .arg("resp3:k")
        .arg("v")
        .query_async(&mut connection)
        .await
        .unwrap();
    let values: Vec<Option<String>> = redis::cmd("MGET")
        .arg("resp3:k")
        .arg("resp3:missing")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(values, vec![Some("v".to_owned()), None]);

    drop(connection);
    drop(shutdown);
    serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1; proves AUTH-required startup"]
async fn client_matrix_runs_auth_required_connection_scenario() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping Redis client matrix; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }

    let (shutdown, addr, serving) = spawn_auth_resp_facade().await;
    let unauthenticated = redis::Client::open(format!("redis://{addr}/")).unwrap();
    let mut unauthenticated = unauthenticated
        .get_multiplexed_async_connection()
        .await
        .unwrap();
    let error = redis::cmd("GET")
        .arg("auth:k")
        .query_async::<Option<String>>(&mut unauthenticated)
        .await
        .unwrap_err();
    assert!(
        error.to_string().contains("NOAUTH"),
        "expected NOAUTH before AUTH, got {error}"
    );
    drop(unauthenticated);

    let client = redis::Client::open(redis_auth_url(addr)).unwrap();
    let mut connection = client.get_multiplexed_async_connection().await.unwrap();
    let pong: String = redis::cmd("PING")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(pong, "PONG");
    let _: () = redis::cmd("SET")
        .arg("auth:k")
        .arg("v")
        .query_async(&mut connection)
        .await
        .unwrap();
    let value: String = redis::cmd("GET")
        .arg("auth:k")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(value, "v");

    drop(connection);
    drop(shutdown);
    serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1; proves rediss:// startup"]
async fn client_matrix_runs_rediss_required_connection_scenario() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping Redis client matrix; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }

    let (shutdown, addr, serving) = spawn_rediss_auth_resp_facade().await;
    let client = redis::Client::open(redis_auth_rediss_insecure_url(addr)).unwrap();
    let mut connection = client.get_multiplexed_async_connection().await.unwrap();
    let pong: String = redis::cmd("PING")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(pong, "PONG");
    let _: () = redis::cmd("SET")
        .arg("rediss:k")
        .arg("v")
        .query_async(&mut connection)
        .await
        .unwrap();
    let _: () = redis::cmd("MSET")
        .arg("rediss:a")
        .arg("1")
        .arg("rediss:b")
        .arg("2")
        .query_async(&mut connection)
        .await
        .unwrap();
    let _: () = redis::cmd("SET")
        .arg("rediss:ttl")
        .arg("v")
        .arg("EX")
        .arg(30)
        .query_async(&mut connection)
        .await
        .unwrap();
    let ttl: i64 = redis::cmd("TTL")
        .arg("rediss:ttl")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert!(ttl > 0);
    let value: String = redis::cmd("GET")
        .arg("rediss:k")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(value, "v");

    drop(connection);
    drop(shutdown);
    serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1; proves HydraCache HC tag extensions through redis-rs"]
async fn client_matrix_runs_hydracache_tag_extension_scenario() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping Redis client matrix; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }

    let (shutdown, addr, serving) = spawn_resp_facade().await;
    let client = redis::Client::open(format!("redis://{addr}/")).unwrap();
    let mut connection = client.get_multiplexed_async_connection().await.unwrap();

    run_redis_rs_hc_tag_scenario(&mut connection, "rust-hc")
        .await
        .unwrap();

    drop(connection);
    drop(shutdown);
    serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 and optional Python/Node/Go/JVM client runtimes"]
async fn nightly_python_node_go_jvm_clients_bootstrap_and_run_supported_subset() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping Redis client matrix; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }

    let (shutdown, addr, serving) = spawn_auth_resp_facade_for_docker_clients().await;
    let url = redis_auth_url(addr);
    let docker_url = docker_redis_auth_url(addr);
    for ecosystem in ClientEcosystem::all() {
        let url = url.clone();
        let docker_url = docker_url.clone();
        tokio::task::spawn_blocking(move || run_external_client(ecosystem, &url, &docker_url))
            .await
            .unwrap();
    }
    drop(shutdown);
    serving.await.unwrap();
}

#[test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 and Docker-pinned Redis oracle images"]
fn redis_oracle_uses_pinned_redis_versions() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping real Redis oracle; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }

    let manifest = include_str!("../../../docs/integrations/redis_compat_conformance.json");
    for image in PINNED_REDIS_IMAGES {
        assert!(manifest.contains(&format!(r#""{image}""#)));
    }
    assert!(!manifest.contains("redis:latest"));
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 and Docker-pinned Redis oracle images"]
async fn redis_oracle_supported_subset_matches_real_redis() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping real Redis oracle; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }
    let Some(oracle) = RedisOracle::start_first_available().await else {
        return;
    };
    let (shutdown, hydracache_addr, hydracache_serving) = spawn_resp_facade().await;

    let redis_replies = run_supported_subset_scenario(oracle.addr, "oracle").await;
    let hydracache_replies = run_supported_subset_scenario(hydracache_addr, "oracle").await;

    assert_eq!(hydracache_replies, redis_replies);

    drop(shutdown);
    hydracache_serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 and Docker-pinned Redis oracle images"]
async fn redis_oracle_del_exists_counts_match_real_redis() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping real Redis oracle; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }
    let Some(oracle) = RedisOracle::start_first_available().await else {
        return;
    };
    let (shutdown, hydracache_addr, hydracache_serving) = spawn_resp_facade().await;

    let redis_replies = run_count_scenario(oracle.addr, "counts").await;
    let hydracache_replies = run_count_scenario(hydracache_addr, "counts").await;

    assert_eq!(hydracache_replies, redis_replies);

    drop(shutdown);
    hydracache_serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 and Docker-pinned Redis oracle images"]
async fn redis_oracle_mget_nil_and_order_match_real_redis() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping real Redis oracle; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }
    let Some(oracle) = RedisOracle::start_first_available().await else {
        return;
    };
    let (shutdown, hydracache_addr, hydracache_serving) = spawn_resp_facade().await;

    let redis_replies = run_mget_order_scenario(oracle.addr, "mget").await;
    let hydracache_replies = run_mget_order_scenario(hydracache_addr, "mget").await;

    assert_eq!(hydracache_replies, redis_replies);

    drop(shutdown);
    hydracache_serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 and Docker-pinned Redis oracle images"]
async fn redis_oracle_mset_atomicity_matches_real_redis() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping real Redis oracle; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }
    let Some(oracle) = RedisOracle::start_first_available().await else {
        return;
    };
    let (shutdown, hydracache_addr, hydracache_serving) = spawn_resp_facade().await;

    let redis_replies = run_mset_scenario(oracle.addr, "mset").await;
    let hydracache_replies = run_mset_scenario(hydracache_addr, "mset").await;

    assert_eq!(hydracache_replies, redis_replies);

    drop(shutdown);
    hydracache_serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 and Docker-pinned Redis oracle images"]
async fn redis_oracle_ttl_matches_real_redis_with_bounded_tolerance() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping real Redis oracle; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }
    let Some(oracle) = RedisOracle::start_first_available().await else {
        return;
    };
    let (shutdown, hydracache_addr, hydracache_serving) = spawn_resp_facade().await;

    assert_ttl_scenario_shape(run_ttl_scenario(oracle.addr, "ttl-oracle").await, "redis");
    assert_ttl_scenario_shape(
        run_ttl_scenario(hydracache_addr, "ttl-oracle").await,
        "hydracache",
    );

    drop(shutdown);
    hydracache_serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 and Docker-pinned Redis oracle images"]
async fn redis_oracle_ttl_edge_cases_match_real_redis() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping real Redis oracle; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }
    let Some(oracle) = RedisOracle::start_first_available().await else {
        return;
    };
    let (shutdown, hydracache_addr, hydracache_serving) = spawn_resp_facade().await;

    let redis_replies = run_ttl_edge_scenario(oracle.addr, "ttl-edge").await;
    let hydracache_replies = run_ttl_edge_scenario(hydracache_addr, "ttl-edge").await;

    assert_eq!(hydracache_replies, redis_replies);

    drop(shutdown);
    hydracache_serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 and Docker-pinned Redis oracle images"]
async fn redis_oracle_set_nx_px_lock_acquire_matches_real_redis() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping real Redis oracle; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }
    let Some(oracle) = RedisOracle::start_first_available().await else {
        return;
    };
    let (shutdown, hydracache_addr, hydracache_serving) = spawn_resp_facade().await;

    let redis_replies = run_lock_acquire_scenario(oracle.addr, "lock-acquire").await;
    let hydracache_replies = run_lock_acquire_scenario(hydracache_addr, "lock-acquire").await;
    assert_eq!(hydracache_replies, redis_replies);

    drop(shutdown);
    hydracache_serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 and Docker-pinned Redis oracle images"]
async fn redis_oracle_lock_unlock_script_matches_real_redis() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping real Redis oracle; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }
    let Some(oracle) = RedisOracle::start_first_available().await else {
        return;
    };
    let (shutdown, hydracache_addr, hydracache_serving) = spawn_resp_facade().await;

    let redis_replies = run_lock_release_scenario(oracle.addr, "lock-release").await;
    let hydracache_replies = run_lock_release_scenario(hydracache_addr, "lock-release").await;
    assert_eq!(hydracache_replies, redis_replies);

    drop(shutdown);
    hydracache_serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 and Docker-pinned Redis oracle images"]
async fn redis_oracle_lock_extend_script_matches_real_redis_with_ttl_tolerance() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping real Redis oracle; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }
    let Some(oracle) = RedisOracle::start_first_available().await else {
        return;
    };
    let (shutdown, hydracache_addr, hydracache_serving) = spawn_resp_facade().await;

    assert_lock_extend_shape(
        run_lock_extend_scenario(oracle.addr, "lock-extend").await,
        "redis",
    );
    assert_lock_extend_shape(
        run_lock_extend_scenario(hydracache_addr, "lock-extend").await,
        "hydracache",
    );
    assert_redis_py_lock_extend_shape(
        run_redis_py_lock_extend_scenario(oracle.addr, "redis-py-lock-extend").await,
        "redis",
    );
    assert_redis_py_lock_extend_shape(
        run_redis_py_lock_extend_scenario(hydracache_addr, "redis-py-lock-extend").await,
        "hydracache",
    );

    drop(shutdown);
    hydracache_serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 and Docker-pinned Redis oracle images"]
async fn redis_oracle_set_options_are_documented_divergence() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping real Redis oracle; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }
    let Some(oracle) = RedisOracle::start_first_available().await else {
        return;
    };
    let (shutdown, hydracache_addr, hydracache_serving) = spawn_resp_facade().await;

    let redis_seed = query_reply(oracle.addr, "SET", &["set-options:k", "old"]).await;
    let redis_reply = query_reply(oracle.addr, "SET", &["set-options:k", "new", "XX", "GET"]).await;
    let redis_get = query_reply(oracle.addr, "GET", &["set-options:k"]).await;
    let hydracache_seed = query_reply(hydracache_addr, "SET", &["set-options:k", "old"]).await;
    let hydracache_reply = query_reply(
        hydracache_addr,
        "SET",
        &["set-options:k", "new", "XX", "GET"],
    )
    .await;
    let hydracache_get = query_reply(hydracache_addr, "GET", &["set-options:k"]).await;

    assert_eq!(redis_seed, OracleReply::Status("OK".to_owned()));
    assert_eq!(hydracache_seed, OracleReply::Status("OK".to_owned()));
    assert_eq!(redis_reply, OracleReply::Bulk(b"old".to_vec()));
    assert_eq!(redis_get, OracleReply::Bulk(b"new".to_vec()));
    assert!(
        matches!(
            hydracache_reply,
            OracleReply::ErrorClass(ref class) if class == "ERR"
        ),
        "HydraCache SET XX GET should normalize to ERR, got {hydracache_reply:?}"
    );
    assert_eq!(hydracache_get, OracleReply::Bulk(b"old".to_vec()));

    drop(shutdown);
    hydracache_serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 and Docker-pinned Redis oracle images"]
async fn redis_oracle_unsupported_divergence_is_documented() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping real Redis oracle; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }
    let Some(oracle) = RedisOracle::start_first_available().await else {
        return;
    };
    let (shutdown, hydracache_addr, hydracache_serving) = spawn_resp_facade().await;

    let redis_reply = query_reply(oracle.addr, "HSET", &["hash", "field", "value"]).await;
    let hydracache_reply = query_reply(hydracache_addr, "HSET", &["hash", "field", "value"]).await;

    assert_eq!(redis_reply, OracleReply::Int(1));
    assert!(
        matches!(
            hydracache_reply,
            OracleReply::ErrorClass(ref class) if class == "ERR"
        ),
        "HydraCache unsupported HSET should normalize to ERR, got {hydracache_reply:?}"
    );

    drop(shutdown);
    hydracache_serving.await.unwrap();
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 and Docker-pinned Redis oracle images"]
async fn redis_oracle_hc_extensions_are_hydracache_only() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping real Redis oracle; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }
    let Some(oracle) = RedisOracle::start_first_available().await else {
        return;
    };
    let (shutdown, hydracache_addr, hydracache_serving) = spawn_resp_facade().await;

    let redis_reply = query_reply(oracle.addr, "HC.STATS", &[]).await;
    let hydracache_reply = query_reply(hydracache_addr, "HC.STATS", &[]).await;
    let redis_tag_reply = query_reply(oracle.addr, "HC.TAG", &["hc:key", "model"]).await;
    let hydracache_set = query_reply(hydracache_addr, "SET", &["hc:key", "v"]).await;
    let hydracache_tag_reply = query_reply(hydracache_addr, "HC.TAG", &["hc:key", "model"]).await;
    let hydracache_invalidate_tag_reply =
        query_reply(hydracache_addr, "HC.INVALIDATE_TAG", &["model"]).await;
    let hydracache_get_after_tag_invalidate =
        query_reply(hydracache_addr, "GET", &["hc:key"]).await;

    assert!(
        matches!(
            redis_reply,
            OracleReply::ErrorClass(ref class) if class == "ERR"
        ),
        "real Redis HC.STATS should normalize to ERR, got {redis_reply:?}"
    );
    assert!(matches!(hydracache_reply, OracleReply::Array(_)));
    assert!(
        matches!(
            redis_tag_reply,
            OracleReply::ErrorClass(ref class) if class == "ERR"
        ),
        "real Redis HC.TAG should normalize to ERR, got {redis_tag_reply:?}"
    );
    assert_eq!(hydracache_set, OracleReply::Status("OK".to_owned()));
    assert_eq!(hydracache_tag_reply, OracleReply::Int(1));
    assert_eq!(hydracache_invalidate_tag_reply, OracleReply::Int(1));
    assert_eq!(hydracache_get_after_tag_invalidate, OracleReply::Nil);

    drop(shutdown);
    hydracache_serving.await.unwrap();
}

fn env_gate_enabled(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

#[derive(Debug, Clone, Copy)]
enum ClientEcosystem {
    Python,
    Node,
    Go,
    Jvm,
}

impl ClientEcosystem {
    fn all() -> [Self; 4] {
        [Self::Python, Self::Node, Self::Go, Self::Jvm]
    }

    fn label(self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::Node => "node",
            Self::Go => "go",
            Self::Jvm => "jvm",
        }
    }

    fn require_env(self) -> &'static str {
        match self {
            Self::Python => "HYDRACACHE_REQUIRE_REDIS_CLIENT_PYTHON",
            Self::Node => "HYDRACACHE_REQUIRE_REDIS_CLIENT_NODE",
            Self::Go => "HYDRACACHE_REQUIRE_REDIS_CLIENT_GO",
            Self::Jvm => "HYDRACACHE_REQUIRE_REDIS_CLIENT_JVM",
        }
    }

    fn has_docker_fallback(self) -> bool {
        matches!(self, Self::Python | Self::Node | Self::Jvm)
    }
}

fn run_external_client(ecosystem: ClientEcosystem, redis_url: &str, docker_redis_url: &str) {
    let result = if env_gate_enabled(CLIENT_DOCKER_FORCE_ENV) && ecosystem.has_docker_fallback() {
        run_docker_client(ecosystem, docker_redis_url)
    } else {
        run_external_client_with_local_fallback(ecosystem, redis_url, docker_redis_url)
    };
    match result {
        ClientRun::Passed => {}
        ClientRun::Skipped(reason) if !env_gate_enabled(ecosystem.require_env()) => {
            eprintln!(
                "skipping {} Redis client matrix row: {reason}",
                ecosystem.label()
            );
        }
        ClientRun::Skipped(reason) => panic!(
            "{} Redis client matrix row was required but skipped: {reason}",
            ecosystem.label()
        ),
        ClientRun::Failed(output) => panic!(
            "{} Redis client matrix row failed:\n{}",
            ecosystem.label(),
            output
        ),
    }
}

async fn run_redis_rs_mset_ttl_scenario(
    connection: &mut redis::aio::MultiplexedConnection,
    prefix: &str,
) -> redis::RedisResult<()> {
    let a = format!("{prefix}:a");
    let b = format!("{prefix}:b");
    let ttl_key = format!("{prefix}:ttl");

    redis::cmd("MSET")
        .arg(&a)
        .arg("1")
        .arg(&b)
        .arg("2")
        .query_async::<()>(connection)
        .await?;
    let values: Vec<Option<String>> = redis::cmd("MGET")
        .arg(&a)
        .arg(&b)
        .query_async(connection)
        .await?;
    assert_eq!(values, vec![Some("1".to_owned()), Some("2".to_owned())]);

    redis::cmd("SET")
        .arg(&ttl_key)
        .arg("v")
        .arg("PX")
        .arg(5_000)
        .query_async::<()>(connection)
        .await?;
    let pttl: i64 = redis::cmd("PTTL")
        .arg(&ttl_key)
        .query_async(connection)
        .await?;
    assert!(
        (1..=5_000).contains(&pttl),
        "PTTL should be positive and bounded, got {pttl}"
    );
    let persisted: i64 = redis::cmd("PERSIST")
        .arg(&ttl_key)
        .query_async(connection)
        .await?;
    assert_eq!(persisted, 1);
    let ttl: i64 = redis::cmd("TTL")
        .arg(&ttl_key)
        .query_async(connection)
        .await?;
    assert_eq!(ttl, -1);

    Ok(())
}

async fn run_redis_rs_lock_scenario(
    connection: &mut redis::aio::MultiplexedConnection,
    prefix: &str,
) -> redis::RedisResult<()> {
    let key = format!("{prefix}:lock");
    let first: Option<String> = redis::cmd("SET")
        .arg(&key)
        .arg("owner")
        .arg("NX")
        .arg("PX")
        .arg(5_000)
        .query_async(connection)
        .await?;
    assert_eq!(first.as_deref(), Some("OK"));

    let contender: Option<String> = redis::cmd("SET")
        .arg(&key)
        .arg("contender")
        .arg("NX")
        .arg("PX")
        .arg(5_000)
        .query_async(connection)
        .await?;
    assert_eq!(contender, None);

    let current: String = redis::cmd("GET").arg(&key).query_async(connection).await?;
    assert_eq!(current, "owner");

    let wrong_release: i64 = redis::cmd("EVAL")
        .arg(LOCK_RELEASE_SCRIPT)
        .arg(1)
        .arg(&key)
        .arg("contender")
        .query_async(connection)
        .await?;
    assert_eq!(wrong_release, 0);

    let extended: i64 = redis::cmd("EVAL")
        .arg(LOCK_EXTEND_SCRIPT)
        .arg(1)
        .arg(&key)
        .arg("owner")
        .arg(5_000)
        .query_async(connection)
        .await?;
    assert_eq!(extended, 1);

    let released: i64 = redis::cmd("EVAL")
        .arg(LOCK_RELEASE_SCRIPT)
        .arg(1)
        .arg(&key)
        .arg("owner")
        .query_async(connection)
        .await?;
    assert_eq!(released, 1);

    let after_release: Option<String> = redis::cmd("GET").arg(&key).query_async(connection).await?;
    assert_eq!(after_release, None);

    Ok(())
}

async fn run_redis_rs_hc_tag_scenario(
    connection: &mut redis::aio::MultiplexedConnection,
    prefix: &str,
) -> redis::RedisResult<()> {
    let first = format!("{prefix}:tagged:1");
    let second = format!("{prefix}:tagged:2");
    let keep = format!("{prefix}:untagged");

    let namespace: String = redis::cmd("HC.NAMESPACE").query_async(connection).await?;
    assert_eq!(namespace, DEFAULT_REDIS_NAMESPACE);
    let selected: String = redis::cmd("HC.NAMESPACE")
        .arg(DEFAULT_REDIS_NAMESPACE)
        .query_async(connection)
        .await?;
    assert_eq!(selected, "OK");

    redis::cmd("SET")
        .arg(&first)
        .arg("v1")
        .query_async::<()>(connection)
        .await?;
    redis::cmd("SET")
        .arg(&second)
        .arg("v2")
        .query_async::<()>(connection)
        .await?;
    redis::cmd("SET")
        .arg(&keep)
        .arg("keep")
        .query_async::<()>(connection)
        .await?;

    let added: i64 = redis::cmd("HC.TAG")
        .arg(&first)
        .arg("model")
        .arg("shared")
        .query_async(connection)
        .await?;
    assert_eq!(added, 2);
    let duplicate: i64 = redis::cmd("HC.TAG")
        .arg(&first)
        .arg("model")
        .query_async(connection)
        .await?;
    assert_eq!(duplicate, 0);
    let replaced: i64 = redis::cmd("HC.SETTAGS")
        .arg(&second)
        .arg("model")
        .arg("model")
        .query_async(connection)
        .await?;
    assert_eq!(replaced, 1);

    let invalidated: i64 = redis::cmd("HC.INVALIDATE_TAG")
        .arg("model")
        .query_async(connection)
        .await?;
    assert_eq!(invalidated, 2);
    let values: Vec<Option<String>> = redis::cmd("MGET")
        .arg(&first)
        .arg(&second)
        .arg(&keep)
        .query_async(connection)
        .await?;
    assert_eq!(values, vec![None, None, Some("keep".to_owned())]);
    let invalidated_again: i64 = redis::cmd("HC.INVALIDATE_TAG")
        .arg("model")
        .query_async(connection)
        .await?;
    assert_eq!(invalidated_again, 0);

    Ok(())
}

fn run_external_client_with_local_fallback(
    ecosystem: ClientEcosystem,
    redis_url: &str,
    docker_redis_url: &str,
) -> ClientRun {
    let local_result = match ecosystem {
        ClientEcosystem::Python => run_python_client(redis_url),
        ClientEcosystem::Node => run_node_client(redis_url),
        ClientEcosystem::Go => run_go_client(redis_url),
        ClientEcosystem::Jvm => run_jvm_client(redis_url),
    };
    match local_result {
        ClientRun::Passed | ClientRun::Failed(_) => local_result,
        ClientRun::Skipped(local_reason) => match run_docker_client(ecosystem, docker_redis_url) {
            ClientRun::Passed => ClientRun::Passed,
            ClientRun::Skipped(docker_reason) => ClientRun::Skipped(format!(
                "{local_reason}; Docker fallback skipped: {docker_reason}"
            )),
            ClientRun::Failed(output) => ClientRun::Failed(format!(
                "local row skipped: {local_reason}\nDocker fallback failed:\n{output}"
            )),
        },
    }
}

enum ClientRun {
    Passed,
    Skipped(String),
    Failed(String),
}

fn run_docker_client(ecosystem: ClientEcosystem, redis_url: &str) -> ClientRun {
    if !docker_available() {
        return ClientRun::Skipped("docker CLI is not available".to_owned());
    }
    match ecosystem {
        ClientEcosystem::Python => run_python_client_docker(redis_url),
        ClientEcosystem::Node => run_node_client_docker(redis_url),
        ClientEcosystem::Go => {
            ClientRun::Skipped("Go client uses the local Go toolchain".to_owned())
        }
        ClientEcosystem::Jvm => run_jvm_client_docker(redis_url),
    }
}

fn run_python_client(redis_url: &str) -> ClientRun {
    let script = r#"
import sys
try:
    import redis
except Exception as exc:
    print(f"missing python redis client: {exc}")
    sys.exit(42)
if redis.__version__ != "5.2.1":
    print(f"unsupported python redis client version: {redis.__version__}")
    sys.exit(42)
r = redis.Redis.from_url(sys.argv[1], decode_responses=True)
assert r.ping() is True
info = r.info()
assert info["redis_mode"] == "standalone"
assert info["hydracache_resp"] == "RESP2+RESP3"
assert "used_memory" not in info
assert "db0" not in info
assert r.set("python:k", "v") is True
assert r.get("python:k") == "v"
assert r.type("python:k") == "string"
assert r.type("python:missing") == "none"
assert r.mset({"python:a": "1", "python:b": "2"}) is True
assert r.mget(["python:a", "python:b"]) == ["1", "2"]
assert r.set("python:ttl", "v", px=5000) is True
pttl = r.pttl("python:ttl")
assert 0 < pttl <= 5000
assert r.persist("python:ttl") is True
assert r.ttl("python:ttl") == -1
rb = redis.Redis.from_url(sys.argv[1], decode_responses=False)
lock = rb.lock("python:lock", timeout=5, blocking_timeout=0.1)
assert lock.acquire(blocking=False) is True
contender = rb.lock("python:lock", timeout=5, blocking_timeout=0.1)
assert contender.acquire(blocking=False) is False
before_extend = rb.pttl("python:lock")
assert 0 < before_extend <= 5000
assert lock.extend(5) is True
after_extend = rb.pttl("python:lock")
assert after_extend > 5000
lock.release()
assert rb.get("python:lock") is None
assert r.mget(["python:k", "python:missing"]) == ["v", None]
assert r.exists("python:k", "python:missing") == 1
assert r.delete("python:k", "python:missing") == 1
assert r.execute_command("HC.NAMESPACE") == "redis"
assert r.execute_command("HC.NAMESPACE", "redis") == "OK"
assert r.set("python:hc:a", "1") is True
assert r.set("python:hc:b", "2") is True
assert r.set("python:hc:keep", "keep") is True
assert r.execute_command("HC.TAG", "python:hc:a", "model", "shared") == 2
assert r.execute_command("HC.TAG", "python:hc:a", "model") == 0
assert r.execute_command("HC.SETTAGS", "python:hc:b", "model", "model") == 1
assert r.execute_command("HC.INVALIDATE_TAG", "model") == 2
assert r.mget(["python:hc:a", "python:hc:b", "python:hc:keep"]) == [None, None, "keep"]
assert r.execute_command("HC.INVALIDATE_TAG", "model") == 0
"#;
    run_optional_command(
        "python",
        Command::new("python").arg("-c").arg(script).arg(redis_url),
    )
}

fn run_python_client_docker(redis_url: &str) -> ClientRun {
    let script = r#"
import os
import redis
assert redis.__version__ == "5.2.1"
r = redis.Redis.from_url(os.environ["REDIS_URL"], decode_responses=True)
assert r.ping() is True
info = r.info()
assert info["redis_mode"] == "standalone"
assert info["hydracache_resp"] == "RESP2+RESP3"
assert "used_memory" not in info
assert "db0" not in info
assert r.set("python:k", "v") is True
assert r.get("python:k") == "v"
assert r.type("python:k") == "string"
assert r.type("python:missing") == "none"
assert r.mset({"python:a": "1", "python:b": "2"}) is True
assert r.mget(["python:a", "python:b"]) == ["1", "2"]
assert r.set("python:ttl", "v", px=5000) is True
pttl = r.pttl("python:ttl")
assert 0 < pttl <= 5000
assert r.persist("python:ttl") is True
assert r.ttl("python:ttl") == -1
rb = redis.Redis.from_url(os.environ["REDIS_URL"], decode_responses=False)
lock = rb.lock("python:lock", timeout=5, blocking_timeout=0.1)
assert lock.acquire(blocking=False) is True
contender = rb.lock("python:lock", timeout=5, blocking_timeout=0.1)
assert contender.acquire(blocking=False) is False
before_extend = rb.pttl("python:lock")
assert 0 < before_extend <= 5000
assert lock.extend(5) is True
after_extend = rb.pttl("python:lock")
assert after_extend > 5000
lock.release()
assert rb.get("python:lock") is None
assert r.mget(["python:k", "python:missing"]) == ["v", None]
assert r.exists("python:k", "python:missing") == 1
assert r.delete("python:k", "python:missing") == 1
assert r.execute_command("HC.NAMESPACE") == "redis"
assert r.execute_command("HC.NAMESPACE", "redis") == "OK"
assert r.set("python:hc:a", "1") is True
assert r.set("python:hc:b", "2") is True
assert r.set("python:hc:keep", "keep") is True
assert r.execute_command("HC.TAG", "python:hc:a", "model", "shared") == 2
assert r.execute_command("HC.TAG", "python:hc:a", "model") == 0
assert r.execute_command("HC.SETTAGS", "python:hc:b", "model", "model") == 1
assert r.execute_command("HC.INVALIDATE_TAG", "model") == 2
assert r.mget(["python:hc:a", "python:hc:b", "python:hc:keep"]) == [None, None, "keep"]
assert r.execute_command("HC.INVALIDATE_TAG", "model") == 0
"#;
    let mut command = docker_client_command(redis_url);
    command
        .arg(PYTHON_CLIENT_DOCKER_IMAGE)
        .arg("sh")
        .arg("-lc")
        .arg(format!(
        "python -m pip install --quiet --disable-pip-version-check {PYTHON_CLIENT_DOCKER_PACKAGE} && python - <<'PY'\n{script}\nPY"
    ));
    run_optional_command("docker redis-py", &mut command)
}

fn run_node_client(redis_url: &str) -> ClientRun {
    let script = r#"
(async () => {
  let redis;
  try {
    redis = require("redis");
  } catch (error) {
    console.log(`missing node redis client: ${error}`);
    process.exit(42);
  }
  let Redlock;
  try {
    Redlock = require("redlock").default;
  } catch (error) {
    console.log(`missing node redlock client: ${error}`);
    process.exit(42);
  }
  function attachRedlockNodeRedisAdapter(client) {
    const evalCommand = client.eval.bind(client);
    const evalShaCommand = client.evalSha.bind(client);
    const split = (numKeys, args) => ({
      keys: args.slice(0, numKeys),
      arguments: args.slice(numKeys).map(String)
    });
    client.eval = (script, numKeys, args) => evalCommand(script, split(numKeys, args));
    client.evalsha = (sha, numKeys, args) => evalShaCommand(sha, split(numKeys, args));
  }
  const client = redis.createClient({ url: process.argv[1] });
  await client.connect();
  attachRedlockNodeRedisAdapter(client);
  if (await client.ping() !== "PONG") throw new Error("PING failed");
  const info = await client.sendCommand(["INFO"]);
  if (!info.includes("redis_mode:standalone")) throw new Error("INFO missing standalone mode");
  if (!info.includes("hydracache_resp:RESP2+RESP3")) throw new Error("INFO missing RESP dialects");
  if (info.includes("used_memory") || info.includes("db0:")) throw new Error("INFO fabricated Redis server state");
  if (await client.set("node:k", "v") !== "OK") throw new Error("SET failed");
  if (await client.get("node:k") !== "v") throw new Error("GET failed");
  if (await client.sendCommand(["TYPE", "node:k"]) !== "string") throw new Error("TYPE existing failed");
  if (await client.sendCommand(["TYPE", "node:missing"]) !== "none") throw new Error("TYPE missing failed");
  if (await client.sendCommand(["MSET", "node:a", "1", "node:b", "2"]) !== "OK") throw new Error("MSET failed");
  const msetValues = await client.mGet(["node:a", "node:b"]);
  if (JSON.stringify(msetValues) !== JSON.stringify(["1", "2"])) throw new Error(`MSET/MGET failed: ${JSON.stringify(msetValues)}`);
  if (await client.sendCommand(["SET", "node:ttl", "v", "PX", "5000"]) !== "OK") throw new Error("SET PX failed");
  const pttl = Number(await client.sendCommand(["PTTL", "node:ttl"]));
  if (!(pttl > 0 && pttl <= 5000)) throw new Error(`PTTL out of range: ${pttl}`);
  if (Number(await client.sendCommand(["PERSIST", "node:ttl"])) !== 1) throw new Error("PERSIST failed");
  if (Number(await client.sendCommand(["TTL", "node:ttl"])) !== -1) throw new Error("TTL after PERSIST failed");
  const redlock = new Redlock([client], { retryCount: 0, retryDelay: 1, retryJitter: 0 });
  let lock = await redlock.acquire(["node:lock"], 5000);
  let contended = false;
  try {
    await redlock.acquire(["node:lock"], 5000);
  } catch (error) {
    contended = true;
  }
  if (!contended) throw new Error("redlock contention unexpectedly acquired");
  lock = await lock.extend(5000);
  await lock.release();
  if (await client.get("node:lock") !== null) throw new Error("redlock release left value behind");
  const values = await client.mGet(["node:k", "node:missing"]);
  if (JSON.stringify(values) !== JSON.stringify(["v", null])) throw new Error(`MGET failed: ${JSON.stringify(values)}`);
  if (await client.exists(["node:k", "node:missing"]) !== 1) throw new Error("EXISTS failed");
  if (await client.del(["node:k", "node:missing"]) !== 1) throw new Error("DEL failed");
  if (await client.sendCommand(["HC.NAMESPACE"]) !== "redis") throw new Error("HC.NAMESPACE failed");
  if (await client.sendCommand(["HC.NAMESPACE", "redis"]) !== "OK") throw new Error("HC.NAMESPACE select failed");
  if (await client.set("node:hc:a", "1") !== "OK") throw new Error("HC SET a failed");
  if (await client.set("node:hc:b", "2") !== "OK") throw new Error("HC SET b failed");
  if (await client.set("node:hc:keep", "keep") !== "OK") throw new Error("HC SET keep failed");
  if (Number(await client.sendCommand(["HC.TAG", "node:hc:a", "model", "shared"])) !== 2) throw new Error("HC.TAG failed");
  if (Number(await client.sendCommand(["HC.TAG", "node:hc:a", "model"])) !== 0) throw new Error("HC.TAG duplicate failed");
  if (Number(await client.sendCommand(["HC.SETTAGS", "node:hc:b", "model", "model"])) !== 1) throw new Error("HC.SETTAGS failed");
  if (Number(await client.sendCommand(["HC.INVALIDATE_TAG", "model"])) !== 2) throw new Error("HC.INVALIDATE_TAG failed");
  const hcValues = await client.mGet(["node:hc:a", "node:hc:b", "node:hc:keep"]);
  if (JSON.stringify(hcValues) !== JSON.stringify([null, null, "keep"])) throw new Error(`HC invalidate failed: ${JSON.stringify(hcValues)}`);
  if (Number(await client.sendCommand(["HC.INVALIDATE_TAG", "model"])) !== 0) throw new Error("HC.INVALIDATE_TAG repeat failed");
  await client.quit();
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
"#;
    run_optional_command(
        "node",
        Command::new("node").arg("-e").arg(script).arg(redis_url),
    )
}

fn run_node_client_docker(redis_url: &str) -> ClientRun {
    let script = r#"
(async () => {
  const redis = require("redis");
  const Redlock = require("redlock").default;
  function attachRedlockNodeRedisAdapter(client) {
    const evalCommand = client.eval.bind(client);
    const evalShaCommand = client.evalSha.bind(client);
    const split = (numKeys, args) => ({
      keys: args.slice(0, numKeys),
      arguments: args.slice(numKeys).map(String)
    });
    client.eval = (script, numKeys, args) => evalCommand(script, split(numKeys, args));
    client.evalsha = (sha, numKeys, args) => evalShaCommand(sha, split(numKeys, args));
  }
  const client = redis.createClient({
    url: process.env.REDIS_URL,
    socket: { reconnectStrategy: false }
  });
  await client.connect();
  attachRedlockNodeRedisAdapter(client);
  if (await client.ping() !== "PONG") throw new Error("PING failed");
  const info = await client.sendCommand(["INFO"]);
  if (!info.includes("redis_mode:standalone")) throw new Error("INFO missing standalone mode");
  if (!info.includes("hydracache_resp:RESP2+RESP3")) throw new Error("INFO missing RESP dialects");
  if (info.includes("used_memory") || info.includes("db0:")) throw new Error("INFO fabricated Redis server state");
  if (await client.set("node:k", "v") !== "OK") throw new Error("SET failed");
  if (await client.get("node:k") !== "v") throw new Error("GET failed");
  if (await client.sendCommand(["TYPE", "node:k"]) !== "string") throw new Error("TYPE existing failed");
  if (await client.sendCommand(["TYPE", "node:missing"]) !== "none") throw new Error("TYPE missing failed");
  if (await client.sendCommand(["MSET", "node:a", "1", "node:b", "2"]) !== "OK") throw new Error("MSET failed");
  const msetValues = await client.mGet(["node:a", "node:b"]);
  if (JSON.stringify(msetValues) !== JSON.stringify(["1", "2"])) throw new Error(`MSET/MGET failed: ${JSON.stringify(msetValues)}`);
  if (await client.sendCommand(["SET", "node:ttl", "v", "PX", "5000"]) !== "OK") throw new Error("SET PX failed");
  const pttl = Number(await client.sendCommand(["PTTL", "node:ttl"]));
  if (!(pttl > 0 && pttl <= 5000)) throw new Error(`PTTL out of range: ${pttl}`);
  if (Number(await client.sendCommand(["PERSIST", "node:ttl"])) !== 1) throw new Error("PERSIST failed");
  if (Number(await client.sendCommand(["TTL", "node:ttl"])) !== -1) throw new Error("TTL after PERSIST failed");
  const redlock = new Redlock([client], { retryCount: 0, retryDelay: 1, retryJitter: 0 });
  let lock = await redlock.acquire(["node:lock"], 5000);
  let contended = false;
  try {
    await redlock.acquire(["node:lock"], 5000);
  } catch (error) {
    contended = true;
  }
  if (!contended) throw new Error("redlock contention unexpectedly acquired");
  lock = await lock.extend(5000);
  await lock.release();
  if (await client.get("node:lock") !== null) throw new Error("redlock release left value behind");
  const values = await client.mGet(["node:k", "node:missing"]);
  if (JSON.stringify(values) !== JSON.stringify(["v", null])) throw new Error(`MGET failed: ${JSON.stringify(values)}`);
  if (await client.exists(["node:k", "node:missing"]) !== 1) throw new Error("EXISTS failed");
  if (await client.del(["node:k", "node:missing"]) !== 1) throw new Error("DEL failed");
  if (await client.sendCommand(["HC.NAMESPACE"]) !== "redis") throw new Error("HC.NAMESPACE failed");
  if (await client.sendCommand(["HC.NAMESPACE", "redis"]) !== "OK") throw new Error("HC.NAMESPACE select failed");
  if (await client.set("node:hc:a", "1") !== "OK") throw new Error("HC SET a failed");
  if (await client.set("node:hc:b", "2") !== "OK") throw new Error("HC SET b failed");
  if (await client.set("node:hc:keep", "keep") !== "OK") throw new Error("HC SET keep failed");
  if (Number(await client.sendCommand(["HC.TAG", "node:hc:a", "model", "shared"])) !== 2) throw new Error("HC.TAG failed");
  if (Number(await client.sendCommand(["HC.TAG", "node:hc:a", "model"])) !== 0) throw new Error("HC.TAG duplicate failed");
  if (Number(await client.sendCommand(["HC.SETTAGS", "node:hc:b", "model", "model"])) !== 1) throw new Error("HC.SETTAGS failed");
  if (Number(await client.sendCommand(["HC.INVALIDATE_TAG", "model"])) !== 2) throw new Error("HC.INVALIDATE_TAG failed");
  const hcValues = await client.mGet(["node:hc:a", "node:hc:b", "node:hc:keep"]);
  if (JSON.stringify(hcValues) !== JSON.stringify([null, null, "keep"])) throw new Error(`HC invalidate failed: ${JSON.stringify(hcValues)}`);
  if (Number(await client.sendCommand(["HC.INVALIDATE_TAG", "model"])) !== 0) throw new Error("HC.INVALIDATE_TAG repeat failed");
  await client.quit();
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
"#;
    let mut command = docker_client_command(redis_url);
    command
        .arg(NODE_CLIENT_DOCKER_IMAGE)
        .arg("sh")
        .arg("-lc")
        .arg(format!(
        "mkdir -p /tmp/hydracache-redis-client && cd /tmp/hydracache-redis-client && npm init -y >/dev/null 2>&1 && npm install --no-audit --no-fund --silent {NODE_CLIENT_DOCKER_PACKAGE} >/dev/null && node - <<'NODE'\n{script}\nNODE"
    ));
    run_optional_command("docker node-redis", &mut command)
}

fn run_go_client(redis_url: &str) -> ClientRun {
    let Ok(dir) = prepare_go_client(redis_url) else {
        return ClientRun::Skipped("could not prepare temporary Go module".to_owned());
    };
    let mut tidy = Command::new("go");
    tidy.arg("mod").arg("tidy").current_dir(&dir);
    if let ClientRun::Failed(output) = run_optional_command("go mod tidy", &mut tidy) {
        let _ = fs::remove_dir_all(&dir);
        return ClientRun::Failed(output);
    }
    let mut command = Command::new("go");
    command.arg("run").arg(".").current_dir(&dir);
    let result = run_optional_command("go", &mut command);
    let _ = fs::remove_dir_all(dir);
    result
}

fn prepare_go_client(redis_url: &str) -> Result<std::path::PathBuf, std::io::Error> {
    let dir = unique_temp_dir("go-client");
    fs::create_dir_all(&dir)?;
    fs::write(
        dir.join("go.mod"),
        "module hydracache-redis-client-smoke\n\ngo 1.22\n\nrequire github.com/redis/go-redis/v9 v9.7.0\n",
    )?;
    fs::write(
        dir.join("main.go"),
        format!(
            r#"
package main

import (
    "context"
    "fmt"
    "os"
    "strings"
    "time"

    redis "github.com/redis/go-redis/v9"
)

func must(ok bool, message string) {{
    if !ok {{
        panic(message)
    }}
}}

func mustNoErr(err error, message string) {{
    if err != nil {{
        panic(fmt.Sprintf("%s: %v", message, err))
    }}
}}

func main() {{
    options, err := redis.ParseURL("{redis_url}")
    if err != nil {{
        panic(err)
    }}
    options.Protocol = 2
    options.MaxRetries = -1
    options.ReadTimeout = 2 * time.Second
    options.WriteTimeout = 2 * time.Second
    client := redis.NewClient(options)
    defer client.Close()
    ctx := context.Background()

    pong, err := client.Ping(ctx).Result()
    mustNoErr(err, "PING failed")
    must(pong == "PONG", fmt.Sprintf("PING got %q", pong))

    info, err := client.Info(ctx).Result()
    mustNoErr(err, "INFO failed")
    must(strings.Contains(info, "redis_mode:standalone"), "INFO missing standalone mode")
    must(strings.Contains(info, "hydracache_resp:RESP2+RESP3"), "INFO missing RESP dialects")
    must(!strings.Contains(info, "used_memory") && !strings.Contains(info, "db0:"), "INFO fabricated Redis server state")

    set, err := client.Set(ctx, "go:k", "v", 0).Result()
    mustNoErr(err, "SET failed")
    must(set == "OK", fmt.Sprintf("SET got %q", set))

    got, err := client.Get(ctx, "go:k").Result()
    mustNoErr(err, "GET failed")
    must(got == "v", fmt.Sprintf("GET got %q", got))

    existingType, err := client.Type(ctx, "go:k").Result()
    mustNoErr(err, "TYPE existing failed")
    must(existingType == "string", fmt.Sprintf("TYPE existing got %q", existingType))
    missingType, err := client.Type(ctx, "go:missing").Result()
    mustNoErr(err, "TYPE missing failed")
    must(missingType == "none", fmt.Sprintf("TYPE missing got %q", missingType))

    mustNoErr(client.MSet(ctx, "go:a", "1", "go:b", "2").Err(), "MSET failed")
    msetValues, err := client.MGet(ctx, "go:a", "go:b").Result()
    mustNoErr(err, "MSET/MGET failed")
    must(len(msetValues) == 2 && msetValues[0] == "1" && msetValues[1] == "2", fmt.Sprintf("MSET/MGET failed: %#v", msetValues))

    setTtl, err := client.Set(ctx, "go:ttl", "v", 5*time.Second).Result()
    mustNoErr(err, "SET PX/EX failed")
    must(setTtl == "OK", fmt.Sprintf("SET TTL got %q", setTtl))
    pttl, err := client.PTTL(ctx, "go:ttl").Result()
    mustNoErr(err, "PTTL failed")
    must(pttl > 0 && pttl <= 5*time.Second, fmt.Sprintf("PTTL out of range: %v", pttl))
    persisted, err := client.Persist(ctx, "go:ttl").Result()
    mustNoErr(err, "PERSIST failed")
    must(persisted, "PERSIST returned false")
    ttl, err := client.TTL(ctx, "go:ttl").Result()
    mustNoErr(err, "TTL after PERSIST failed")
    must(ttl == -1*time.Nanosecond, fmt.Sprintf("TTL after PERSIST got %v", ttl))

    values, err := client.MGet(ctx, "go:k", "go:missing").Result()
    mustNoErr(err, "MGET failed")
    must(len(values) == 2 && values[0] == "v" && values[1] == nil, fmt.Sprintf("MGET failed: %#v", values))

    exists, err := client.Exists(ctx, "go:k", "go:missing").Result()
    mustNoErr(err, "EXISTS failed")
    must(exists == 1, fmt.Sprintf("EXISTS got %d", exists))

    deleted, err := client.Del(ctx, "go:k", "go:missing").Result()
    mustNoErr(err, "DEL failed")
    must(deleted == 1, fmt.Sprintf("DEL got %d", deleted))

    namespace, err := client.Do(ctx, "HC.NAMESPACE").Text()
    mustNoErr(err, "HC.NAMESPACE failed")
    must(namespace == "redis", fmt.Sprintf("HC.NAMESPACE got %q", namespace))
    selected, err := client.Do(ctx, "HC.NAMESPACE", "redis").Text()
    mustNoErr(err, "HC.NAMESPACE select failed")
    must(selected == "OK", fmt.Sprintf("HC.NAMESPACE select got %q", selected))
    mustNoErr(client.Set(ctx, "go:hc:a", "1", 0).Err(), "HC SET a failed")
    mustNoErr(client.Set(ctx, "go:hc:b", "2", 0).Err(), "HC SET b failed")
    mustNoErr(client.Set(ctx, "go:hc:keep", "keep", 0).Err(), "HC SET keep failed")
    added, err := client.Do(ctx, "HC.TAG", "go:hc:a", "model", "shared").Int()
    mustNoErr(err, "HC.TAG failed")
    must(added == 2, fmt.Sprintf("HC.TAG got %d", added))
    duplicate, err := client.Do(ctx, "HC.TAG", "go:hc:a", "model").Int()
    mustNoErr(err, "HC.TAG duplicate failed")
    must(duplicate == 0, fmt.Sprintf("HC.TAG duplicate got %d", duplicate))
    replaced, err := client.Do(ctx, "HC.SETTAGS", "go:hc:b", "model", "model").Int()
    mustNoErr(err, "HC.SETTAGS failed")
    must(replaced == 1, fmt.Sprintf("HC.SETTAGS got %d", replaced))
    invalidated, err := client.Do(ctx, "HC.INVALIDATE_TAG", "model").Int()
    mustNoErr(err, "HC.INVALIDATE_TAG failed")
    must(invalidated == 2, fmt.Sprintf("HC.INVALIDATE_TAG got %d", invalidated))
    hcValues, err := client.MGet(ctx, "go:hc:a", "go:hc:b", "go:hc:keep").Result()
    mustNoErr(err, "HC MGET failed")
    must(len(hcValues) == 3 && hcValues[0] == nil && hcValues[1] == nil && hcValues[2] == "keep", fmt.Sprintf("HC invalidate failed: %#v", hcValues))
    invalidatedAgain, err := client.Do(ctx, "HC.INVALIDATE_TAG", "model").Int()
    mustNoErr(err, "HC.INVALIDATE_TAG repeat failed")
    must(invalidatedAgain == 0, fmt.Sprintf("HC.INVALIDATE_TAG repeat got %d", invalidatedAgain))
    _ = os.Stdout
}}
"#
        ),
    )?;
    Ok(dir)
}

fn run_jvm_client(redis_url: &str) -> ClientRun {
    let Ok(dir) = prepare_jvm_client() else {
        return ClientRun::Skipped("could not prepare temporary JVM module".to_owned());
    };
    let mut command = maven_command();
    command
        .args(["-q", "-f", "pom.xml", "compile", "exec:java"])
        .env("REDIS_URL", redis_url)
        .current_dir(&dir);
    let result = run_optional_command("mvn", &mut command);
    let _ = fs::remove_dir_all(dir);
    result
}

fn run_jvm_client_docker(redis_url: &str) -> ClientRun {
    let Ok(dir) = prepare_jvm_client() else {
        return ClientRun::Skipped("could not prepare temporary JVM module".to_owned());
    };
    let volume = docker_volume(&dir);
    let mut command = docker_client_command(redis_url);
    command
        .arg("-v")
        .arg(volume)
        .arg("-w")
        .arg("/workspace")
        .arg(JVM_CLIENT_DOCKER_IMAGE)
        .args(["mvn", "-q", "-f", "pom.xml", "compile", "exec:java"]);
    let result = run_optional_command("docker jedis", &mut command);
    let _ = fs::remove_dir_all(dir);
    result
}

fn prepare_jvm_client() -> Result<std::path::PathBuf, std::io::Error> {
    let dir = unique_temp_dir("jvm-client");
    fs::create_dir_all(dir.join("src/main/java/hydracache"))?;
    fs::write(
        dir.join("pom.xml"),
        r#"<project xmlns="http://maven.apache.org/POM/4.0.0">
  <modelVersion>4.0.0</modelVersion>
  <groupId>io.hydracache</groupId>
  <artifactId>redis-client-smoke</artifactId>
  <version>1.0.0</version>
  <properties>
    <maven.compiler.source>17</maven.compiler.source>
    <maven.compiler.target>17</maven.compiler.target>
  </properties>
  <dependencies>
    <dependency>
      <groupId>redis.clients</groupId>
      <artifactId>jedis</artifactId>
      <version>5.2.0</version>
    </dependency>
  </dependencies>
  <build>
    <plugins>
      <plugin>
        <groupId>org.codehaus.mojo</groupId>
        <artifactId>exec-maven-plugin</artifactId>
        <version>3.5.0</version>
        <configuration>
          <mainClass>hydracache.RedisClientSmoke</mainClass>
        </configuration>
      </plugin>
    </plugins>
  </build>
</project>"#,
    )?;
    fs::write(
        dir.join("src/main/java/hydracache/RedisClientSmoke.java"),
        r#"package hydracache;

import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.util.List;
import redis.clients.jedis.Jedis;
import redis.clients.jedis.commands.ProtocolCommand;

public final class RedisClientSmoke {
  public static void main(String[] args) {
    try (Jedis jedis = new Jedis(URI.create(System.getenv("REDIS_URL")))) {
      must("PONG".equals(jedis.ping()), "PING failed");
      String info = jedis.info();
      must(info.contains("redis_mode:standalone"), "INFO missing standalone mode");
      must(info.contains("hydracache_resp:RESP2+RESP3"), "INFO missing RESP dialects");
      must(!info.contains("used_memory") && !info.contains("db0:"), "INFO fabricated Redis server state");
      must("OK".equals(jedis.set("jvm:k", "v")), "SET failed");
      must("v".equals(jedis.get("jvm:k")), "GET failed");
      must("string".equals(jedis.type("jvm:k")), "TYPE existing failed");
      must("none".equals(jedis.type("jvm:missing")), "TYPE missing failed");
      must("OK".equals(jedis.mset("jvm:a", "1", "jvm:b", "2")), "MSET failed");
      List<String> msetValues = jedis.mget("jvm:a", "jvm:b");
      must(msetValues.size() == 2 && "1".equals(msetValues.get(0)) && "2".equals(msetValues.get(1)), "MSET/MGET failed");
      must("OK".equals(jedis.setex("jvm:ttl", 5, "v")), "SETEX failed");
      long ttl = jedis.ttl("jvm:ttl");
      must(ttl > 0L && ttl <= 5L, "TTL out of range");
      must(jedis.persist("jvm:ttl") == 1L, "PERSIST failed");
      must(jedis.ttl("jvm:ttl") == -1L, "TTL after PERSIST failed");
      List<String> values = jedis.mget("jvm:k", "jvm:missing");
      must(values.size() == 2 && "v".equals(values.get(0)) && values.get(1) == null, "MGET failed");
      must(jedis.exists("jvm:k", "jvm:missing") == 1L, "EXISTS failed");
      must(jedis.del("jvm:k", "jvm:missing") == 1L, "DEL failed");
      must("redis".equals(asString(jedis.sendCommand(HcCommand.NAMESPACE))), "HC.NAMESPACE failed");
      must("OK".equals(asString(jedis.sendCommand(HcCommand.NAMESPACE, "redis"))), "HC.NAMESPACE select failed");
      must("OK".equals(jedis.set("jvm:hc:a", "1")), "HC SET a failed");
      must("OK".equals(jedis.set("jvm:hc:b", "2")), "HC SET b failed");
      must("OK".equals(jedis.set("jvm:hc:keep", "keep")), "HC SET keep failed");
      must(Long.valueOf(2L).equals(jedis.sendCommand(HcCommand.TAG, "jvm:hc:a", "model", "shared")), "HC.TAG failed");
      must(Long.valueOf(0L).equals(jedis.sendCommand(HcCommand.TAG, "jvm:hc:a", "model")), "HC.TAG duplicate failed");
      must(Long.valueOf(1L).equals(jedis.sendCommand(HcCommand.SETTAGS, "jvm:hc:b", "model", "model")), "HC.SETTAGS failed");
      must(Long.valueOf(2L).equals(jedis.sendCommand(HcCommand.INVALIDATE_TAG, "model")), "HC.INVALIDATE_TAG failed");
      List<String> hcValues = jedis.mget("jvm:hc:a", "jvm:hc:b", "jvm:hc:keep");
      must(hcValues.size() == 3 && hcValues.get(0) == null && hcValues.get(1) == null && "keep".equals(hcValues.get(2)), "HC invalidate failed");
      must(Long.valueOf(0L).equals(jedis.sendCommand(HcCommand.INVALIDATE_TAG, "model")), "HC.INVALIDATE_TAG repeat failed");
    }
  }

  private enum HcCommand implements ProtocolCommand {
    NAMESPACE("HC.NAMESPACE"),
    TAG("HC.TAG"),
    SETTAGS("HC.SETTAGS"),
    INVALIDATE_TAG("HC.INVALIDATE_TAG");

    private final byte[] raw;

    HcCommand(String raw) {
      this.raw = raw.getBytes(StandardCharsets.UTF_8);
    }

    @Override
    public byte[] getRaw() {
      return raw;
    }
  }

  private static String asString(Object value) {
    if (value instanceof byte[] bytes) {
      return new String(bytes, StandardCharsets.UTF_8);
    }
    return String.valueOf(value);
  }

  private static void must(boolean ok, String message) {
    if (!ok) {
      throw new IllegalStateException(message);
    }
  }
}
"#,
    )?;
    Ok(dir)
}

fn docker_client_command(redis_url: &str) -> Command {
    let mut command = Command::new("docker");
    command
        .arg("run")
        .arg("--rm")
        .arg("--add-host")
        .arg(format!("{DOCKER_HOST_GATEWAY}:host-gateway"))
        .arg("-e")
        .arg(format!("REDIS_URL={redis_url}"));
    command
}

fn docker_volume(path: &std::path::Path) -> String {
    format!("{}:/workspace", path.display())
}

fn redis_auth_url(addr: SocketAddr) -> String {
    format!("redis://default:secret@{addr}/")
}

fn redis_auth_rediss_insecure_url(addr: SocketAddr) -> String {
    format!("rediss://default:secret@{addr}/#insecure")
}

fn docker_redis_auth_url(addr: SocketAddr) -> String {
    format!(
        "redis://default:secret@{DOCKER_HOST_GATEWAY}:{}/",
        addr.port()
    )
}

fn maven_command() -> Command {
    if cfg!(windows) {
        Command::new("mvn.cmd")
    } else {
        Command::new("mvn")
    }
}

fn unique_temp_dir(label: &str) -> std::path::PathBuf {
    env::temp_dir().join(format!(
        "hydracache-redis-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

struct TestTlsMaterial {
    cert_path: PathBuf,
    key_path: PathBuf,
}

fn write_test_tls_material(label: &str) -> TestTlsMaterial {
    let dir = unique_temp_dir(label);
    fs::create_dir_all(&dir).unwrap();
    let rcgen::CertifiedKey { cert, signing_key } =
        rcgen::generate_simple_self_signed(["127.0.0.1".to_owned(), "localhost".to_owned()])
            .unwrap();
    let cert_path = dir.join("cert.pem");
    let key_path = dir.join("key.pem");
    fs::write(&cert_path, cert.pem()).unwrap();
    fs::write(&key_path, signing_key.serialize_pem()).unwrap();
    TestTlsMaterial {
        cert_path,
        key_path,
    }
}

fn rediss_acceptor(label: &str) -> TlsAcceptor {
    install_test_rustls_provider();
    let material = write_test_tls_material(label);
    let certs = read_certs(&material.cert_path);
    let key = read_private_key(&material.key_path);
    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .unwrap();
    TlsAcceptor::from(Arc::new(config))
}

fn read_certs(path: &Path) -> Vec<CertificateDer<'static>> {
    let pem = fs::read(path).unwrap();
    let certs = CertificateDer::pem_slice_iter(&pem)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(
        !certs.is_empty(),
        "test TLS certificate file should contain a certificate"
    );
    certs
}

fn read_private_key(path: &Path) -> PrivateKeyDer<'static> {
    let pem = fs::read(path).unwrap();
    PrivateKeyDer::from_pem_slice(&pem).expect("test TLS private key file should contain a key")
}

fn install_test_rustls_provider() {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    }
}

fn run_optional_command(label: &str, command: &mut Command) -> ClientRun {
    let output = match command.output() {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return ClientRun::Skipped(format!("{label} executable not found"));
        }
        Err(error) => return ClientRun::Failed(error.to_string()),
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    if output.status.success() {
        ClientRun::Passed
    } else if output.status.code() == Some(CLIENT_RUNTIME_SKIP_EXIT) {
        ClientRun::Skipped(combined)
    } else {
        ClientRun::Failed(combined)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OracleReply {
    Status(String),
    Bulk(Vec<u8>),
    Int(i64),
    Array(Vec<OracleReply>),
    Nil,
    ErrorClass(String),
}

impl OracleReply {
    fn from_value(value: Value) -> Self {
        match value {
            Value::Nil => Self::Nil,
            Value::Int(value) => Self::Int(value),
            Value::BulkString(value) => Self::Bulk(value),
            Value::Array(values) => Self::Array(values.into_iter().map(Self::from_value).collect()),
            Value::SimpleString(value) => Self::Status(value),
            Value::Okay => Self::Status("OK".to_owned()),
            Value::ServerError(error) => Self::ErrorClass(error.code().to_owned()),
            other => Self::Status(format!("{other:?}")),
        }
    }
}

async fn run_supported_subset_scenario(addr: SocketAddr, prefix: &str) -> Vec<OracleReply> {
    let key = format!("{prefix}:k");
    let first = format!("{prefix}:a");
    let second = format!("{prefix}:b");
    let missing = format!("{prefix}:missing");
    vec![
        query_reply(addr, "PING", &[]).await,
        query_reply(addr, "ECHO", &["hello"]).await,
        query_reply(addr, "SET", &[&key, "v"]).await,
        query_reply(addr, "GET", &[&key]).await,
        query_reply(addr, "TYPE", &[&key]).await,
        query_reply(addr, "TYPE", &[&missing]).await,
        query_reply(addr, "MSET", &[&first, "1", &second, "2", &first, "3"]).await,
        query_reply(addr, "MGET", &[&first, &second]).await,
        query_reply(addr, "MGET", &[&key, &missing]).await,
        query_reply(addr, "EXISTS", &[&key, &missing]).await,
        query_reply(addr, "DEL", &[&key, &missing]).await,
        query_reply(addr, "GET", &[&key]).await,
    ]
}

async fn run_count_scenario(addr: SocketAddr, prefix: &str) -> Vec<OracleReply> {
    let first = format!("{prefix}:first");
    let second = format!("{prefix}:second");
    let missing = format!("{prefix}:missing");
    vec![
        query_reply(addr, "SET", &[&first, "1"]).await,
        query_reply(addr, "SET", &[&second, "2"]).await,
        query_reply(addr, "EXISTS", &[&first, &second, &missing]).await,
        query_reply(addr, "DEL", &[&first, &second, &missing]).await,
        query_reply(addr, "EXISTS", &[&first, &second, &missing]).await,
    ]
}

async fn run_mget_order_scenario(addr: SocketAddr, prefix: &str) -> Vec<OracleReply> {
    let first = format!("{prefix}:first");
    let second = format!("{prefix}:second");
    let missing = format!("{prefix}:missing");
    vec![
        query_reply(addr, "SET", &[&first, "1"]).await,
        query_reply(addr, "SET", &[&second, "2"]).await,
        query_reply(addr, "MGET", &[&second, &missing, &first]).await,
    ]
}

async fn run_mset_scenario(addr: SocketAddr, prefix: &str) -> Vec<OracleReply> {
    let first = format!("{prefix}:first");
    let second = format!("{prefix}:second");
    vec![
        query_reply(addr, "MSET", &[&first, "1", &second, "2", &first, "3"]).await,
        query_reply(addr, "MGET", &[&first, &second]).await,
    ]
}

async fn run_ttl_scenario(addr: SocketAddr, prefix: &str) -> Vec<OracleReply> {
    let key = format!("{prefix}:key");
    let missing = format!("{prefix}:missing");
    vec![
        query_reply(addr, "SET", &[&key, "v", "PX", "5000"]).await,
        query_reply(addr, "PTTL", &[&key]).await,
        query_reply(addr, "TTL", &[&key]).await,
        query_reply(addr, "PERSIST", &[&key]).await,
        query_reply(addr, "TTL", &[&key]).await,
        query_reply(addr, "EXPIRE", &[&key, "1"]).await,
        query_reply(addr, "TTL", &[&key]).await,
        query_reply(addr, "TTL", &[&missing]).await,
    ]
}

async fn run_ttl_edge_scenario(addr: SocketAddr, prefix: &str) -> Vec<OracleReply> {
    let zero = format!("{prefix}:zero");
    let negative = format!("{prefix}:negative");
    let missing = format!("{prefix}:missing");
    vec![
        query_reply(addr, "SET", &[&zero, "v"]).await,
        query_reply(addr, "EXPIRE", &[&zero, "0"]).await,
        query_reply(addr, "GET", &[&zero]).await,
        query_reply(addr, "EXISTS", &[&zero]).await,
        query_reply(addr, "TTL", &[&zero]).await,
        query_reply(addr, "MGET", &[&zero, &missing]).await,
        query_reply(addr, "SET", &[&negative, "v"]).await,
        query_reply(addr, "PEXPIRE", &[&negative, "-1"]).await,
        query_reply(addr, "GET", &[&negative]).await,
        query_reply(addr, "EXPIRE", &[&missing, "30"]).await,
        query_reply(addr, "PEXPIRE", &[&missing, "250"]).await,
        query_reply(addr, "PERSIST", &[&missing]).await,
    ]
}

async fn run_lock_acquire_scenario(addr: SocketAddr, prefix: &str) -> Vec<OracleReply> {
    let key = format!("{prefix}:key");
    vec![
        query_reply(addr, "SET", &[&key, "owner", "NX", "PX", "5000"]).await,
        query_reply(addr, "SET", &[&key, "contender", "NX", "PX", "5000"]).await,
        query_reply(addr, "GET", &[&key]).await,
    ]
}

async fn run_lock_release_scenario(addr: SocketAddr, prefix: &str) -> Vec<OracleReply> {
    let key = format!("{prefix}:key");
    vec![
        query_reply(addr, "SET", &[&key, "owner", "NX", "PX", "5000"]).await,
        query_reply(addr, "EVAL", &[LOCK_RELEASE_SCRIPT, "1", &key, "contender"]).await,
        query_reply(addr, "GET", &[&key]).await,
        query_reply(addr, "EVAL", &[LOCK_RELEASE_SCRIPT, "1", &key, "owner"]).await,
        query_reply(addr, "GET", &[&key]).await,
    ]
}

async fn run_lock_extend_scenario(addr: SocketAddr, prefix: &str) -> Vec<OracleReply> {
    let key = format!("{prefix}:key");
    vec![
        query_reply(addr, "SET", &[&key, "owner", "NX", "PX", "100"]).await,
        query_reply(
            addr,
            "EVAL",
            &[LOCK_EXTEND_SCRIPT, "1", &key, "contender", "5000"],
        )
        .await,
        query_reply(
            addr,
            "EVAL",
            &[LOCK_EXTEND_SCRIPT, "1", &key, "owner", "5000"],
        )
        .await,
        query_reply(addr, "PTTL", &[&key]).await,
    ]
}

async fn run_redis_py_lock_extend_scenario(addr: SocketAddr, prefix: &str) -> Vec<OracleReply> {
    let key = format!("{prefix}:key");
    let persistent_key = format!("{prefix}:persistent");
    vec![
        query_reply(addr, "SET", &[&key, "owner", "NX", "PX", "10000"]).await,
        query_reply(
            addr,
            "EVAL",
            &[
                LOCK_EXTEND_SCRIPT_REDIS_PY,
                "1",
                &key,
                "contender",
                "5000",
                "0",
            ],
        )
        .await,
        query_reply(
            addr,
            "EVAL",
            &[LOCK_EXTEND_SCRIPT_REDIS_PY, "1", &key, "owner", "5000", "0"],
        )
        .await,
        query_reply(addr, "PTTL", &[&key]).await,
        query_reply(
            addr,
            "EVAL",
            &[LOCK_EXTEND_SCRIPT_REDIS_PY, "1", &key, "owner", "4000", "1"],
        )
        .await,
        query_reply(addr, "PTTL", &[&key]).await,
        query_reply(addr, "SET", &[&persistent_key, "owner"]).await,
        query_reply(
            addr,
            "EVAL",
            &[
                LOCK_EXTEND_SCRIPT_REDIS_PY,
                "1",
                &persistent_key,
                "owner",
                "5000",
                "0",
            ],
        )
        .await,
        query_reply(addr, "PTTL", &[&persistent_key]).await,
    ]
}

fn assert_ttl_scenario_shape(replies: Vec<OracleReply>, label: &str) {
    assert_eq!(replies.len(), 8, "{label}: unexpected TTL scenario length");
    assert_eq!(
        replies[0],
        OracleReply::Status("OK".to_owned()),
        "{label}: SET PX"
    );
    let OracleReply::Int(pttl) = replies[1] else {
        panic!("{label}: PTTL should return integer, got {:?}", replies[1]);
    };
    assert!(
        (1..=5_000).contains(&pttl),
        "{label}: PTTL should be within 1..=5000, got {pttl}"
    );
    let OracleReply::Int(ttl) = replies[2] else {
        panic!("{label}: TTL should return integer, got {:?}", replies[2]);
    };
    assert!(
        (1..=5).contains(&ttl),
        "{label}: TTL should be within 1..=5, got {ttl}"
    );
    assert_eq!(replies[3], OracleReply::Int(1), "{label}: PERSIST");
    assert_eq!(
        replies[4],
        OracleReply::Int(-1),
        "{label}: TTL after PERSIST"
    );
    assert_eq!(replies[5], OracleReply::Int(1), "{label}: EXPIRE");
    let OracleReply::Int(expiring_ttl) = replies[6] else {
        panic!(
            "{label}: TTL after EXPIRE should return integer, got {:?}",
            replies[6]
        );
    };
    assert!(
        (0..=1).contains(&expiring_ttl),
        "{label}: TTL after EXPIRE should be within 0..=1, got {expiring_ttl}"
    );
    assert_eq!(replies[7], OracleReply::Int(-2), "{label}: TTL missing");
}

fn assert_lock_extend_shape(replies: Vec<OracleReply>, label: &str) {
    assert_eq!(
        replies.len(),
        4,
        "{label}: unexpected lock extend scenario length"
    );
    assert_eq!(
        replies[0],
        OracleReply::Status("OK".to_owned()),
        "{label}: SET NX PX"
    );
    assert_eq!(
        replies[1],
        OracleReply::Int(0),
        "{label}: wrong token extend"
    );
    assert_eq!(replies[2], OracleReply::Int(1), "{label}: owner extend");
    let OracleReply::Int(pttl) = replies[3] else {
        panic!(
            "{label}: PTTL after extend should return integer, got {:?}",
            replies[3]
        );
    };
    assert!(
        (1..=5_000).contains(&pttl),
        "{label}: PTTL after extend should be within 1..=5000, got {pttl}"
    );
}

fn assert_redis_py_lock_extend_shape(replies: Vec<OracleReply>, label: &str) {
    assert_eq!(
        replies.len(),
        9,
        "{label}: unexpected redis-py lock extend scenario length"
    );
    assert_eq!(
        replies[0],
        OracleReply::Status("OK".to_owned()),
        "{label}: SET NX PX"
    );
    assert_eq!(
        replies[1],
        OracleReply::Int(0),
        "{label}: wrong token additive extend"
    );
    assert_eq!(
        replies[2],
        OracleReply::Int(1),
        "{label}: owner additive extend"
    );
    let OracleReply::Int(additive_pttl) = replies[3] else {
        panic!(
            "{label}: PTTL after additive redis-py extend should return integer, got {:?}",
            replies[3]
        );
    };
    assert!(
        (5_001..=15_000).contains(&additive_pttl),
        "{label}: additive extend should preserve remaining TTL and add new TTL, got {additive_pttl}"
    );
    assert_eq!(
        replies[4],
        OracleReply::Int(1),
        "{label}: owner replace_ttl extend"
    );
    let OracleReply::Int(replace_pttl) = replies[5] else {
        panic!(
            "{label}: PTTL after replace_ttl redis-py extend should return integer, got {:?}",
            replies[5]
        );
    };
    assert!(
        (1..=4_000).contains(&replace_pttl),
        "{label}: replace_ttl extend should replace remaining TTL, got {replace_pttl}"
    );
    assert_eq!(
        replies[6],
        OracleReply::Status("OK".to_owned()),
        "{label}: persistent SET"
    );
    assert_eq!(
        replies[7],
        OracleReply::Int(0),
        "{label}: redis-py extend on persistent key"
    );
    assert_eq!(
        replies[8],
        OracleReply::Int(-1),
        "{label}: persistent key should remain persistent"
    );
}

async fn query_reply(addr: SocketAddr, command: &str, args: &[&str]) -> OracleReply {
    let client = redis::Client::open(format!("redis://{addr}/")).unwrap();
    let mut connection = client.get_multiplexed_async_connection().await.unwrap();
    let mut cmd = redis::cmd(command);
    for arg in args {
        cmd.arg(arg);
    }
    match cmd.query_async::<Value>(&mut connection).await {
        Ok(value) => OracleReply::from_value(value),
        Err(error) => OracleReply::ErrorClass(error_class(&error)),
    }
}

fn error_class(error: &redis::RedisError) -> String {
    error.code().unwrap_or("ERR").to_owned()
}

struct RedisOracle {
    container_id: String,
    addr: SocketAddr,
}

impl RedisOracle {
    async fn start_first_available() -> Option<Self> {
        if !docker_available() {
            if env_gate_enabled(REDIS_ORACLE_REQUIRE_ENV) {
                panic!("real Redis oracle was required but docker CLI is not available");
            }
            eprintln!("skipping real Redis oracle; docker CLI is not available");
            return None;
        }
        let mut last_error = None;
        for image in PINNED_REDIS_IMAGES {
            match Self::start(image).await {
                Ok(oracle) => return Some(oracle),
                Err(error) => {
                    last_error = Some(error);
                    eprintln!("redis oracle image {image} did not start: {last_error:?}");
                }
            }
        }
        panic!(
            "none of the pinned Redis oracle images could start: {:?}",
            last_error
        );
    }

    async fn start(image: &str) -> Result<Self, String> {
        let output = Command::new("docker")
            .args([
                "run",
                "--rm",
                "-d",
                "-p",
                "127.0.0.1::6379",
                image,
                "redis-server",
                "--save",
                "",
                "--appendonly",
                "no",
            ])
            .output()
            .map_err(|error| error.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }
        let container_id = String::from_utf8(output.stdout)
            .map_err(|error| error.to_string())?
            .trim()
            .to_owned();
        let addr = docker_published_redis_addr(&container_id)?;
        wait_for_redis(addr).await?;
        Ok(Self { container_id, addr })
    }
}

impl Drop for RedisOracle {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.container_id])
            .output();
    }
}

fn docker_available() -> bool {
    Command::new("docker")
        .args(["version", "--format", "{{.Server.Version}}"])
        .output()
        .is_ok_and(|output| output.status.success())
}

fn docker_published_redis_addr(container_id: &str) -> Result<SocketAddr, String> {
    let output = Command::new("docker")
        .args(["port", container_id, "6379/tcp"])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    let port_line = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
    let port = port_line
        .lines()
        .next()
        .and_then(|line| line.rsplit(':').next())
        .ok_or_else(|| format!("docker port did not return host port: {port_line}"))?;
    format!("127.0.0.1:{port}")
        .parse::<SocketAddr>()
        .map_err(|error| error.to_string())
}

async fn wait_for_redis(addr: SocketAddr) -> Result<(), String> {
    for _ in 0..50 {
        let client = redis::Client::open(format!("redis://{addr}/")).map_err(|e| e.to_string())?;
        if let Ok(mut connection) = client.get_multiplexed_async_connection().await {
            let pong = redis::cmd("PING")
                .query_async::<String>(&mut connection)
                .await;
            if pong.as_deref() == Ok("PONG") {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(format!("redis oracle at {addr} did not become ready"))
}

async fn spawn_resp_facade() -> (watch::Sender<bool>, SocketAddr, JoinHandle<()>) {
    spawn_resp_facade_on("127.0.0.1:0", RedisListenerConfig::default()).await
}

async fn spawn_auth_resp_facade() -> (watch::Sender<bool>, SocketAddr, JoinHandle<()>) {
    spawn_resp_facade_on("127.0.0.1:0", auth_listener_config()).await
}

async fn spawn_rediss_auth_resp_facade() -> (watch::Sender<bool>, SocketAddr, JoinHandle<()>) {
    spawn_rediss_resp_facade_on("127.0.0.1:0", auth_listener_config()).await
}

async fn spawn_auth_resp_facade_for_docker_clients(
) -> (watch::Sender<bool>, SocketAddr, JoinHandle<()>) {
    spawn_resp_facade_on("0.0.0.0:0", auth_listener_config()).await
}

async fn spawn_resp_facade_on(
    bind: &str,
    config: RedisListenerConfig,
) -> (watch::Sender<bool>, SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind(bind).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), addr.port());
    let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
    let server = Arc::new(
        RedisRespServer::new(
            state,
            RedisListenerConfig {
                tenant: DEFAULT_REDIS_NAMESPACE.to_owned(),
                ..config
            },
        )
        .unwrap(),
    );
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let serving = tokio::spawn(async move {
        loop {
            tokio::select! {
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        return;
                    }
                }
                accepted = listener.accept() => {
                    let (stream, _) = accepted.unwrap();
                    let server = Arc::clone(&server);
                    tokio::spawn(async move {
                        let _ = server.serve_connection(stream).await;
                    });
                }
            }
        }
    });
    (shutdown_tx, addr, serving)
}

async fn spawn_rediss_resp_facade_on(
    bind: &str,
    config: RedisListenerConfig,
) -> (watch::Sender<bool>, SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind(bind).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), addr.port());
    let tls = rediss_acceptor("client-matrix-rediss");
    let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
    let server = Arc::new(
        RedisRespServer::new(
            state,
            RedisListenerConfig {
                tenant: DEFAULT_REDIS_NAMESPACE.to_owned(),
                ..config
            },
        )
        .unwrap(),
    );
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let serving = tokio::spawn(async move {
        loop {
            tokio::select! {
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        return;
                    }
                }
                accepted = listener.accept() => {
                    let (stream, _) = accepted.unwrap();
                    let server = Arc::clone(&server);
                    let tls = tls.clone();
                    tokio::spawn(async move {
                        if let Ok(stream) = tls.accept(stream).await {
                            let _ = server.serve_connection(stream).await;
                        }
                    });
                }
            }
        }
    });
    (shutdown_tx, addr, serving)
}

fn auth_listener_config() -> RedisListenerConfig {
    let mut auth = RedisAuthConfig::required("secret");
    auth.username = Some("default".to_owned());
    RedisListenerConfig {
        auth,
        ..RedisListenerConfig::default()
    }
}
