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
  TopologySnapshot topology();
  ClientMetricsSnapshot metrics();
  @Override void close();
}
