package io.hydracache.client.hc2;

import java.time.Duration;
import java.util.ArrayList;
import java.util.List;
import java.util.Objects;
import java.util.Optional;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionException;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;
import java.util.concurrent.atomic.AtomicReference;
import java.util.concurrent.atomic.LongAdder;
import java.util.function.BiFunction;

/**
 * Recovery owner for the preview HC/2 Java SDK.
 *
 * <p>Connection establishment and invocation replay have separate bounded policies. Reads may be
 * replayed after an authenticated stream failure. Mutations and mutating batches require a nonempty
 * idempotency key. A reconnect emits an explicit subscription repair boundary and permanently loses
 * every fenced session; it never silently reacquires a lock/session.</p>
 */
public final class RecoveringHydraCacheClient implements HydraCacheClient {
  @FunctionalInterface
  interface ClientConnector {
    HydraCacheClient connect(HydraCacheClientConfig config);
  }

  private record Connection(long generation, int endpointIndex, HydraCacheClient client) {}

  private final List<ReconnectEndpoint> endpoints;
  private final ReconnectPolicy reconnectPolicy;
  private final InvocationRetryPolicy retryPolicy;
  private final ClientConnector connector;
  private final String clusterId;
  private final ExecutorService recoveryExecutor;
  private final Object reconnectLock = new Object();
  private final AtomicReference<Connection> current;
  private final AtomicBoolean closed = new AtomicBoolean();
  private final AtomicLong subscriptionIds = new AtomicLong();
  private final AtomicLong sessionIds = new AtomicLong();
  private final AtomicLong preferredEndpoint = new AtomicLong();
  private final ConcurrentHashMap<Long, LogicalSubscription> subscriptions = new ConcurrentHashMap<>();
  private final ConcurrentHashMap<Long, LogicalSession> sessions = new ConcurrentHashMap<>();
  private final RecoveryMetrics recoveryMetrics = new RecoveryMetrics();

  /** Connect through the ordered, bounded, fail-closed endpoint policy. */
  public static RecoveringHydraCacheClient connect(
      List<ReconnectEndpoint> endpoints,
      ReconnectPolicy reconnectPolicy,
      InvocationRetryPolicy retryPolicy) {
    return connect(endpoints, reconnectPolicy, retryPolicy, HydraCacheClient::connect);
  }

  static RecoveringHydraCacheClient connect(
      List<ReconnectEndpoint> endpoints,
      ReconnectPolicy reconnectPolicy,
      InvocationRetryPolicy retryPolicy,
      ClientConnector connector) {
    List<ReconnectEndpoint> checked = validateEndpoints(endpoints);
    Objects.requireNonNull(reconnectPolicy, "reconnectPolicy");
    Objects.requireNonNull(retryPolicy, "retryPolicy");
    Objects.requireNonNull(connector, "connector");
    RecoveryMetrics initialMetrics = new RecoveryMetrics();
    Connection initial = establish(
        checked, reconnectPolicy, connector, 0, 1, initialMetrics, false, null);
    return new RecoveringHydraCacheClient(
        checked, reconnectPolicy, retryPolicy, connector, initial, initialMetrics);
  }

  private RecoveringHydraCacheClient(
      List<ReconnectEndpoint> endpoints,
      ReconnectPolicy reconnectPolicy,
      InvocationRetryPolicy retryPolicy,
      ClientConnector connector,
      Connection initial,
      RecoveryMetrics initialMetrics) {
    this.endpoints = endpoints;
    this.reconnectPolicy = reconnectPolicy;
    this.retryPolicy = retryPolicy;
    this.connector = connector;
    this.current = new AtomicReference<>(initial);
    this.clusterId = requireClusterId(initial.client().clusterId());
    this.preferredEndpoint.set(initial.endpointIndex());
    this.recoveryMetrics.copyFrom(initialMetrics);
    this.recoveryExecutor = Executors.newSingleThreadExecutor(runnable -> {
      Thread thread = new Thread(runnable, "hydracache-hc2-recovery");
      thread.setDaemon(true);
      return thread;
    });
  }

  /** Current privacy-safe endpoint identity. */
  public String currentEndpoint() { return endpoints.get(current.get().endpointIndex()).id(); }

  /** Current logical nonzero connection generation. */
  public long connectionGeneration() { return current.get().generation(); }

  /** Prefer an already configured endpoint for the next reconnect. */
  public void preferEndpoint(String endpointId) {
    for (int index = 0; index < endpoints.size(); index++) {
      if (endpoints.get(index).id().equals(endpointId)) {
        preferredEndpoint.set(index);
        return;
      }
    }
    throw local(ErrorCode.INVALID_REQUEST, RetryAdvice.NEVER,
        "leader hint names an unknown endpoint");
  }

  /** Explicitly replace the current connection without replaying an operation. */
  public CompletableFuture<Void> reconnectNow() {
    return reconnectIfGeneration(connectionGeneration());
  }

  /** Pull bounded, privacy-safe reconnect and repair counters. */
  public RecoveryMetricsSnapshot recoveryMetrics() { return recoveryMetrics.snapshot(); }

  @Override
  public CompletableFuture<Optional<CacheValue>> get(byte[] key, RequestOptions options) {
    byte[] copied = key.clone();
    return withRetry(true, options, (client, adjusted) -> client.get(copied, adjusted));
  }

  @Override
  public CompletableFuture<MutationResult> put(
      byte[] key, byte[] value, Duration ttl, RequestOptions options) {
    byte[] copiedKey = key.clone();
    byte[] copiedValue = value.clone();
    return withRetry(replaySafe(options), options,
        (client, adjusted) -> client.put(copiedKey, copiedValue, ttl, adjusted));
  }

  @Override
  public CompletableFuture<MutationResult> delete(byte[] key, RequestOptions options) {
    byte[] copied = key.clone();
    return withRetry(replaySafe(options), options,
        (client, adjusted) -> client.delete(copied, adjusted));
  }

  @Override
  public CompletableFuture<MutationResult> compareAndSet(
      byte[] key, byte[] expected, byte[] replacement, Duration ttl, RequestOptions options) {
    byte[] copiedKey = key.clone();
    byte[] copiedExpected = expected.clone();
    byte[] copiedReplacement = replacement.clone();
    return withRetry(replaySafe(options), options, (client, adjusted) -> client.compareAndSet(
        copiedKey, copiedExpected, copiedReplacement, ttl, adjusted));
  }

  @Override
  public CompletableFuture<List<BatchItemResult>> batch(
      List<BatchOperation> operations, RequestOptions options) {
    List<BatchOperation> copied = List.copyOf(operations);
    boolean readOnly = copied.stream().allMatch(BatchOperation.Get.class::isInstance);
    return withRetry(readOnly || replaySafe(options), options,
        (client, adjusted) -> client.batch(copied, adjusted));
  }

  @Override
  public CompletableFuture<Subscription> subscribe(
      byte[] keyPrefix, long resumeWatermark, CacheEventListener listener) {
    if (closed.get()) return failed(connectionError("recovering client is closed"));
    Objects.requireNonNull(listener, "listener");
    long id = nextNonzero(subscriptionIds, "logical subscription id exhausted");
    LogicalSubscription logical = new LogicalSubscription(
        id, keyPrefix.clone(), resumeWatermark, listener);
    subscriptions.put(id, logical);
    return logical.bind(current.get(), false).handle((ignored, failure) -> {
      if (failure != null) {
        subscriptions.remove(id, logical);
        logical.closeInternal();
        throw propagate(failure);
      }
      return (Subscription) logical;
    });
  }

  @Override
  public CompletableFuture<FencedSession> openSession(Duration requestedTtl) {
    if (closed.get()) return failed(connectionError("recovering client is closed"));
    Connection observed = current.get();
    return observed.client().openSession(requestedTtl).thenApply(delegate -> {
      if (current.get().generation() != observed.generation() || closed.get()) {
        delegate.close();
        throw sessionError("session connection changed during open");
      }
      long id = nextNonzero(sessionIds, "logical session id exhausted");
      LogicalSession logical = new LogicalSession(id, delegate);
      sessions.put(id, logical);
      return (FencedSession) logical;
    });
  }

  @Override public TopologySnapshot topology() { return current.get().client().topology(); }

  @Override public String clusterId() { return clusterId; }

  @Override public ClientMetricsSnapshot metrics() { return current.get().client().metrics(); }

  @Override
  public void close() {
    if (!closed.compareAndSet(false, true)) return;
    subscriptions.values().forEach(LogicalSubscription::closeInternal);
    subscriptions.clear();
    sessions.values().forEach(LogicalSession::markLost);
    sessions.clear();
    current.get().client().close();
    recoveryExecutor.shutdownNow();
  }

  private <T> CompletableFuture<T> withRetry(
      boolean retrySafe,
      RequestOptions options,
      BiFunction<HydraCacheClient, RequestOptions, CompletableFuture<T>> operation) {
    Objects.requireNonNull(options, "options");
    long deadline = saturatedAdd(System.nanoTime(), options.timeout().toNanos());
    return attempt(retrySafe, options, operation, deadline, 1);
  }

  private <T> CompletableFuture<T> attempt(
      boolean retrySafe,
      RequestOptions options,
      BiFunction<HydraCacheClient, RequestOptions, CompletableFuture<T>> operation,
      long deadline,
      int attempt) {
    if (closed.get()) return failed(connectionError("recovering client is closed"));
    long remaining = deadline - System.nanoTime();
    if (remaining <= 0) return failed(deadlineError());
    Connection observed = current.get();
    RequestOptions adjusted = new RequestOptions(Duration.ofNanos(remaining), options.idempotencyKey());
    CompletableFuture<T> invoked;
    try {
      invoked = operation.apply(observed.client(), adjusted);
    } catch (RuntimeException error) {
      invoked = failed(error);
    }
    return invoked.handle((value, failure) -> {
      Throwable cause = failure == null ? null : unwrap(failure);
      if (cause == null && current.get().generation() != observed.generation()) {
        cause = connectionError("stale completion from a replaced connection");
      }
      if (cause == null) return CompletableFuture.completedFuture(value);
      if (!retrySafe || !reconnectable(cause) || attempt >= retryPolicy.maxAttempts()) {
        return RecoveringHydraCacheClient.<T>failed(cause);
      }
      recoveryMetrics.invocationRetries.increment();
      return reconnectIfGeneration(observed.generation())
          .thenCompose(ignored -> attempt(retrySafe, options, operation, deadline, attempt + 1));
    }).thenCompose(next -> next);
  }

  private CompletableFuture<Void> reconnectIfGeneration(long observedGeneration) {
    if (closed.get()) return failed(connectionError("recovering client is closed"));
    return CompletableFuture.runAsync(() -> {
      synchronized (reconnectLock) {
        if (closed.get()) throw connectionError("recovering client is closed");
        Connection previous = current.get();
        if (previous.generation() != observedGeneration) return;
        sessions.values().forEach(LogicalSession::markLost);
        sessions.clear();
        long generation;
        try { generation = Math.incrementExact(observedGeneration); }
        catch (ArithmeticException exhausted) {
          throw local(ErrorCode.UNSUPPORTED, RetryAdvice.NEVER,
              "connection generation exhausted");
        }
        Connection next;
        try {
          next = establish(endpoints, reconnectPolicy, connector,
              Math.toIntExact(preferredEndpoint.get()), generation, recoveryMetrics, true,
              clusterId);
        } catch (RuntimeException error) {
          recoveryMetrics.reconnectExhausted.increment();
          subscriptions.values().forEach(LogicalSubscription::requireRepair);
          throw error;
        }
        if (closed.get()) {
          next.client().close();
          throw connectionError("recovering client closed during reconnect");
        }
        current.set(next);
        preferredEndpoint.set(next.endpointIndex());
        previous.client().close();
        recoveryMetrics.reconnectSuccesses.increment();
        repairSubscriptions(next);
      }
    }, recoveryExecutor);
  }

  private void repairSubscriptions(Connection connection) {
    for (LogicalSubscription subscription : new ArrayList<>(subscriptions.values())) {
      if (subscription.closed.get()) continue;
      subscription.requireRepair();
      try {
        subscription.bind(connection, true).join();
      } catch (CompletionException failure) {
        subscription.requireRepair();
      }
    }
  }

  private static Connection establish(
      List<ReconnectEndpoint> endpoints,
      ReconnectPolicy policy,
      ClientConnector connector,
      int start,
      long generation,
      RecoveryMetrics metrics,
      boolean reconnect,
      String expectedClusterId) {
    RuntimeException last = connectionError("no reconnect attempt executed");
    for (int attempt = 1; attempt <= policy.maxAttempts(); attempt++) {
      if (attempt > 1) sleep(policy.delay(attempt - 1));
      int endpointIndex = Math.floorMod(start + attempt - 1, endpoints.size());
      if (reconnect) metrics.reconnectAttempts.increment();
      try {
        HydraCacheClient client = connector.connect(endpoints.get(endpointIndex).config());
        String actualClusterId = requireClusterId(client.clusterId());
        if (expectedClusterId != null && !expectedClusterId.equals(actualClusterId)) {
          client.close();
          throw local(ErrorCode.UNAUTHENTICATED, RetryAdvice.NEVER,
              "reconnect endpoint belongs to another cluster");
        }
        return new Connection(generation, endpointIndex, client);
      } catch (RuntimeException error) {
        last = endpointFailure(endpoints.get(endpointIndex), error);
        if (!reconnectable(error)) throw last;
      }
    }
    throw last;
  }

  private static HydraCacheException endpointFailure(
      ReconnectEndpoint endpoint, RuntimeException failure) {
    if (failure instanceof HydraCacheException hc) {
      return new HydraCacheException(hc.code(), hc.retryAdvice(),
          "HC/2 endpoint '" + endpoint.id() + "' failed", failure);
    }
    return new HydraCacheException(ErrorCode.UNAVAILABLE, RetryAdvice.NEVER,
        "HC/2 endpoint '" + endpoint.id() + "' failed", failure);
  }

  private final class LogicalSubscription implements Subscription {
    private final long id;
    private final byte[] keyPrefix;
    private final long initialWatermark;
    private final CacheEventListener listener;
    private final AtomicLong lastWatermark;
    private final AtomicLong binding = new AtomicLong();
    private final AtomicBoolean repairRequired = new AtomicBoolean();
    private final AtomicBoolean closed = new AtomicBoolean();
    private final AtomicReference<Subscription> delegate = new AtomicReference<>();

    private LogicalSubscription(
        long id, byte[] keyPrefix, long resumeWatermark, CacheEventListener listener) {
      this.id = id;
      this.keyPrefix = keyPrefix;
      this.initialWatermark = resumeWatermark;
      this.lastWatermark = new AtomicLong(resumeWatermark);
      this.listener = listener;
    }

    CompletableFuture<Void> bind(Connection connection, boolean repair) {
      long bindingId = binding.incrementAndGet();
      CacheEventListener forwarding = new CacheEventListener() {
        @Override public void onEvent(CacheEvent event) {
          if (binding.get() != bindingId || closed.get() || repairRequired.get()) return;
          long previous = lastWatermark.get();
          if (event.watermark() <= previous) {
            recoveryMetrics.duplicateEvents.increment();
            return;
          }
          if (previous != 0 && event.watermark() != previous + 1) {
            requireRepair();
            return;
          }
          lastWatermark.set(event.watermark());
          listener.onEvent(new CacheEvent(id, event.watermark(), event.key(), event.value(), event.removed()));
        }

        @Override public void onGap(long ignoredId, long afterWatermark) { requireRepair(); }
      };
      return connection.client().subscribe(keyPrefix, lastWatermark.get(), forwarding)
          .thenAccept(next -> {
            if (closed.get() || current.get().generation() != connection.generation()) {
              next.close();
              throw connectionError("connection changed during subscription repair");
            }
            Subscription previous = delegate.getAndSet(next);
            if (previous != null) previous.close();
            if (repair) recoveryMetrics.subscriptionRepairs.increment();
          });
    }

    void requireRepair() {
      if (!repairRequired.compareAndSet(false, true) || closed.get()) return;
      recoveryMetrics.gapRepairsRequired.increment();
      try {
        endpoints.get(current.get().endpointIndex()).config().callbackExecutor()
            .execute(() -> listener.onGap(id, lastWatermark.get()));
      } catch (RuntimeException ignored) { }
    }

    @Override public long id() { return id; }
    @Override public long initialWatermark() { return initialWatermark; }
    @Override public long lastWatermark() { return lastWatermark.get(); }

    @Override
    public CompletableFuture<Void> repair(long watermark) {
      if (watermark < lastWatermark.get()) {
        return failed(local(ErrorCode.INVALID_REQUEST, RetryAdvice.NEVER,
            "repair watermark cannot move backwards"));
      }
      if (closed.get()) return failed(connectionError("subscription is closed"));
      lastWatermark.set(watermark);
      repairRequired.set(false);
      Subscription previous = delegate.getAndSet(null);
      if (previous != null) previous.close();
      return bind(current.get(), true).exceptionally(failure -> {
        requireRepair();
        throw propagate(failure);
      });
    }

    @Override public void close() {
      subscriptions.remove(id, this);
      closeInternal();
    }

    void closeInternal() {
      if (!closed.compareAndSet(false, true)) return;
      binding.incrementAndGet();
      Subscription previous = delegate.getAndSet(null);
      if (previous != null) previous.close();
    }
  }

  private final class LogicalSession implements FencedSession {
    private final long logicalId;
    private final AtomicReference<FencedSession> delegate;
    private final AtomicBoolean lost = new AtomicBoolean();
    private final AtomicBoolean sessionClosed = new AtomicBoolean();

    LogicalSession(long logicalId, FencedSession delegate) {
      this.logicalId = logicalId;
      this.delegate = new AtomicReference<>(delegate);
    }

    void markLost() {
      if (!lost.compareAndSet(false, true)) return;
      FencedSession previous = delegate.getAndSet(null);
      if (previous != null) previous.close();
      recoveryMetrics.sessionLosses.increment();
    }

    @Override public byte[] id() {
      FencedSession active = active();
      return active.id();
    }

    @Override public long fence() { return active().fence(); }

    private FencedSession active() {
      FencedSession active = delegate.get();
      if (lost.get() || sessionClosed.get() || active == null) {
        throw sessionError("fenced session was lost during reconnect");
      }
      return active;
    }

    @Override public void close() {
      if (!sessionClosed.compareAndSet(false, true)) return;
      sessions.remove(logicalId, this);
      FencedSession previous = delegate.getAndSet(null);
      if (previous != null) previous.close();
    }
  }

  private static final class RecoveryMetrics {
    final LongAdder reconnectAttempts = new LongAdder();
    final LongAdder reconnectSuccesses = new LongAdder();
    final LongAdder reconnectExhausted = new LongAdder();
    final LongAdder invocationRetries = new LongAdder();
    final LongAdder subscriptionRepairs = new LongAdder();
    final LongAdder duplicateEvents = new LongAdder();
    final LongAdder gapRepairsRequired = new LongAdder();
    final LongAdder sessionLosses = new LongAdder();

    void copyFrom(RecoveryMetrics other) {
      reconnectAttempts.add(other.reconnectAttempts.sum());
      reconnectSuccesses.add(other.reconnectSuccesses.sum());
      reconnectExhausted.add(other.reconnectExhausted.sum());
      invocationRetries.add(other.invocationRetries.sum());
      subscriptionRepairs.add(other.subscriptionRepairs.sum());
      duplicateEvents.add(other.duplicateEvents.sum());
      gapRepairsRequired.add(other.gapRepairsRequired.sum());
      sessionLosses.add(other.sessionLosses.sum());
    }

    RecoveryMetricsSnapshot snapshot() {
      return new RecoveryMetricsSnapshot(reconnectAttempts.sum(), reconnectSuccesses.sum(),
          reconnectExhausted.sum(), invocationRetries.sum(), subscriptionRepairs.sum(),
          duplicateEvents.sum(), gapRepairsRequired.sum(), sessionLosses.sum());
    }
  }

  private static List<ReconnectEndpoint> validateEndpoints(List<ReconnectEndpoint> endpoints) {
    Objects.requireNonNull(endpoints, "endpoints");
    if (endpoints.isEmpty() || endpoints.size() > 32) {
      throw new IllegalArgumentException("endpoint count must be in [1, 32]");
    }
    List<ReconnectEndpoint> checked = List.copyOf(endpoints);
    if (checked.stream().map(ReconnectEndpoint::id).distinct().count() != checked.size()) {
      throw new IllegalArgumentException("endpoint identities must be unique");
    }
    HydraCacheClientConfig identity = checked.get(0).config();
    for (ReconnectEndpoint endpoint : checked) {
      HydraCacheClientConfig config = endpoint.config();
      if (!config.clientId().equals(identity.clientId()) || !config.tenant().equals(identity.tenant())
          || !config.requestedCapabilities().equals(identity.requestedCapabilities())
          || config.transportPolicy() != identity.transportPolicy()) {
        throw new IllegalArgumentException(
            "all reconnect endpoints must preserve client, tenant, capability, and transport identity");
      }
    }
    return checked;
  }

  private static String requireClusterId(String value) {
    if (value == null || value.isBlank() || value.length() > 128
        || value.chars().anyMatch(Character::isISOControl)) {
      throw local(ErrorCode.UNAUTHENTICATED, RetryAdvice.NEVER,
          "peer returned an invalid cluster identity");
    }
    return value;
  }

  private static boolean replaySafe(RequestOptions options) {
    return options != null && options.idempotencyKey().length != 0;
  }

  private static boolean reconnectable(Throwable failure) {
    Throwable cause = unwrap(failure);
    return cause instanceof HydraCacheException error
        && error.retryAdvice() == RetryAdvice.RECONNECT_IDEMPOTENT;
  }

  private static Throwable unwrap(Throwable failure) {
    Throwable current = failure;
    while ((current instanceof CompletionException
        || current instanceof java.util.concurrent.ExecutionException)
        && current.getCause() != null) {
      current = current.getCause();
    }
    return current;
  }

  private static CompletionException propagate(Throwable failure) {
    Throwable cause = unwrap(failure);
    return cause instanceof CompletionException completion
        ? completion : new CompletionException(cause);
  }

  private static void sleep(Duration duration) {
    try { TimeUnit.NANOSECONDS.sleep(duration.toNanos()); }
    catch (InterruptedException interrupted) {
      Thread.currentThread().interrupt();
      throw connectionError("reconnect interrupted");
    }
  }

  private static long saturatedAdd(long left, long right) {
    try { return Math.addExact(left, right); }
    catch (ArithmeticException overflow) { return Long.MAX_VALUE; }
  }

  private static long nextNonzero(AtomicLong sequence, String detail) {
    long value = sequence.incrementAndGet();
    if (value <= 0) throw local(ErrorCode.INTERNAL, RetryAdvice.NEVER, detail);
    return value;
  }

  private static HydraCacheException deadlineError() {
    return local(ErrorCode.DEADLINE_EXCEEDED, RetryAdvice.NEVER,
        "operation deadline exhausted during recovery");
  }

  private static HydraCacheException connectionError(String detail) {
    return local(ErrorCode.UNAVAILABLE, RetryAdvice.RECONNECT_IDEMPOTENT, detail);
  }

  private static HydraCacheException sessionError(String detail) {
    return local(ErrorCode.SESSION_LOST, RetryAdvice.NEVER, detail);
  }

  private static HydraCacheException local(
      ErrorCode code, RetryAdvice retryAdvice, String detail) {
    return new HydraCacheException(code, retryAdvice, detail);
  }

  private static <T> CompletableFuture<T> failed(Throwable failure) {
    return CompletableFuture.failedFuture(failure);
  }
}
