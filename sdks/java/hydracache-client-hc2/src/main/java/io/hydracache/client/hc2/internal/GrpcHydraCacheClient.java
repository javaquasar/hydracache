package io.hydracache.client.hc2.internal;

import com.google.protobuf.ByteString;
import io.grpc.ManagedChannel;
import io.grpc.Status;
import io.grpc.netty.shaded.io.grpc.netty.GrpcSslContexts;
import io.grpc.netty.shaded.io.grpc.netty.NettyChannelBuilder;
import io.grpc.stub.StreamObserver;
import io.hydracache.client.hc2.BatchItemResult;
import io.hydracache.client.hc2.BatchOperation;
import io.hydracache.client.hc2.CacheEventListener;
import io.hydracache.client.hc2.CacheValue;
import io.hydracache.client.hc2.Capability;
import io.hydracache.client.hc2.ClientMetricsSnapshot;
import io.hydracache.client.hc2.ErrorCode;
import io.hydracache.client.hc2.FencedSession;
import io.hydracache.client.hc2.HydraCacheClient;
import io.hydracache.client.hc2.HydraCacheClientConfig;
import io.hydracache.client.hc2.HydraCacheException;
import io.hydracache.client.hc2.MutationResult;
import io.hydracache.client.hc2.NodeEndpoint;
import io.hydracache.client.hc2.RequestOptions;
import io.hydracache.client.hc2.RetryAdvice;
import io.hydracache.client.hc2.Subscription;
import io.hydracache.client.hc2.TopologySnapshot;
import io.hydracache.client.hc2.internal.wire.BatchItem;
import io.hydracache.client.hc2.internal.wire.BatchRequest;
import io.hydracache.client.hc2.internal.wire.BatchResult;
import io.hydracache.client.hc2.internal.wire.Cancel;
import io.hydracache.client.hc2.internal.wire.ClientEnvelope;
import io.hydracache.client.hc2.internal.wire.ClientPlaneAlphaGrpc;
import io.hydracache.client.hc2.internal.wire.CompareAndSetRequest;
import io.hydracache.client.hc2.internal.wire.DeleteRequest;
import io.hydracache.client.hc2.internal.wire.EventGap;
import io.hydracache.client.hc2.internal.wire.GetRequest;
import io.hydracache.client.hc2.internal.wire.Handshake;
import io.hydracache.client.hc2.internal.wire.HandshakeAck;
import io.hydracache.client.hc2.internal.wire.InvocationRequest;
import io.hydracache.client.hc2.internal.wire.InvocationResponse;
import io.hydracache.client.hc2.internal.wire.PutRequest;
import io.hydracache.client.hc2.internal.wire.RequestMeta;
import io.hydracache.client.hc2.internal.wire.ResponseMeta;
import io.hydracache.client.hc2.internal.wire.ServerEnvelope;
import io.hydracache.client.hc2.internal.wire.SessionClose;
import io.hydracache.client.hc2.internal.wire.SessionHeartbeat;
import io.hydracache.client.hc2.internal.wire.SessionOpen;
import io.hydracache.client.hc2.internal.wire.Subscribe;
import io.hydracache.client.hc2.internal.wire.SubscriptionAck;
import io.hydracache.client.hc2.internal.wire.TopologyUpdate;
import io.hydracache.client.hc2.internal.wire.Unsubscribe;
import java.net.URI;
import java.time.Duration;
import java.time.Instant;
import java.util.ArrayList;
import java.util.EnumSet;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.ScheduledFuture;
import java.util.concurrent.Semaphore;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;
import java.util.concurrent.atomic.LongAdder;

/** gRPC implementation hidden behind the stable preview API. */
public final class GrpcHydraCacheClient implements HydraCacheClient {
  private static final int GENERATION = 5;
  private static final long CONNECTION_GENERATION = 1;

  private final HydraCacheClientConfig config;
  private final ManagedChannel channel;
  private final ScheduledExecutorService scheduler;
  private final StreamObserver<ClientEnvelope> outbound;
  private final Object outboundLock = new Object();
  private final AtomicBoolean closed = new AtomicBoolean();
  private final AtomicLong correlations = new AtomicLong();
  private final AtomicLong subscriptionIds = new AtomicLong();
  private final Semaphore invocationPermits;
  private final Semaphore subscriptionPermits;
  private final Semaphore sessionPermits;
  private final CompletableFuture<HandshakeAck> handshake = new CompletableFuture<>();
  private volatile long handshakeCorrelation;
  private final Map<Long, PendingInvocation> pendingInvocations = new ConcurrentHashMap<>();
  private final Map<Long, PendingSubscription> pendingSubscriptions = new ConcurrentHashMap<>();
  private final Map<Long, ActiveSubscription> subscriptions = new ConcurrentHashMap<>();
  private final Map<Long, PendingSession> pendingSessions = new ConcurrentHashMap<>();
  private final Map<ByteString, SessionHandle> sessions = new ConcurrentHashMap<>();
  private volatile TopologySnapshot topology = TopologySnapshot.empty();
  private final LongAdder submitted = new LongAdder();
  private final LongAdder completed = new LongAdder();
  private final LongAdder failed = new LongAdder();
  private final LongAdder cancelled = new LongAdder();
  private final LongAdder events = new LongAdder();
  private final LongAdder listenerFailures = new LongAdder();

  private GrpcHydraCacheClient(HydraCacheClientConfig config) throws Exception {
    this.config = config;
    this.scheduler = java.util.concurrent.Executors.newSingleThreadScheduledExecutor(runnable -> {
      Thread thread = new Thread(runnable, "hydracache-hc2-deadlines");
      thread.setDaemon(true);
      return thread;
    });
    this.invocationPermits = new Semaphore(config.maxPendingInvocations());
    this.subscriptionPermits = new Semaphore(config.maxSubscriptions());
    this.sessionPermits = new Semaphore(config.maxSessions());
    URI endpoint = config.endpoint();
    var sslContext = GrpcSslContexts.forClient()
        .trustManager(config.trustCertificate().toFile())
        .keyManager(config.clientCertificate().toFile(), config.clientPrivateKey().toFile())
        .build();
    this.channel = NettyChannelBuilder.forAddress(endpoint.getHost(), endpoint.getPort())
        .sslContext(sslContext)
        .overrideAuthority(config.serverName())
        .maxInboundMessageSize(config.maxInboundMessageBytes())
        .build();
    var stub = ClientPlaneAlphaGrpc.newStub(channel);
    this.outbound = stub.open(new InboundObserver());
  }

  public static HydraCacheClient connect(HydraCacheClientConfig config) {
    GrpcHydraCacheClient client = null;
    try {
      client = new GrpcHydraCacheClient(config);
      long correlation = client.nextCorrelation();
      client.handshakeCorrelation = correlation;
      client.send(ClientEnvelope.newBuilder()
          .setGeneration(GENERATION)
          .setConnectionGeneration(CONNECTION_GENERATION)
          .setCorrelationId(correlation)
          .setHandshake(Handshake.newBuilder()
              .setGeneration(GENERATION)
              .setConnectionGeneration(CONNECTION_GENERATION)
              .setClientId(config.clientId())
              .addAllRequested(config.requestedCapabilities().stream()
                  .map(GrpcHydraCacheClient::toWireCapability)
                  .toList()))
          .build());
      HandshakeAck ack = client.handshake.get(config.connectTimeout().toMillis(), TimeUnit.MILLISECONDS);
      if (ack.getGeneration() != GENERATION || ack.getConnectionGeneration() != CONNECTION_GENERATION) {
        throw local(ErrorCode.UNSUPPORTED, "peer negotiated an unexpected HC/2 generation");
      }
      Set<Capability> accepted = ack.getAcceptedList().stream()
          .map(GrpcHydraCacheClient::fromWireCapability)
          .collect(() -> EnumSet.noneOf(Capability.class), Set::add, Set::addAll);
      if (!accepted.containsAll(config.requestedCapabilities())) {
        throw local(ErrorCode.UNSUPPORTED, "peer omitted a required HC/2 capability");
      }
      return client;
    } catch (InterruptedException error) {
      Thread.currentThread().interrupt();
      if (client != null) client.close();
      throw new HydraCacheException(ErrorCode.UNAVAILABLE, RetryAdvice.NEVER, "HC/2 connect interrupted", error);
    } catch (ExecutionException | TimeoutException error) {
      if (client != null) client.close();
      throw new HydraCacheException(ErrorCode.UNAVAILABLE, RetryAdvice.NEVER, "HC/2 handshake failed", error);
    } catch (HydraCacheException error) {
      if (client != null) client.close();
      throw error;
    } catch (Exception error) {
      if (client != null) client.close();
      throw new HydraCacheException(ErrorCode.UNAVAILABLE, RetryAdvice.NEVER, "HC/2 channel setup failed", error);
    }
  }

  @Override
  public CompletableFuture<Optional<CacheValue>> get(byte[] key, RequestOptions options) {
    requireBytes(key, "key", 1, 1024 * 1024);
    return invoke(InvocationRequest.newBuilder()
        .setMeta(meta(options))
        .setGet(GetRequest.newBuilder().setKey(ByteString.copyFrom(key)))
        .build(), options).thenApply(response -> {
          requireSuccess(response.getMeta());
          if (!response.hasValue() || !response.getValue().getFound()) return Optional.empty();
          long expires = response.getValue().getExpiresAtUnixMs();
          return Optional.of(new CacheValue(
              response.getValue().getValue().toByteArray(),
              expires == 0 ? Optional.empty() : Optional.of(Instant.ofEpochMilli(expires))));
        });
  }

  @Override
  public CompletableFuture<MutationResult> put(
      byte[] key, byte[] value, Duration ttl, RequestOptions options) {
    requireBytes(key, "key", 1, 1024 * 1024);
    requireBytes(value, "value", 0, 16 * 1024 * 1024);
    return mutation(InvocationRequest.newBuilder()
        .setMeta(meta(options))
        .setPut(PutRequest.newBuilder().setKey(ByteString.copyFrom(key))
            .setValue(ByteString.copyFrom(value)).setTtlMs(ttlMillis(ttl)))
        .build(), options);
  }

  @Override
  public CompletableFuture<MutationResult> delete(byte[] key, RequestOptions options) {
    requireBytes(key, "key", 1, 1024 * 1024);
    return mutation(InvocationRequest.newBuilder().setMeta(meta(options))
        .setDelete(DeleteRequest.newBuilder().setKey(ByteString.copyFrom(key))).build(), options);
  }

  @Override
  public CompletableFuture<MutationResult> compareAndSet(
      byte[] key, byte[] expected, byte[] replacement, Duration ttl, RequestOptions options) {
    requireBytes(key, "key", 1, 1024 * 1024);
    requireBytes(expected, "expected", 0, 16 * 1024 * 1024);
    requireBytes(replacement, "replacement", 0, 16 * 1024 * 1024);
    return mutation(InvocationRequest.newBuilder().setMeta(meta(options))
        .setCompareAndSet(CompareAndSetRequest.newBuilder()
            .setKey(ByteString.copyFrom(key)).setExpected(ByteString.copyFrom(expected))
            .setReplacement(ByteString.copyFrom(replacement)).setTtlMs(ttlMillis(ttl)))
        .build(), options);
  }

  @Override
  public CompletableFuture<List<BatchItemResult>> batch(
      List<BatchOperation> operations, RequestOptions options) {
    if (operations.isEmpty() || operations.size() > 1024) {
      throw new IllegalArgumentException("batch size must be in [1, 1024]");
    }
    BatchRequest.Builder batch = BatchRequest.newBuilder();
    int id = 1;
    for (BatchOperation operation : operations) {
      BatchItem.Builder item = BatchItem.newBuilder().setItemId(id++);
      if (operation instanceof BatchOperation.Get get) {
        item.setGet(GetRequest.newBuilder().setKey(ByteString.copyFrom(get.key())));
      } else if (operation instanceof BatchOperation.Put put) {
        item.setPut(PutRequest.newBuilder().setKey(ByteString.copyFrom(put.key()))
            .setValue(ByteString.copyFrom(put.value())).setTtlMs(ttlMillis(put.ttl())));
      } else if (operation instanceof BatchOperation.Delete delete) {
        item.setDelete(DeleteRequest.newBuilder().setKey(ByteString.copyFrom(delete.key())));
      } else if (operation instanceof BatchOperation.CompareAndSet cas) {
        item.setCompareAndSet(CompareAndSetRequest.newBuilder()
            .setKey(ByteString.copyFrom(cas.key())).setExpected(ByteString.copyFrom(cas.expected()))
            .setReplacement(ByteString.copyFrom(cas.replacement())).setTtlMs(ttlMillis(cas.ttl())));
      } else {
        throw new IllegalArgumentException("unsupported batch operation");
      }
      batch.addItems(item);
    }
    return invoke(InvocationRequest.newBuilder().setMeta(meta(options)).setBatch(batch).build(), options)
        .thenApply(response -> mapBatch(response, operations.size()));
  }

  @Override
  public CompletableFuture<Subscription> subscribe(
      byte[] keyPrefix, long resumeWatermark, CacheEventListener listener) {
    requireBytes(keyPrefix, "keyPrefix", 0, 1024 * 1024);
    java.util.Objects.requireNonNull(listener, "listener");
    if (resumeWatermark < 0) throw new IllegalArgumentException("resumeWatermark must be nonnegative");
    if (closed.get()) return CompletableFuture.failedFuture(local(ErrorCode.UNAVAILABLE, "client is closed"));
    if (!subscriptionPermits.tryAcquire()) {
      return CompletableFuture.failedFuture(local(ErrorCode.QUOTA_EXCEEDED, "subscription limit reached"));
    }
    long id = nextNonzero(subscriptionIds, "subscription id exhausted");
    long correlation = nextCorrelation();
    PendingSubscription pending = new PendingSubscription(id, listener);
    pendingSubscriptions.put(correlation, pending);
    if (closed.get() && pendingSubscriptions.remove(correlation, pending)) {
      pending.fail(local(ErrorCode.UNAVAILABLE, "client is closed"));
      return pending.future;
    }
    pending.future.whenComplete((ignored, error) -> {
      if (pending.future.isCancelled() && pendingSubscriptions.remove(correlation, pending)) {
        pending.cancelTimeout();
        pending.release();
        try {
          send(ClientEnvelope.newBuilder().setGeneration(GENERATION)
              .setConnectionGeneration(CONNECTION_GENERATION).setCorrelationId(nextCorrelation())
              .setUnsubscribe(Unsubscribe.newBuilder().setSubscriptionId(id)).build());
        } catch (RuntimeException ignoredSend) { }
      }
    });
    try {
      pending.timeout = scheduler.schedule(() -> {
        if (pendingSubscriptions.remove(correlation, pending)) {
          pending.fail(local(ErrorCode.DEADLINE_EXCEEDED, "subscription acknowledgement timed out"));
        }
      }, config.defaultRequestTimeout().toNanos(), TimeUnit.NANOSECONDS);
      send(ClientEnvelope.newBuilder().setGeneration(GENERATION)
          .setConnectionGeneration(CONNECTION_GENERATION).setCorrelationId(correlation)
          .setSubscribe(Subscribe.newBuilder().setSubscriptionId(id)
              .setKeyPrefix(ByteString.copyFrom(keyPrefix)).setResumeWatermark(resumeWatermark))
          .build());
    } catch (RuntimeException error) {
      if (pendingSubscriptions.remove(correlation, pending)) pending.fail(error);
    }
    return pending.future;
  }

  @Override
  public CompletableFuture<FencedSession> openSession(Duration requestedTtl) {
    long ttl = ttlMillis(requestedTtl);
    if (ttl == 0) throw new IllegalArgumentException("session TTL must be positive");
    if (closed.get()) return CompletableFuture.failedFuture(local(ErrorCode.UNAVAILABLE, "client is closed"));
    if (!sessionPermits.tryAcquire()) {
      return CompletableFuture.failedFuture(local(ErrorCode.QUOTA_EXCEEDED, "session limit reached"));
    }
    long correlation = nextCorrelation();
    PendingSession pending = new PendingSession(Duration.ofMillis(ttl));
    pendingSessions.put(correlation, pending);
    if (closed.get() && pendingSessions.remove(correlation, pending)) {
      pending.fail(local(ErrorCode.UNAVAILABLE, "client is closed"));
      return pending.future;
    }
    pending.future.whenComplete((ignored, error) -> {
      if (pending.future.isCancelled() && pendingSessions.remove(correlation, pending)) {
        pending.cancelTimeout();
        pending.release();
      }
    });
    try {
      pending.timeout = scheduler.schedule(() -> {
        if (pendingSessions.remove(correlation, pending)) {
          pending.fail(local(ErrorCode.DEADLINE_EXCEEDED, "session acknowledgement timed out"));
        }
      }, config.defaultRequestTimeout().toNanos(), TimeUnit.NANOSECONDS);
      send(ClientEnvelope.newBuilder().setGeneration(GENERATION)
          .setConnectionGeneration(CONNECTION_GENERATION).setCorrelationId(correlation)
          .setSessionOpen(SessionOpen.newBuilder().setRequestedTtlMs(ttl)).build());
    } catch (RuntimeException error) {
      if (pendingSessions.remove(correlation, pending)) pending.fail(error);
    }
    return pending.future;
  }

  @Override public TopologySnapshot topology() { return topology; }

  @Override
  public ClientMetricsSnapshot metrics() {
    return new ClientMetricsSnapshot(submitted.sum(), completed.sum(), failed.sum(), cancelled.sum(),
        events.sum(), listenerFailures.sum(), pendingInvocations.size(), subscriptions.size(), sessions.size());
  }

  @Override
  public void close() {
    if (!closed.compareAndSet(false, true)) return;
    synchronized (outboundLock) {
      try { outbound.onCompleted(); } catch (RuntimeException ignored) { }
    }
    HydraCacheException error = local(ErrorCode.UNAVAILABLE, "client closed");
    failOutstanding(error);
    scheduler.shutdownNow();
    channel.shutdown();
    try {
      if (!channel.awaitTermination(config.connectTimeout().toMillis(), TimeUnit.MILLISECONDS)) {
        channel.shutdownNow();
      }
    } catch (InterruptedException interrupted) {
      channel.shutdownNow();
      Thread.currentThread().interrupt();
    }
  }

  private CompletableFuture<MutationResult> mutation(
      InvocationRequest request, RequestOptions options) {
    return invoke(request, options).thenApply(response -> {
      requireSuccess(response.getMeta());
      if (!response.hasMutation()) throw local(ErrorCode.INTERNAL, "peer omitted mutation result");
      return new MutationResult(response.getMutation().getApplied());
    });
  }

  private CompletableFuture<InvocationResponse> invoke(
      InvocationRequest request, RequestOptions options) {
    if (closed.get()) return CompletableFuture.failedFuture(local(ErrorCode.UNAVAILABLE, "client is closed"));
    if (!invocationPermits.tryAcquire()) {
      return CompletableFuture.failedFuture(local(ErrorCode.QUOTA_EXCEEDED, "pending invocation limit reached"));
    }
    long correlation = nextCorrelation();
    PendingInvocation pending = new PendingInvocation(correlation);
    pendingInvocations.put(correlation, pending);
    submitted.increment();
    if (closed.get() && pendingInvocations.remove(correlation, pending)) {
      pending.fail(local(ErrorCode.UNAVAILABLE, "client is closed"));
      return pending.future;
    }
    pending.future.whenComplete((ignored, error) -> {
      if (pending.future.isCancelled() && pendingInvocations.remove(correlation, pending)) {
        cancelled.increment();
        pending.cancelTimeout();
        pending.release();
        sendCancel(correlation, "caller cancelled");
      }
    });
    try {
      pending.timeout = scheduler.schedule(
          () -> timeout(pending), options.timeout().toNanos(), TimeUnit.NANOSECONDS);
      send(ClientEnvelope.newBuilder().setGeneration(GENERATION)
          .setConnectionGeneration(CONNECTION_GENERATION).setCorrelationId(correlation)
          .setInvocation(request).build());
    } catch (RuntimeException error) {
      if (pendingInvocations.remove(correlation, pending)) pending.fail(error);
    }
    return pending.future;
  }

  private RequestMeta meta(RequestOptions options) {
    long now = System.currentTimeMillis();
    long timeout = options.timeout().toMillis();
    long deadline = now > Long.MAX_VALUE - timeout ? Long.MAX_VALUE : now + timeout;
    return RequestMeta.newBuilder().setDeadlineUnixMs(deadline)
        .setIdempotencyKey(ByteString.copyFrom(options.idempotencyKey()))
        .setTenant(config.tenant()).setTopologyEpoch(topology.epoch()).build();
  }

  private void timeout(PendingInvocation pending) {
    if (pendingInvocations.remove(pending.correlation, pending)) {
      cancelled.increment();
      pending.fail(local(ErrorCode.DEADLINE_EXCEEDED, "local invocation deadline exceeded"));
      sendCancel(pending.correlation, "deadline exceeded");
    }
  }

  private void sendCancel(long correlation, String reason) {
    if (closed.get()) return;
    try {
      send(ClientEnvelope.newBuilder().setGeneration(GENERATION)
          .setConnectionGeneration(CONNECTION_GENERATION).setCorrelationId(nextCorrelation())
          .setCancel(Cancel.newBuilder().setCorrelationId(correlation).setSafeReason(reason)).build());
    } catch (RuntimeException ignored) { }
  }

  private void send(ClientEnvelope envelope) {
    if (closed.get()) throw local(ErrorCode.UNAVAILABLE, "client is closed");
    synchronized (outboundLock) { outbound.onNext(envelope); }
  }

  private long nextCorrelation() { return nextNonzero(correlations, "correlation id exhausted"); }

  private static long nextNonzero(AtomicLong sequence, String exhausted) {
    long value = sequence.incrementAndGet();
    if (value <= 0) throw local(ErrorCode.INTERNAL, exhausted);
    return value;
  }

  private void failConnection(Throwable cause) {
    HydraCacheException error = cause instanceof HydraCacheException hc ? hc
        : new HydraCacheException(ErrorCode.UNAVAILABLE, RetryAdvice.RECONNECT_IDEMPOTENT,
            "HC/2 connection failed", cause);
    if (!closed.compareAndSet(false, true)) return;
    failed.add(pendingInvocations.size());
    failOutstanding(error);
    scheduler.shutdownNow();
    channel.shutdownNow();
  }

  private void failOutstanding(HydraCacheException error) {
    handshake.completeExceptionally(error);
    pendingInvocations.values().forEach(pending -> pending.fail(error));
    pendingInvocations.clear();
    pendingSubscriptions.values().forEach(pending -> pending.fail(error));
    pendingSubscriptions.clear();
    pendingSessions.values().forEach(pending -> pending.fail(error));
    pendingSessions.clear();
    subscriptions.values().forEach(ActiveSubscription::stopWithoutSend);
    subscriptions.clear();
    sessions.values().forEach(SessionHandle::stopWithoutSend);
    sessions.clear();
  }

  private void accept(ServerEnvelope envelope) {
    if (envelope.getGeneration() != GENERATION
        || envelope.getConnectionGeneration() != CONNECTION_GENERATION) {
      failConnection(local(ErrorCode.UNSUPPORTED, "stale or incompatible server envelope"));
      return;
    }
    switch (envelope.getMessageCase()) {
      case HANDSHAKE -> {
        if (envelope.getCorrelationId() != handshakeCorrelation || handshake.isDone()) {
          failConnection(local(ErrorCode.INTERNAL, "handshake correlation mismatch"));
        } else {
          handshake.complete(envelope.getHandshake());
        }
      }
      case INVOCATION -> completeInvocation(envelope.getCorrelationId(), envelope.getInvocation());
      case SUBSCRIBED -> completeSubscription(envelope.getCorrelationId(), envelope.getSubscribed());
      case EVENT -> dispatchEvent(envelope.getEvent());
      case GAP -> dispatchGap(envelope.getGap());
      case TOPOLOGY -> acceptTopology(envelope.getTopology());
      case SESSION_HEARTBEAT -> acceptSessionHeartbeat(envelope.getCorrelationId(), envelope.getSessionHeartbeat());
      case SESSION_LOST -> {
        SessionHandle lost = sessions.remove(envelope.getSessionLost().getSessionId());
        if (lost != null) lost.stopWithoutSend();
      }
      case DIAGNOSTICS, MESSAGE_NOT_SET -> { }
    }
  }

  private void completeInvocation(long correlation, InvocationResponse response) {
    PendingInvocation pending = pendingInvocations.remove(correlation);
    if (pending == null) return;
    try {
      requireSuccess(response.getMeta());
      completed.increment();
      pending.complete(response);
    } catch (RuntimeException error) {
      failed.increment();
      pending.fail(error);
    }
  }

  private void completeSubscription(long correlation, SubscriptionAck ack) {
    PendingSubscription pending = pendingSubscriptions.remove(correlation);
    if (pending == null) return;
    if (pending.id != ack.getSubscriptionId()) {
      pending.fail(local(ErrorCode.INTERNAL, "subscription acknowledgement identity mismatch"));
      failConnection(local(ErrorCode.INTERNAL, "subscription acknowledgement identity mismatch"));
      return;
    }
    ActiveSubscription subscription = new ActiveSubscription(ack.getSubscriptionId(), ack.getWatermark(), pending.listener);
    pending.transferOwnership();
    subscriptions.put(subscription.id(), subscription);
    if (closed.get()) {
      subscriptions.remove(subscription.id(), subscription);
      subscription.stopWithoutSend();
      pending.fail(local(ErrorCode.UNAVAILABLE, "connection closed during subscription acknowledgement"));
      return;
    }
    if (!pending.complete(subscription)) subscription.close();
  }

  private void dispatchEvent(io.hydracache.client.hc2.internal.wire.CacheEvent event) {
    ActiveSubscription subscription = subscriptions.get(event.getSubscriptionId());
    if (subscription == null) return;
    events.increment();
    try {
      config.callbackExecutor().execute(() -> {
        try {
          subscription.listener.onEvent(new io.hydracache.client.hc2.CacheEvent(
              event.getSubscriptionId(), event.getWatermark(), event.getKey().toByteArray(),
              event.getValue().toByteArray(), event.getRemoved()));
        } catch (RuntimeException error) { listenerFailures.increment(); }
      });
    } catch (RuntimeException rejected) { listenerFailures.increment(); }
  }

  private void dispatchGap(EventGap gap) {
    ActiveSubscription subscription = subscriptions.get(gap.getSubscriptionId());
    if (subscription == null) return;
    try {
      config.callbackExecutor().execute(() -> {
        try { subscription.listener.onGap(gap.getSubscriptionId(), gap.getAfterWatermark()); }
        catch (RuntimeException error) { listenerFailures.increment(); }
      });
    } catch (RuntimeException rejected) { listenerFailures.increment(); }
  }

  private void acceptTopology(TopologyUpdate update) {
    if (update.getEpoch() <= topology.epoch()) return;
    if (update.getNodesCount() > config.maxTopologyNodes()) {
      failConnection(local(ErrorCode.QUOTA_EXCEEDED, "topology node limit exceeded"));
      return;
    }
    try {
      List<NodeEndpoint> nodes = update.getNodesList().stream().map(node -> new NodeEndpoint(
          node.getNodeId(), node.getNodeEpoch(), URI.create(node.getEndpointUri()), node.getServerName())).toList();
      topology = new TopologySnapshot(update.getEpoch(), nodes);
    } catch (RuntimeException malformed) {
      failConnection(new HydraCacheException(ErrorCode.INVALID_REQUEST, RetryAdvice.NEVER,
          "peer sent malformed topology", malformed));
    }
  }

  private void acceptSessionHeartbeat(long correlation, SessionHeartbeat heartbeat) {
    PendingSession pending = pendingSessions.remove(correlation);
    if (pending != null) {
      if (heartbeat.getSessionId().isEmpty() || heartbeat.getFence() == 0) {
        pending.fail(local(ErrorCode.INTERNAL, "invalid session acknowledgement"));
        failConnection(local(ErrorCode.INTERNAL, "invalid session acknowledgement"));
        return;
      }
      SessionHandle handle = new SessionHandle(heartbeat.getSessionId(), heartbeat.getFence(), pending.ttl);
      pending.transferOwnership();
      sessions.put(heartbeat.getSessionId(), handle);
      if (closed.get()) {
        sessions.remove(heartbeat.getSessionId(), handle);
        handle.stopWithoutSend();
        pending.fail(local(ErrorCode.UNAVAILABLE, "connection closed during session acknowledgement"));
        return;
      }
      try {
        handle.startHeartbeat();
      } catch (RuntimeException error) {
        if (sessions.remove(heartbeat.getSessionId(), handle)) handle.stopWithoutSend();
        pending.fail(error);
        failConnection(error);
        return;
      }
      if (!pending.complete(handle)) handle.close();
      return;
    }
    SessionHandle existing = sessions.get(heartbeat.getSessionId());
    if (existing != null) existing.fence.accumulateAndGet(heartbeat.getFence(), Math::max);
  }

  private static List<BatchItemResult> mapBatch(InvocationResponse response, int expected) {
    requireSuccess(response.getMeta());
    if (!response.hasBatch()) throw local(ErrorCode.INTERNAL, "peer omitted batch result");
    BatchResult batch = response.getBatch();
    if (batch.getItemsCount() != expected) throw local(ErrorCode.INTERNAL, "batch result count mismatch");
    List<BatchItemResult> results = new ArrayList<>(expected);
    for (int index = 0; index < expected; index++) {
      InvocationResponse item = batch.getItems(index);
      requireSuccess(item.getMeta());
      Optional<CacheValue> value = Optional.empty();
      Optional<MutationResult> mutation = Optional.empty();
      if (item.hasValue() && item.getValue().getFound()) {
        long expires = item.getValue().getExpiresAtUnixMs();
        value = Optional.of(new CacheValue(item.getValue().getValue().toByteArray(),
            expires == 0 ? Optional.empty() : Optional.of(Instant.ofEpochMilli(expires))));
      } else if (item.hasMutation()) {
        mutation = Optional.of(new MutationResult(item.getMutation().getApplied()));
      }
      results.add(new BatchItemResult(index + 1, value, mutation));
    }
    return List.copyOf(results);
  }

  private static void requireSuccess(ResponseMeta meta) {
    if (meta.getError() == io.hydracache.client.hc2.internal.wire.StableErrorCode.STABLE_ERROR_UNSPECIFIED) return;
    throw new HydraCacheException(mapError(meta.getError()), mapRetry(meta.getRetry()), meta.getSafeDetail());
  }

  private static ErrorCode mapError(io.hydracache.client.hc2.internal.wire.StableErrorCode code) {
    return switch (code) {
      case STABLE_ERROR_INVALID_REQUEST -> ErrorCode.INVALID_REQUEST;
      case STABLE_ERROR_UNAUTHENTICATED -> ErrorCode.UNAUTHENTICATED;
      case STABLE_ERROR_UNAUTHORIZED -> ErrorCode.UNAUTHORIZED;
      case STABLE_ERROR_NOT_FOUND -> ErrorCode.NOT_FOUND;
      case STABLE_ERROR_CONFLICT -> ErrorCode.CONFLICT;
      case STABLE_ERROR_QUOTA_EXCEEDED -> ErrorCode.QUOTA_EXCEEDED;
      case STABLE_ERROR_DEADLINE_EXCEEDED -> ErrorCode.DEADLINE_EXCEEDED;
      case STABLE_ERROR_UNAVAILABLE -> ErrorCode.UNAVAILABLE;
      case STABLE_ERROR_UNSUPPORTED -> ErrorCode.UNSUPPORTED;
      case STABLE_ERROR_GAP_REPAIR_REQUIRED -> ErrorCode.GAP_REPAIR_REQUIRED;
      case STABLE_ERROR_SESSION_LOST -> ErrorCode.SESSION_LOST;
      case STABLE_ERROR_INTERNAL -> ErrorCode.INTERNAL;
      case STABLE_ERROR_UNSPECIFIED, UNRECOGNIZED -> ErrorCode.UNKNOWN;
    };
  }

  private static RetryAdvice mapRetry(io.hydracache.client.hc2.internal.wire.RetryDirective retry) {
    return switch (retry) {
      case RETRY_DIRECTIVE_NEVER -> RetryAdvice.NEVER;
      case RETRY_DIRECTIVE_SAME_CONNECTION -> RetryAdvice.SAME_CONNECTION;
      case RETRY_DIRECTIVE_RECONNECT_IDEMPOTENT -> RetryAdvice.RECONNECT_IDEMPOTENT;
      case RETRY_DIRECTIVE_REPAIR_REQUIRED -> RetryAdvice.REPAIR_REQUIRED;
      case RETRY_DIRECTIVE_UNSPECIFIED, UNRECOGNIZED -> RetryAdvice.UNSPECIFIED;
    };
  }

  private static io.hydracache.client.hc2.internal.wire.Capability toWireCapability(Capability capability) {
    return io.hydracache.client.hc2.internal.wire.Capability.valueOf("CAPABILITY_" + capability.name());
  }

  private static Capability fromWireCapability(io.hydracache.client.hc2.internal.wire.Capability capability) {
    if (capability == io.hydracache.client.hc2.internal.wire.Capability.UNRECOGNIZED
        || capability == io.hydracache.client.hc2.internal.wire.Capability.CAPABILITY_UNSPECIFIED) {
      throw local(ErrorCode.UNSUPPORTED, "peer returned an unknown capability");
    }
    return Capability.valueOf(capability.name().substring("CAPABILITY_".length()));
  }

  private static long ttlMillis(Duration ttl) {
    if (ttl == null) return 0;
    if (ttl.isNegative() || ttl.compareTo(Duration.ofDays(3650)) > 0) {
      throw new IllegalArgumentException("TTL must be nonnegative and at most ten years");
    }
    return ttl.toMillis();
  }

  private static void requireBytes(byte[] value, String name, int min, int max) {
    if (value == null || value.length < min || value.length > max) {
      throw new IllegalArgumentException(name + " length must be in [" + min + ", " + max + "]");
    }
  }

  private static HydraCacheException local(ErrorCode code, String detail) {
    return new HydraCacheException(code, RetryAdvice.NEVER, detail);
  }

  private final class InboundObserver implements StreamObserver<ServerEnvelope> {
    @Override public void onNext(ServerEnvelope value) { accept(value); }
    @Override public void onError(Throwable error) { failConnection(error); }
    @Override public void onCompleted() { failConnection(Status.UNAVAILABLE.withDescription("peer closed HC/2 stream").asRuntimeException()); }
  }

  private final class PendingInvocation {
    final long correlation;
    final CompletableFuture<InvocationResponse> future = new CompletableFuture<>();
    final AtomicBoolean released = new AtomicBoolean();
    volatile ScheduledFuture<?> timeout;
    PendingInvocation(long correlation) { this.correlation = correlation; }
    void complete(InvocationResponse value) { cancelTimeout(); future.complete(value); release(); }
    void fail(Throwable error) { cancelTimeout(); future.completeExceptionally(error); release(); }
    void cancelTimeout() { if (timeout != null) timeout.cancel(false); }
    void release() { if (released.compareAndSet(false, true)) invocationPermits.release(); }
  }

  private final class PendingSubscription {
    final long id;
    final CacheEventListener listener;
    final CompletableFuture<Subscription> future = new CompletableFuture<>();
    final AtomicBoolean released = new AtomicBoolean();
    volatile ScheduledFuture<?> timeout;
    PendingSubscription(long id, CacheEventListener listener) {
      this.id = id; this.listener = listener;
    }
    boolean complete(Subscription subscription) {
      cancelTimeout();
      return future.complete(subscription);
    }
    void fail(Throwable error) { cancelTimeout(); future.completeExceptionally(error); release(); }
    void cancelTimeout() { if (timeout != null) timeout.cancel(false); }
    void release() { if (released.compareAndSet(false, true)) subscriptionPermits.release(); }
    void transferOwnership() { released.set(true); }
  }

  private final class PendingSession {
    final Duration ttl;
    final CompletableFuture<FencedSession> future = new CompletableFuture<>();
    final AtomicBoolean released = new AtomicBoolean();
    volatile ScheduledFuture<?> timeout;
    PendingSession(Duration ttl) { this.ttl = ttl; }
    boolean complete(FencedSession session) {
      cancelTimeout();
      return future.complete(session);
    }
    void fail(Throwable error) { cancelTimeout(); future.completeExceptionally(error); release(); }
    void cancelTimeout() { if (timeout != null) timeout.cancel(false); }
    void release() { if (released.compareAndSet(false, true)) sessionPermits.release(); }
    void transferOwnership() { released.set(true); }
  }

  private final class ActiveSubscription implements Subscription {
    private final long id;
    private final long initialWatermark;
    private final CacheEventListener listener;
    private final AtomicBoolean stopped = new AtomicBoolean();
    ActiveSubscription(long id, long initialWatermark, CacheEventListener listener) {
      this.id = id; this.initialWatermark = initialWatermark; this.listener = listener;
    }
    @Override public long id() { return id; }
    @Override public long initialWatermark() { return initialWatermark; }
    @Override public void close() {
      if (!stopped.compareAndSet(false, true) || !subscriptions.remove(id, this)) return;
      subscriptionPermits.release();
      if (!closed.get()) {
        send(ClientEnvelope.newBuilder().setGeneration(GENERATION)
            .setConnectionGeneration(CONNECTION_GENERATION).setCorrelationId(nextCorrelation())
            .setUnsubscribe(Unsubscribe.newBuilder().setSubscriptionId(id)).build());
      }
    }
    void stopWithoutSend() {
      if (stopped.compareAndSet(false, true)) subscriptionPermits.release();
    }
  }

  private final class SessionHandle implements FencedSession {
    private final ByteString id;
    private final AtomicLong fence;
    private final Duration ttl;
    private final AtomicBoolean stopped = new AtomicBoolean();
    private volatile ScheduledFuture<?> heartbeatTask;
    SessionHandle(ByteString id, long fence, Duration ttl) {
      this.id = id; this.fence = new AtomicLong(fence); this.ttl = ttl;
    }
    void startHeartbeat() {
      long period = Math.max(100, ttl.toMillis() / 3);
      heartbeatTask = scheduler.scheduleAtFixedRate(() -> {
        if (stopped.get() || closed.get()) return;
        try {
          send(ClientEnvelope.newBuilder().setGeneration(GENERATION)
              .setConnectionGeneration(CONNECTION_GENERATION).setCorrelationId(nextCorrelation())
              .setSessionHeartbeat(SessionHeartbeat.newBuilder()
                  .setSessionId(id).setFence(fence.get())).build());
        } catch (RuntimeException error) {
          failConnection(error);
        }
      }, period, period, TimeUnit.MILLISECONDS);
    }
    void stopWithoutSend() {
      if (stopped.compareAndSet(false, true)) sessionPermits.release();
      if (heartbeatTask != null) heartbeatTask.cancel(false);
    }
    @Override public byte[] id() { return id.toByteArray(); }
    @Override public long fence() { return fence.get(); }
    @Override public void close() {
      if (!stopped.compareAndSet(false, true) || !sessions.remove(id, this)) return;
      sessionPermits.release();
      if (heartbeatTask != null) heartbeatTask.cancel(false);
      if (!closed.get()) send(ClientEnvelope.newBuilder().setGeneration(GENERATION)
          .setConnectionGeneration(CONNECTION_GENERATION).setCorrelationId(nextCorrelation())
          .setSessionClose(SessionClose.newBuilder().setSessionId(id).setFence(fence.get())).build());
    }
  }
}
