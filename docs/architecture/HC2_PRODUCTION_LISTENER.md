# HC/2 Production Listener Integration

Status: H01 production integration complete and off by default. ADR-0019/H03
accepts bidirectional gRPC over mandatory mTLS as the first HC/2 adapter.

## Scope

`hydracache-server` now owns a real HC/2 listener instead of importing or
starting the non-published transport spike. The listener uses the generated
HC/2 schema and the production `Hc2ClientPlaneService`, while HC/1 and RESP
continue to be separate protocol surfaces.

All enabled daemon sockets are bound before any listener starts accepting.
HC/2 TLS files are read and parsed before readiness is printed. A port
conflict, missing certificate, malformed key, or invalid client CA therefore
terminates startup without leaving HC/1, RESP, or the admin surface partially
available.

## Configuration

HC/2 is disabled by default. Enabling it requires complete global TLS material:

| Environment variable | Required | Meaning |
| --- | --- | --- |
| `HYDRACACHE_HC2_ENABLED=true` | yes | enable the dedicated HC/2 listener |
| `HYDRACACHE_HC2_ADDR` | no | bind address; default `127.0.0.1:9443` |
| `HYDRACACHE_HC2_CLUSTER_ID` | no | non-empty handshake cluster identity; default `hydracache-local` |
| `HYDRACACHE_TLS_ENABLED=true` | yes | activate TLS policy |
| `HYDRACACHE_TLS_CERT_PATH` | yes | PEM server certificate chain |
| `HYDRACACHE_TLS_KEY_PATH` | yes | PEM server private key |
| `HYDRACACHE_TLS_CA_PATH` | yes | PEM trust roots for mandatory client certificates |

The HC/2 address must differ from the HC/1, cluster, enabled admin, and enabled
RESP addresses. Non-loopback exposure remains subject to the server-wide TLS
startup policy. There is no plaintext HC/2 constructor or permissive fallback.

## Identity and dispatch boundary

The service derives `client_id` from the SHA-256 fingerprint of the client
certificate verified by the TLS channel. A client-supplied handshake name is
not an authority. The request tenant is validated and combined with that
verified peer identity before dispatch.

Get, put, delete, compare-and-set, and bounded batch requests are translated
to `ClientRequestEnvelope` and executed through the existing
`ClientSurfaceState::dispatch_verified_request` path. This reuses the HC/1/RESP
tenant, quota, deadline, idempotency, audit, and core-cache boundary instead of
creating a second cache implementation. When HC/1 and HC/2 are enabled in the
standalone daemon, both receive the same `Arc<ClientSurfaceState>`.

Mutation events are emitted only after dispatch returns an applied mutation.
A rejected put or compare-and-set mismatch cannot manufacture a cache event.

## Lifecycle and bounded ownership

The production service accounts for authenticated connections, subscriptions,
fenced sessions, pending invocations, and pre-dispatch rejections. Connection
and stream resource guards release counters on normal close, protocol error,
client reset, outbound cancellation, and task cancellation; cleanup does not
depend on cooperative unsubscribe/session-close frames.

On drain the daemon signals all client listeners, waits up to the configured
drain timeout, aborts listener tasks after the deadline, shuts down the shared
runtime, and fails loudly if HC/2 retains connection, subscription, session, or
invocation ownership. Listener failures are returned to the process instead of
being printed while an apparently healthy daemon continues running.

The existing internal `/metrics` endpoint appends aggregate production-listener
gauges for connections, pending invocations, subscriptions, and sessions plus
the pre-dispatch rejection counter. Every series has only the closed
`transport="grpc_bidirectional"` label. Per-client identities, tenant IDs,
authorities, certificate material, keys, and values are never exported. The
larger per-connection H21 diagnostic schema remains available to SDK and
telemetry adapters; it is not copied into the daemon or exposed on a public
client port.

## Evidence

The in-process socket test proves:

- plaintext HTTP/2 is rejected before dispatch;
- a client certificate signed by a foreign CA is rejected;
- negotiation uses the generated HC/2 generation/capability schema;
- put/get use production dispatch, a matching subscription receives an event,
  and an active fenced session is accounted;
- abrupt transport close releases every connection-owned resource.

The real-process test starts `hydracache-server` from environment
configuration and proves:

- readiness follows successful bind and TLS preflight;
- HC/1 writes are visible to HC/2 and HC/2 writes are visible to HC/1;
- the internal metrics endpoint reports the active HC/2 connection and zero
  pending work without exposing tenant, client, authority, or key material;
- admin drain terminates the daemon successfully;
- an HC/2 port conflict and unreadable TLS material fail before any listener
  or readiness signal is exposed.

Run the focused evidence with:

```powershell
cargo test -p hydracache-server --lib hc2::tests --locked
cargo test -p hydracache-server --test hc2_daemon_process --locked
cargo test -p hydracache-server --test server_lifecycle --locked
cargo clippy -p hydracache-server --all-targets --locked -- -D warnings
```

`cargo xtask client-plane-spike-check` and the Docker interop form run the
production listener tests as part of the HC/2 evidence workflow. These are
correctness and lifecycle gates, not performance or release-readiness claims.

## Remaining release boundary

H01 is complete. H16-H18 separately own Java/Rust recovery and retained
old/new compatibility evidence. The production metric mount is operational
correctness evidence, not a latency, throughput, capacity, or availability
claim.
