use std::net::SocketAddr;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use std::{env, fs};

use hydracache_client_transport_axum::{ClientSurfaceLimits, ClientSurfaceState};
use hydracache_redis_compat::{RedisListenerConfig, RedisRespServer, DEFAULT_REDIS_NAMESPACE};
use redis::Value;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio::task::JoinHandle;

const CLIENT_MATRIX_ENV: &str = "HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS";
const CLIENT_RUNTIME_SKIP_EXIT: i32 = 42;
const PINNED_REDIS_IMAGES: [&str; 2] = ["redis:6.2.14", "redis:7.2.5"];

#[test]
fn redis_client_gate_manifest_and_docs_are_wired() {
    let gates = include_str!("../../../docs/GATES.md");
    let testing = include_str!("../../../docs/TESTING.md");
    let manifest = include_str!("../../../docs/integrations/redis_compat_conformance.json");

    assert!(gates.contains(CLIENT_MATRIX_ENV));
    assert!(gates.contains("--test redis_clients"));
    assert!(testing.contains("--test redis_clients"));
    assert!(manifest.contains(r#""images": ["redis:6.2.14", "redis:7.2.5"]"#));
    assert!(manifest.contains("redis_oracle_supported_subset_matches_real_redis"));
    assert!(manifest.contains("redis_oracle_del_exists_counts_match_real_redis"));
    assert!(manifest.contains("redis_oracle_mget_nil_and_order_match_real_redis"));
    assert!(manifest.contains("redis_oracle_unsupported_divergence_is_documented"));
    assert!(manifest.contains("redis_oracle_hc_extensions_are_hydracache_only"));
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
        "HYDRACACHE_REQUIRE_REDIS_CLIENT_PYTHON",
        "HYDRACACHE_REQUIRE_REDIS_CLIENT_NODE",
        "HYDRACACHE_REQUIRE_REDIS_CLIENT_GO",
        "HYDRACACHE_REQUIRE_REDIS_CLIENT_JVM",
    ] {
        assert!(source.contains(env_var));
        assert!(testing.contains(env_var));
    }
    assert!(gates.contains("--test redis_clients"));
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1; uses mainstream redis-rs client path"]
async fn mainstream_redis_client_can_talk_to_the_facade() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping Redis client matrix; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }

    let (shutdown, addr, serving) = spawn_resp_facade().await;
    let client = redis::Client::open(format!("redis://{addr}/")).unwrap();
    let mut connection = client.get_multiplexed_async_connection().await.unwrap();

    let pong: String = redis::cmd("PING")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(pong, "PONG");

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

    let values: Vec<Option<String>> = redis::cmd("MGET")
        .arg("k")
        .arg("missing")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(values, vec![Some("v".to_owned()), None]);

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
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 and optional Python/Node/Go/JVM client runtimes"]
async fn nightly_python_node_go_jvm_clients_bootstrap_and_run_supported_subset() {
    if !env_gate_enabled(CLIENT_MATRIX_ENV) {
        eprintln!("skipping Redis client matrix; set {CLIENT_MATRIX_ENV}=1 to run it");
        return;
    }

    let (shutdown, addr, serving) = spawn_resp_facade().await;
    let url = format!("redis://{addr}/");
    for ecosystem in ClientEcosystem::all() {
        let url = url.clone();
        tokio::task::spawn_blocking(move || run_external_client(ecosystem, &url))
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

    assert!(
        matches!(
            redis_reply,
            OracleReply::ErrorClass(ref class) if class == "ERR"
        ),
        "real Redis HC.STATS should normalize to ERR, got {redis_reply:?}"
    );
    assert!(matches!(hydracache_reply, OracleReply::Array(_)));

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
}

fn run_external_client(ecosystem: ClientEcosystem, redis_url: &str) {
    let result = match ecosystem {
        ClientEcosystem::Python => run_python_client(redis_url),
        ClientEcosystem::Node => run_node_client(redis_url),
        ClientEcosystem::Go => run_go_client(redis_url),
        ClientEcosystem::Jvm => run_jvm_client(redis_url),
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

enum ClientRun {
    Passed,
    Skipped(String),
    Failed(String),
}

fn run_python_client(redis_url: &str) -> ClientRun {
    let script = r#"
import sys
try:
    import redis
except Exception as exc:
    print(f"missing python redis client: {exc}")
    sys.exit(42)
r = redis.Redis.from_url(sys.argv[1], decode_responses=True)
assert r.ping() is True
assert r.set("python:k", "v") is True
assert r.get("python:k") == "v"
assert r.mget(["python:k", "python:missing"]) == ["v", None]
assert r.exists("python:k", "python:missing") == 1
assert r.delete("python:k", "python:missing") == 1
"#;
    run_optional_command(
        "python",
        Command::new("python").arg("-c").arg(script).arg(redis_url),
    )
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
  const client = redis.createClient({ url: process.argv[1] });
  await client.connect();
  if (await client.ping() !== "PONG") throw new Error("PING failed");
  if (await client.set("node:k", "v") !== "OK") throw new Error("SET failed");
  if (await client.get("node:k") !== "v") throw new Error("GET failed");
  const values = await client.mGet(["node:k", "node:missing"]);
  if (JSON.stringify(values) !== JSON.stringify(["v", null])) throw new Error(`MGET failed: ${JSON.stringify(values)}`);
  if (await client.exists(["node:k", "node:missing"]) !== 1) throw new Error("EXISTS failed");
  if (await client.del(["node:k", "node:missing"]) !== 1) throw new Error("DEL failed");
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

    set, err := client.Set(ctx, "go:k", "v", 0).Result()
    mustNoErr(err, "SET failed")
    must(set == "OK", fmt.Sprintf("SET got %q", set))

    got, err := client.Get(ctx, "go:k").Result()
    mustNoErr(err, "GET failed")
    must(got == "v", fmt.Sprintf("GET got %q", got))

    values, err := client.MGet(ctx, "go:k", "go:missing").Result()
    mustNoErr(err, "MGET failed")
    must(len(values) == 2 && values[0] == "v" && values[1] == nil, fmt.Sprintf("MGET failed: %#v", values))

    exists, err := client.Exists(ctx, "go:k", "go:missing").Result()
    mustNoErr(err, "EXISTS failed")
    must(exists == 1, fmt.Sprintf("EXISTS got %d", exists))

    deleted, err := client.Del(ctx, "go:k", "go:missing").Result()
    mustNoErr(err, "DEL failed")
    must(deleted == 1, fmt.Sprintf("DEL got %d", deleted))
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
    let mut command = Command::new("mvn");
    command
        .args(["-q", "-f", "pom.xml", "compile", "exec:java"])
        .env("REDIS_URL", redis_url)
        .current_dir(&dir);
    let result = run_optional_command("mvn", &mut command);
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
import java.util.List;
import redis.clients.jedis.Jedis;

public final class RedisClientSmoke {
  public static void main(String[] args) {
    try (Jedis jedis = new Jedis(URI.create(System.getenv("REDIS_URL")))) {
      must("PONG".equals(jedis.ping()), "PING failed");
      must("OK".equals(jedis.set("jvm:k", "v")), "SET failed");
      must("v".equals(jedis.get("jvm:k")), "GET failed");
      List<String> values = jedis.mget("jvm:k", "jvm:missing");
      must(values.size() == 2 && "v".equals(values.get(0)) && values.get(1) == null, "MGET failed");
      must(jedis.exists("jvm:k", "jvm:missing") == 1L, "EXISTS failed");
      must(jedis.del("jvm:k", "jvm:missing") == 1L, "DEL failed");
    }
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
    let missing = format!("{prefix}:missing");
    vec![
        query_reply(addr, "PING", &[]).await,
        query_reply(addr, "ECHO", &["hello"]).await,
        query_reply(addr, "SET", &[&key, "v"]).await,
        query_reply(addr, "GET", &[&key]).await,
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
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
    let server = Arc::new(
        RedisRespServer::new(
            state,
            RedisListenerConfig {
                tenant: DEFAULT_REDIS_NAMESPACE.to_owned(),
                ..RedisListenerConfig::default()
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
