package io.hydracache.client.hc2;

import io.hydracache.client.hc2.internal.GrpcHydraCacheClient;
import java.time.Duration;
import java.util.List;
import java.util.Optional;
import java.util.concurrent.CompletableFuture;

/** Thread-safe preview HC/2 client. Generated wire types never appear in this API. */
public interface HydraCacheClient extends AutoCloseable {
  static HydraCacheClient connect(HydraCacheClientConfig config) {
    return GrpcHydraCacheClient.connect(config);
  }

  CompletableFuture<Optional<CacheValue>> get(byte[] key, RequestOptions options);
  CompletableFuture<MutationResult> put(byte[] key, byte[] value, Duration ttl, RequestOptions options);
  CompletableFuture<MutationResult> delete(byte[] key, RequestOptions options);
  CompletableFuture<MutationResult> compareAndSet(
      byte[] key, byte[] expected, byte[] replacement, Duration ttl, RequestOptions options);
  CompletableFuture<List<BatchItemResult>> batch(List<BatchOperation> operations, RequestOptions options);
  CompletableFuture<Subscription> subscribe(byte[] keyPrefix, long resumeWatermark, CacheEventListener listener);
  CompletableFuture<FencedSession> openSession(Duration requestedTtl);
  /** Authenticated cluster identity negotiated during the HC/2 handshake. */
  String clusterId();
  /** Exact HC/2 wire generation selected for this connection. */
  default int protocolGeneration() {
    return HydraCacheClientConfig.CURRENT_PROTOCOL_GENERATION;
  }
  /** Preferred generation advertised by the peer, or the selected generation for a legacy peer. */
  default int preferredProtocolGeneration() { return protocolGeneration(); }
  /** Whether this connection is using a generation inside its deprecation window. */
  default boolean negotiatedGenerationDeprecated() { return false; }
  TopologySnapshot topology();
  ClientMetricsSnapshot metrics();
  @Override void close();
}
