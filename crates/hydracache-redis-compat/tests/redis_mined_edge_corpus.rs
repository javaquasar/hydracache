use std::sync::Arc;

use hydracache_client_transport_axum::{ClientSurfaceLimits, ClientSurfaceState};
use hydracache_redis_compat::{RedisListenerConfig, RedisRespServer};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn redis_mined_edge_corpus_matches_oracle_for_supported_subset() {
    let docs = include_str!("../../../docs/integrations/redis_edge_corpus.md");
    let mut executed = 0usize;

    for row in mined_edge_corpus() {
        assert!(
            docs.contains(row.label),
            "Redis mined edge corpus docs must describe {label}",
            label = row.label
        );
        let server = listener();
        let actual = exchange(&server, row.input).await;
        assert_eq!(
            actual, row.expected,
            "Redis mined corpus row `{}` from {} diverged",
            row.label, row.source
        );
        executed += 1;
    }

    assert_eq!(executed, mined_edge_corpus().len());
}

struct RedisEdgeRow {
    label: &'static str,
    source: &'static str,
    input: &'static [u8],
    expected: &'static [u8],
}

fn mined_edge_corpus() -> Vec<RedisEdgeRow> {
    vec![
        RedisEdgeRow {
            label: "string-get-missing-is-null",
            source: "redis/tests/unit/type/string.tcl MGET/GET nil-shape rows",
            input: b"*2\r\n$3\r\nGET\r\n$7\r\nmissing\r\n*1\r\n$4\r\nQUIT\r\n",
            expected: b"$-1\r\n+OK\r\n",
        },
        RedisEdgeRow {
            label: "string-mget-mixes-value-and-null",
            source: "redis/tests/unit/type/string.tcl MGET against non existing key",
            input: b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n\
                     *3\r\n$4\r\nMGET\r\n$1\r\nk\r\n$7\r\nmissing\r\n\
                     *1\r\n$4\r\nQUIT\r\n",
            expected: b"+OK\r\n*2\r\n$1\r\nv\r\n$-1\r\n+OK\r\n",
        },
        RedisEdgeRow {
            label: "string-mset-duplicate-key-last-write-wins",
            source: "redis/tests/unit/type/string.tcl MSET duplicate key row",
            input: b"*5\r\n$4\r\nMSET\r\n$1\r\nx\r\n$3\r\nold\r\n$1\r\nx\r\n$3\r\nnew\r\n\
                     *2\r\n$3\r\nGET\r\n$1\r\nx\r\n\
                     *1\r\n$4\r\nQUIT\r\n",
            expected: b"+OK\r\n$3\r\nnew\r\n+OK\r\n",
        },
        RedisEdgeRow {
            label: "string-mset-wrong-arity-fails-loud",
            source: "redis/tests/unit/type/string.tcl MSET wrong number of args",
            input: b"*4\r\n$4\r\nMSET\r\n$1\r\nx\r\n$2\r\n10\r\n$1\r\ny\r\n\
                     *2\r\n$3\r\nGET\r\n$1\r\nx\r\n\
                     *1\r\n$4\r\nQUIT\r\n",
            expected: b"-ERR wrong number of arguments for 'MSET' command\r\n$-1\r\n+OK\r\n",
        },
        RedisEdgeRow {
            label: "expire-set-invalid-px-zero-fails-loud",
            source: "redis/tests/unit/expire.tcl invalid expire time in SET command",
            input: b"*5\r\n$3\r\nSET\r\n$1\r\nx\r\n$1\r\nv\r\n$2\r\nPX\r\n$1\r\n0\r\n\
                     *2\r\n$3\r\nGET\r\n$1\r\nx\r\n\
                     *1\r\n$4\r\nQUIT\r\n",
            expected: b"-ERR invalid expire time in 'set' command\r\n$-1\r\n+OK\r\n",
        },
    ]
}

fn listener() -> RedisRespServer {
    RedisRespServer::new(
        Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap()),
        RedisListenerConfig::default(),
    )
    .unwrap()
}

async fn exchange(server: &RedisRespServer, input: &'static [u8]) -> Vec<u8> {
    let (mut client, server_io) = tokio::io::duplex(4096);
    let serve = async {
        server.serve_connection(server_io).await.unwrap();
    };
    let client = async {
        client.write_all(input).await.unwrap();
        let mut output = Vec::new();
        client.read_to_end(&mut output).await.unwrap();
        output
    };
    let (_, output) = tokio::join!(serve, client);
    output
}
