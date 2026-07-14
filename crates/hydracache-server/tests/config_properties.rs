use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use hydracache_observability::PrometheusExporter;
use hydracache_server::{
    ClusterStartMode, RedisApiConfig, ServerConfig, ServerConfigError, ServerRole, ServerRuntime,
    TlsConfig,
};
use proptest::prelude::*;
use proptest::test_runner::TestRunner;

const CONFIG_ENV_VARS: &[&str] = &[
    "HYDRACACHE_ROLE",
    "HYDRACACHE_LISTEN_ADDR",
    "HYDRACACHE_CLUSTER_ADDR",
    "HYDRACACHE_CLUSTER_START",
    "HYDRACACHE_CLUSTER_ADVERTISE_ADDR",
    "HYDRACACHE_CLUSTER_HEADLESS_SERVICE",
    "HYDRACACHE_BOOTSTRAP_REPLICAS",
    "HYDRACACHE_NODE_ID",
    "HYDRACACHE_STORAGE_DIR",
    "HYDRACACHE_SEEDS",
    "HYDRACACHE_JOIN_TIMEOUT_MS",
    "HYDRACACHE_TLS_ACK_INSECURE",
    "HYDRACACHE_TLS_ENABLED",
    "HYDRACACHE_TLS_CERT_PATH",
    "HYDRACACHE_TLS_KEY_PATH",
    "HYDRACACHE_TLS_CA_PATH",
    "HYDRACACHE_CLUSTER_AUTH_KEY_ID",
    "HYDRACACHE_CLUSTER_AUTH_TOKEN_FILE",
    "HYDRACACHE_CLUSTER_AUTH_PREVIOUS_KEY_ID",
    "HYDRACACHE_CLUSTER_AUTH_PREVIOUS_TOKEN_FILE",
    "HYDRACACHE_BACKUP_ENABLED",
    "HYDRACACHE_BACKUP_LOCATION",
    "HYDRACACHE_CLIENT_API_ENABLED",
    "HYDRACACHE_ADMIN_API_ENABLED",
    "HYDRACACHE_ADMIN_ADDR",
    "HYDRACACHE_REDIS_API_ENABLED",
    "HYDRACACHE_REDIS_ADDR",
    "HYDRACACHE_REDIS_AUTH_REQUIRED",
    "HYDRACACHE_REDIS_AUTH_USERNAME",
    "HYDRACACHE_REDIS_AUTH_TOKEN_FILE",
    "HYDRACACHE_REDIS_REDISS_ENABLED",
    "HOSTNAME",
];

struct ConfigEnvGuard {
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl ConfigEnvGuard {
    fn new(overrides: &[(&'static str, &'static str)]) -> Self {
        let saved = CONFIG_ENV_VARS
            .iter()
            .map(|name| (*name, env::var_os(name)))
            .collect::<Vec<_>>();
        for name in CONFIG_ENV_VARS {
            env::remove_var(name);
        }
        for (name, value) in overrides {
            env::set_var(name, value);
        }
        Self { saved }
    }
}

impl Drop for ConfigEnvGuard {
    fn drop(&mut self) {
        for name in CONFIG_ENV_VARS {
            env::remove_var(name);
        }
        for (name, value) in &self.saved {
            if let Some(value) = value {
                env::set_var(name, value);
            }
        }
    }
}

fn write_token(name: &str, token: &str) -> PathBuf {
    let dir = PathBuf::from("target/test-hydracache-server/config-properties");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    fs::write(&path, token).unwrap();
    path
}

fn complete_tls() -> TlsConfig {
    TlsConfig {
        enabled: true,
        cert_path: Some(PathBuf::from("/run/hydracache/tls/tls.crt")),
        key_path: Some(PathBuf::from("/run/hydracache/tls/tls.key")),
        ca_path: Some(PathBuf::from("/run/hydracache/tls/ca.crt")),
        acknowledge_insecure: false,
    }
}

fn output_is_redacted(output: &str, secret: &str) -> Result<(), String> {
    if output.contains(secret) {
        Err("credential bytes appeared in a diagnostic surface".to_owned())
    } else {
        Ok(())
    }
}

#[test]
fn generated_server_configs_preserve_precedence_validation_and_secure_defaults() {
    let token_path = write_token("redis-token", "matrix-token");
    let strategy = (
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        0_u8..4,
    );
    let mut runner = TestRunner::default();
    runner
        .run(
            &strategy,
            |(
                external,
                tls_enabled,
                complete_tls_material,
                acknowledge_insecure,
                redis_enabled,
                auth_required,
                auth_material,
                rediss_enabled,
                redis_conflict,
            )| {
                let mut config = ServerConfig::default();
                if external {
                    config.listen_addr = "192.0.2.10:8080".parse().unwrap();
                }
                config.tls.acknowledge_insecure = acknowledge_insecure;
                if tls_enabled {
                    config.tls = complete_tls();
                    config.tls.acknowledge_insecure = acknowledge_insecure;
                    if !complete_tls_material {
                        config.tls.key_path = None;
                    }
                }
                config.redis_api = RedisApiConfig {
                    enabled: redis_enabled,
                    listen_addr: match redis_conflict {
                        1 => config.listen_addr,
                        2 => config.cluster_addr,
                        3 => config.admin_api.listen_addr,
                        _ => "127.0.0.1:6380".parse().unwrap(),
                    },
                    auth_required,
                    auth_username: auth_required.then(|| "matrix-user".to_owned()),
                    auth_token_file: auth_material.then(|| token_path.clone()),
                    rediss_enabled,
                };

                let tls_material_valid = !tls_enabled || complete_tls_material;
                let external_listener_valid = !external || tls_enabled || acknowledge_insecure;
                let rediss_valid = !redis_enabled || !rediss_enabled || tls_enabled;
                let auth_valid = !redis_enabled || !auth_required || auth_material;
                let listener_valid = !redis_enabled || redis_conflict == 0;
                let expected_valid = tls_material_valid
                    && external_listener_valid
                    && rediss_valid
                    && auth_valid
                    && listener_valid;

                let validation = config.validate();
                prop_assert_eq!(validation.is_ok(), expected_valid);
                if expected_valid {
                    let first = toml::to_string(&config).unwrap();
                    let second = toml::to_string(&config).unwrap();
                    prop_assert_eq!(&first, &second);
                    prop_assert_eq!(ServerConfig::from_toml_str(&first).unwrap(), config.clone());

                    // Server config is an additive, human-authored document: unknown fields are
                    // ignored, while every known field is still validated before startup.
                    let future = format!("future_release_knob = 7\n{first}");
                    prop_assert_eq!(ServerConfig::from_toml_str(&future).unwrap(), config);
                }
                Ok(())
            },
        )
        .unwrap();

    let _guard = ConfigEnvGuard::new(&[
        ("HYDRACACHE_ROLE", "member"),
        ("HYDRACACHE_LISTEN_ADDR", "127.0.0.1:18081"),
        ("HYDRACACHE_CLUSTER_ADDR", "127.0.0.1:17000"),
        ("HYDRACACHE_CLUSTER_START", "bootstrap"),
        ("HYDRACACHE_CLUSTER_HEADLESS_SERVICE", "matrix-headless"),
        ("HYDRACACHE_BOOTSTRAP_REPLICAS", "3"),
        ("HYDRACACHE_STORAGE_DIR", "target/config-properties/member"),
        ("HYDRACACHE_SEEDS", "matrix-0.matrix-headless:7000"),
        ("HYDRACACHE_ADMIN_API_ENABLED", "false"),
        ("HOSTNAME", "matrix-4"),
    ]);
    let env_config = ServerConfig::from_env().unwrap();
    assert_eq!(env_config.role, ServerRole::Member);
    assert_eq!(env_config.listen_addr.to_string(), "127.0.0.1:18081");
    assert_eq!(env_config.cluster_start, ClusterStartMode::Bootstrap);
    assert_eq!(env_config.node_id.as_deref(), Some("matrix-4"));
    assert!(!env_config.admin_api.enabled);

    let mut insecure = ServerConfig::default();
    insecure.listen_addr = "192.0.2.20:8080".parse().unwrap();
    assert!(matches!(
        insecure.validate(),
        Err(ServerConfigError::NonLoopbackWithoutTls)
    ));
}

#[tokio::test]
async fn generated_config_errors_and_debug_output_never_expose_secret_bytes() {
    for index in 0..16 {
        let secret = format!("HC-W36-{index:02}-opaque-credential-9f4c2a");
        let token_path = write_token(&format!("token-{index}"), &secret);
        let mut config = ServerConfig::default();
        config.redis_api = RedisApiConfig {
            enabled: true,
            listen_addr: "127.0.0.1:6380".parse().unwrap(),
            auth_required: true,
            auth_username: Some("matrix-user".to_owned()),
            auth_token_file: Some(token_path),
            rediss_enabled: false,
        };

        config.validate().unwrap();
        output_is_redacted(&format!("{config:?}"), &secret).unwrap();
        output_is_redacted(&toml::to_string(&config).unwrap(), &secret).unwrap();
        output_is_redacted(
            &format!("{:?}", config.redis_listener_config().unwrap()),
            &secret,
        )
        .unwrap();

        let runtime = ServerRuntime::new(config).unwrap();
        output_is_redacted(&format!("{runtime:?}"), &secret).unwrap();
        let metrics = PrometheusExporter::new(runtime.metrics_registry())
            .render()
            .await;
        output_is_redacted(&metrics, &secret).unwrap();
    }

    let missing = Path::new("target/test-hydracache-server/config-properties/missing-token");
    let mut invalid = ServerConfig::default();
    invalid.redis_api = RedisApiConfig {
        enabled: true,
        listen_addr: "127.0.0.1:6380".parse().unwrap(),
        auth_required: true,
        auth_username: None,
        auth_token_file: Some(missing.to_path_buf()),
        rediss_enabled: false,
    };
    let error = invalid.validate().unwrap_err().to_string();
    assert!(error.contains("missing-token"));
    assert!(!error.contains("opaque-credential"));
}

#[test]
fn canary_config_debug_output_contains_a_generated_secret() {
    let secret = "HC-W36-canary-secret";
    let faulty_debug = format!("RedisAuthConfig {{ password: {secret} }}");
    assert!(
        output_is_redacted(&faulty_debug, secret).is_err(),
        "the redaction guard must reject a diagnostic surface containing credential bytes"
    );
}
