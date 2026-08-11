package io.hydracache.client.hc2;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.ArrayDeque;
import java.util.List;
import java.util.Optional;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionException;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

final class RecoveringHydraCacheClientTest {
  private static final RequestOptions READ = RequestOptions.withTimeout(Duration.ofSeconds(2));
  private static final RequestOptions IDEMPOTENT =
      new RequestOptions(Duration.ofSeconds(2), bytes("request-1"));
  private static final ReconnectPolicy RECONNECT = new ReconnectPolicy(
      3, Duration.ofMillis(1), Duration.ofMillis(1), Duration.ZERO, 7);
  private static final InvocationRetryPolicy RETRY = new InvocationRetryPolicy(2);

  @Test
  void reconnectRetriesReadRepairsSubscriptionAndPermanentlyLosesSession(@TempDir Path temp)
      throws Exception {
    FakeClient first = new FakeClient(true);
    FakeClient second = new FakeClient(false);
    ScriptedConnector connector = new ScriptedConnector(first, second);
    RecoveringHydraCacheClient client = RecoveringHydraCacheClient.connect(
        List.of(endpoint(config(temp))), RECONNECT, RETRY, connector);
    AtomicInteger gaps = new AtomicInteger();
    AtomicInteger events = new AtomicInteger();
    Subscription subscription = client.subscribe(bytes("key"), 0, new CacheEventListener() {
      @Override public void onEvent(CacheEvent event) { events.incrementAndGet(); }
      @Override public void onGap(long subscriptionId, long watermark) { gaps.incrementAndGet(); }
    }).join();
    FencedSession session = client.openSession(Duration.ofSeconds(3)).join();

    Optional<CacheValue> value = client.get(bytes("key-1"), READ).join();
    assertArrayEquals(bytes("second"), value.orElseThrow().value());
    assertEquals(2, connector.attempts.get());
    assertEquals(2, client.connectionGeneration());
    assertEquals(1, gaps.get(), "reconnect must require explicit conservative repair");
    HydraCacheException lost = assertThrows(HydraCacheException.class, session::fence);
    assertEquals(ErrorCode.SESSION_LOST, lost.code());

    subscription.repair(0).join();
    second.emit(1);
    second.emit(1);
    assertEquals(1, events.get(), "duplicate watermark must not escape recovery owner");
    assertEquals(1, subscription.lastWatermark());
    RecoveryMetricsSnapshot metrics = client.recoveryMetrics();
    assertEquals(1, metrics.reconnectAttempts());
    assertEquals(1, metrics.reconnectSuccesses());
    assertEquals(1, metrics.invocationRetries());
    assertEquals(2, metrics.subscriptionRepairs());
    assertEquals(1, metrics.duplicateEvents());
    assertEquals(1, metrics.gapRepairsRequired());
    assertEquals(1, metrics.sessionLosses());
    subscription.close();
    client.close();
  }

  @Test
  void mutationReplayRequiresAnIdempotencyKey(@TempDir Path temp) throws Exception {
    FakeClient first = new FakeClient(true);
    FakeClient second = new FakeClient(false);
    ScriptedConnector connector = new ScriptedConnector(first, second);
    RecoveringHydraCacheClient client = RecoveringHydraCacheClient.connect(
        List.of(endpoint(config(temp))), RECONNECT, RETRY, connector);

    CompletionException unsafe = assertThrows(CompletionException.class,
        () -> client.put(bytes("key"), bytes("value"), Duration.ZERO, READ).join());
    assertEquals(RetryAdvice.RECONNECT_IDEMPOTENT,
        ((HydraCacheException) unsafe.getCause()).retryAdvice());
    assertEquals(1, connector.attempts.get(), "unsafe mutation must not reconnect or replay");

    assertTrue(client.put(bytes("key"), bytes("value"), Duration.ZERO, IDEMPOTENT)
        .join().applied());
    assertEquals(2, connector.attempts.get());
    assertEquals(1, client.recoveryMetrics().invocationRetries());
    client.close();
  }

  @Test
  void staleCompletionFromAReplacedGenerationIsDiscardedAndReadIsReplayed(@TempDir Path temp)
      throws Exception {
    CompletableFuture<Optional<CacheValue>> stale = new CompletableFuture<>();
    FakeClient first = new FakeClient(false) {
      @Override public CompletableFuture<Optional<CacheValue>> get(
          byte[] key, RequestOptions options) {
        return stale;
      }
    };
    FakeClient second = new FakeClient(false);
    ScriptedConnector connector = new ScriptedConnector(first, second);
    RecoveringHydraCacheClient client = RecoveringHydraCacheClient.connect(
        List.of(endpoint(config(temp))), RECONNECT, RETRY, connector);

    CompletableFuture<Optional<CacheValue>> pending = client.get(bytes("key"), READ);
    client.reconnectNow().join();
    stale.complete(Optional.of(new CacheValue(bytes("stale"), Optional.empty())));

    assertArrayEquals(bytes("second"), pending.join().orElseThrow().value());
    assertEquals(2, connector.attempts.get());
    assertEquals(1, client.recoveryMetrics().invocationRetries());
    client.close();
  }

  @Test
  void securityFailureStopsEndpointFallbackAndCountsExhaustion(@TempDir Path temp)
      throws Exception {
    FakeClient first = new FakeClient(true);
    HydraCacheException security = new HydraCacheException(
        ErrorCode.UNAUTHENTICATED, RetryAdvice.NEVER, "certificate rejected");
    ScriptedConnector connector = new ScriptedConnector(first, security, new FakeClient(false));
    RecoveringHydraCacheClient client = RecoveringHydraCacheClient.connect(
        List.of(endpoint(config(temp)), endpoint(config(temp))), RECONNECT, RETRY, connector);

    CompletionException failure = assertThrows(CompletionException.class,
        () -> client.get(bytes("key"), READ).join());
    assertEquals(ErrorCode.UNAUTHENTICATED, ((HydraCacheException) failure.getCause()).code());
    assertEquals(2, connector.attempts.get(), "security failure must stop ordered fallback");
    assertEquals(1, client.recoveryMetrics().reconnectAttempts());
    assertEquals(1, client.recoveryMetrics().reconnectExhausted());
    client.close();
  }

  @Test
  void reconnectRejectsAValidClientForAnotherCluster(@TempDir Path temp) throws Exception {
    FakeClient first = new FakeClient(true, "cluster-a");
    FakeClient foreign = new FakeClient(false, "cluster-b");
    ScriptedConnector connector = new ScriptedConnector(first, foreign, new FakeClient(false));
    RecoveringHydraCacheClient client = RecoveringHydraCacheClient.connect(
        List.of(endpoint(config(temp)), endpoint(config(temp))), RECONNECT, RETRY, connector);

    CompletionException failure = assertThrows(CompletionException.class,
        () -> client.get(bytes("key"), READ).join());
    assertEquals(ErrorCode.UNAUTHENTICATED, ((HydraCacheException) failure.getCause()).code());
    assertEquals(2, connector.attempts.get(), "cluster mismatch must stop ordered fallback");
    client.close();
  }

  @Test
  void policiesAreBounded() {
    assertThrows(IllegalArgumentException.class, () -> new InvocationRetryPolicy(0));
    assertThrows(IllegalArgumentException.class, () -> new InvocationRetryPolicy(5));
    assertThrows(IllegalArgumentException.class, () -> new ReconnectPolicy(
        33, Duration.ofMillis(1), Duration.ofMillis(1), Duration.ZERO, 1));
  }

  private static HydraCacheClientConfig config(Path temp) throws Exception {
    Path certificate = temp.resolve("ca.pem");
    Path clientCertificate = temp.resolve("client.pem");
    Path key = temp.resolve("client.key");
    if (!Files.exists(certificate)) Files.writeString(certificate, "fixture");
    if (!Files.exists(clientCertificate)) Files.writeString(clientCertificate, "fixture");
    if (!Files.exists(key)) Files.writeString(key, "fixture");
    return HydraCacheClientConfig.builder()
        .endpoint(URI.create("https://localhost:19443"))
        .serverName("localhost")
        .trustCertificate(certificate)
        .clientCertificate(clientCertificate)
        .clientPrivateKey(key)
        .callbackExecutor(Runnable::run)
        .build();
  }

  private static ReconnectEndpoint endpoint(HydraCacheClientConfig config) {
    return new ReconnectEndpoint("endpoint-" + System.identityHashCode(config), config);
  }

  private static byte[] bytes(String value) { return value.getBytes(StandardCharsets.UTF_8); }

  private static final class ScriptedConnector
      implements RecoveringHydraCacheClient.ClientConnector {
    private final ArrayDeque<Object> outcomes = new ArrayDeque<>();
    final AtomicInteger attempts = new AtomicInteger();

    ScriptedConnector(Object... outcomes) { this.outcomes.addAll(List.of(outcomes)); }

    @Override public synchronized HydraCacheClient connect(HydraCacheClientConfig ignored) {
      attempts.incrementAndGet();
      Object outcome = outcomes.removeFirst();
      if (outcome instanceof RuntimeException failure) throw failure;
      return (HydraCacheClient) outcome;
    }
  }

  private static class FakeClient implements HydraCacheClient {
    private final boolean failOperations;
    private volatile CacheEventListener listener;
    private final AtomicInteger subscriptionIds = new AtomicInteger();

    private final String clusterId;

    FakeClient(boolean failOperations) { this(failOperations, "cluster-a"); }

    FakeClient(boolean failOperations, String clusterId) {
      this.failOperations = failOperations;
      this.clusterId = clusterId;
    }

    @Override public CompletableFuture<Optional<CacheValue>> get(byte[] key, RequestOptions options) {
      if (failOperations) return unavailable();
      return CompletableFuture.completedFuture(Optional.of(
          new CacheValue(bytes("second"), Optional.empty())));
    }

    @Override public CompletableFuture<MutationResult> put(
        byte[] key, byte[] value, Duration ttl, RequestOptions options) {
      return failOperations ? unavailable()
          : CompletableFuture.completedFuture(new MutationResult(true));
    }

    @Override public CompletableFuture<MutationResult> delete(byte[] key, RequestOptions options) {
      return put(key, new byte[0], Duration.ZERO, options);
    }

    @Override public CompletableFuture<MutationResult> compareAndSet(
        byte[] key, byte[] expected, byte[] replacement, Duration ttl, RequestOptions options) {
      return put(key, replacement, ttl, options);
    }

    @Override public CompletableFuture<List<BatchItemResult>> batch(
        List<BatchOperation> operations, RequestOptions options) {
      return failOperations ? unavailable() : CompletableFuture.completedFuture(List.of());
    }

    @Override public CompletableFuture<Subscription> subscribe(
        byte[] keyPrefix, long resumeWatermark, CacheEventListener listener) {
      this.listener = listener;
      long id = subscriptionIds.incrementAndGet();
      return CompletableFuture.completedFuture(new Subscription() {
        @Override public long id() { return id; }
        @Override public long initialWatermark() { return resumeWatermark; }
        @Override public void close() { }
      });
    }

    @Override public CompletableFuture<FencedSession> openSession(Duration requestedTtl) {
      return CompletableFuture.completedFuture(new FencedSession() {
        @Override public byte[] id() { return bytes("session"); }
        @Override public long fence() { return 7; }
        @Override public void close() { }
      });
    }

    void emit(long watermark) {
      listener.onEvent(new CacheEvent(1, watermark, bytes("key"), bytes("value"), false));
    }

    @Override public TopologySnapshot topology() { return TopologySnapshot.empty(); }
    @Override public String clusterId() { return clusterId; }
    @Override public ClientMetricsSnapshot metrics() {
      return new ClientMetricsSnapshot(0, 0, 0, 0, 0, 0, 0, 0, 0);
    }
    @Override public void close() { }

    private static <T> CompletableFuture<T> unavailable() {
      return CompletableFuture.failedFuture(new HydraCacheException(
          ErrorCode.UNAVAILABLE, RetryAdvice.RECONNECT_IDEMPOTENT, "scripted reset"));
    }
  }
}
