package io.hydracache.hazelcast;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import io.hydracache.client.hc2.CacheEvent;
import io.hydracache.client.hc2.CacheEventListener;
import io.hydracache.client.hc2.CacheValue;
import io.hydracache.client.hc2.BatchItemResult;
import io.hydracache.client.hc2.BatchOperation;
import io.hydracache.client.hc2.ClientMetricsSnapshot;
import io.hydracache.client.hc2.FencedSession;
import io.hydracache.client.hc2.HydraCacheClient;
import io.hydracache.client.hc2.LockAcquireResult;
import io.hydracache.client.hc2.LockOwnership;
import io.hydracache.client.hc2.MutationResult;
import io.hydracache.client.hc2.RequestOptions;
import io.hydracache.client.hc2.Subscription;
import io.hydracache.client.hc2.TopologySnapshot;
import java.time.Duration;
import java.util.Arrays;
import java.util.Map;
import java.util.List;
import java.util.Optional;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicLong;
import org.junit.jupiter.api.Test;

final class HydraCacheFacadeTest {
  @Test void mapsUseExplicitCodecsAndIndependentPhysicalNamespaces() {
    FakeClient client = new FakeClient();
    HydraCacheInstance instance = new HydraCacheInstance(client, Duration.ofSeconds(1));
    HydraMap<String, String> first = instance.getMap("first", HydraCodec.utf8(), HydraCodec.utf8());
    HydraMap<String, String> second = instance.getMap("second", HydraCodec.utf8(), HydraCodec.utf8());

    first.put("key", "one");
    second.put("key", "two");
    assertEquals("one", first.get("key").orElseThrow());
    assertEquals("two", second.get("key").orElseThrow());
    assertTrue(first.replace("key", "one", "three"));
    assertFalse(first.replace("key", "one", "stale"));
    assertFalse(first.remove("key", "wrong"));
    assertTrue(first.remove("key", "three"));
    assertTrue(first.get("key").isEmpty());
  }

  @Test void mapListenerDecodesOnlyItsNamespaceAndPreservesWatermark() {
    FakeClient client = new FakeClient();
    HydraMap<String, String> map = new HydraCacheInstance(client, Duration.ofSeconds(1))
        .getMap("events", HydraCodec.utf8(), HydraCodec.utf8());
    var observed = new java.util.concurrent.atomic.AtomicReference<EntryEvent<String, String>>();
    try (var ignored = map.addEntryListener(new EntryListener<>() {
      @Override public void onEvent(EntryEvent<String, String> event) { observed.set(event); }
      @Override public void onGap(long watermark) { throw new AssertionError("unexpected gap"); }
    })) {
      map.put("alpha", "value");
      assertEquals("alpha", observed.get().key());
      assertEquals("value", observed.get().value().orElseThrow());
      assertEquals(1, observed.get().watermark());
    }
  }

  @Test void fencedLockIsNonReentrantAndUsesServerFences() {
    FakeClient client = new FakeClient();
    HydraFencedLock<String> lock = new HydraCacheInstance(client, Duration.ofSeconds(1))
        .getFencedLock("orders", "42", HydraCodec.utf8());

    long fence = lock.tryLock(Duration.ofSeconds(5)).orElseThrow();
    assertEquals(1, fence);
    assertTrue(lock.isLocked());
    assertEquals(fence, lock.getFence().orElseThrow());
    assertThrows(HydraCacheFacadeException.class,
        () -> lock.tryLock(Duration.ofSeconds(5)));
    lock.renew(Duration.ofSeconds(5));
    lock.unlock();
    assertFalse(lock.isLocked());
  }

  @Test void unsupportedSurfaceFailsLoudly() {
    HydraMap<String, String> map = new HydraCacheInstance(
        new FakeClient(), Duration.ofSeconds(1))
        .getMap("map", HydraCodec.utf8(), HydraCodec.utf8());
    assertThrows(UnsupportedOperationException.class, () -> map.executeOnKey("key"));
  }

  private record Key(byte[] value) {
    Key { value = value.clone(); }
    @Override public boolean equals(Object other) {
      return other instanceof Key that && Arrays.equals(value, that.value);
    }
    @Override public int hashCode() { return Arrays.hashCode(value); }
  }

  private static final class FakeClient implements HydraCacheClient {
    private final Map<Key, byte[]> values = new ConcurrentHashMap<>();
    private final Map<Key, Long> locks = new ConcurrentHashMap<>();
    private final AtomicLong fences = new AtomicLong();
    private volatile byte[] listenerPrefix;
    private volatile CacheEventListener listener;
    private final AtomicLong watermarks = new AtomicLong();

    @Override public CompletableFuture<Optional<CacheValue>> get(
        byte[] key, RequestOptions options) {
      byte[] value = values.get(new Key(key));
      return completed(value == null ? Optional.empty()
          : Optional.of(new CacheValue(value, Optional.empty())));
    }

    @Override public CompletableFuture<MutationResult> put(
        byte[] key, byte[] value, Duration ttl, RequestOptions options) {
      values.put(new Key(key), value.clone());
      emit(key, value, false);
      return completed(new MutationResult(true));
    }

    @Override public CompletableFuture<MutationResult> delete(
        byte[] key, RequestOptions options) {
      boolean applied = values.remove(new Key(key)) != null;
      if (applied) emit(key, new byte[0], true);
      return completed(new MutationResult(applied));
    }

    @Override public CompletableFuture<MutationResult> compareAndSet(
        byte[] key, byte[] expected, byte[] replacement, Duration ttl, RequestOptions options) {
      Key wrapped = new Key(key);
      var applied = new java.util.concurrent.atomic.AtomicBoolean();
      values.computeIfPresent(wrapped, (ignored, current) -> {
        if (!Arrays.equals(current, expected)) return current;
        applied.set(true);
        return replacement.clone();
      });
      if (applied.get()) emit(key, replacement, false);
      return completed(new MutationResult(applied.get()));
    }

    @Override public CompletableFuture<MutationResult> removeIfValue(
        byte[] key, byte[] expected, RequestOptions options) {
      Key wrapped = new Key(key);
      var applied = new java.util.concurrent.atomic.AtomicBoolean();
      values.computeIfPresent(wrapped, (ignored, current) -> {
        if (!Arrays.equals(current, expected)) return current;
        applied.set(true);
        return null;
      });
      if (applied.get()) emit(key, new byte[0], true);
      return completed(new MutationResult(applied.get()));
    }

    @Override public CompletableFuture<LockAcquireResult> tryLock(
        byte[] key, Duration lease, RequestOptions options) {
      Key wrapped = new Key(key);
      long fence = fences.incrementAndGet();
      Long prior = locks.putIfAbsent(wrapped, fence);
      return completed(prior == null
          ? new LockAcquireResult(true, fence) : new LockAcquireResult(false, 0));
    }

    @Override public CompletableFuture<MutationResult> unlock(
        byte[] key, long fence, RequestOptions options) {
      return completed(new MutationResult(locks.remove(new Key(key), fence)));
    }

    @Override public CompletableFuture<MutationResult> renewLock(
        byte[] key, long fence, Duration lease, RequestOptions options) {
      return completed(new MutationResult(Long.valueOf(fence).equals(locks.get(new Key(key)))));
    }

    @Override public CompletableFuture<LockOwnership> lockOwnership(
        byte[] key, RequestOptions options) {
      Long fence = locks.get(new Key(key));
      return completed(fence == null
          ? new LockOwnership(false, 0) : new LockOwnership(true, fence));
    }

    @Override public CompletableFuture<Subscription> subscribe(
        byte[] keyPrefix, long resumeWatermark, CacheEventListener listener) {
      this.listenerPrefix = keyPrefix.clone();
      this.listener = listener;
      return completed(new Subscription() {
        @Override public long id() { return 1; }
        @Override public long initialWatermark() { return resumeWatermark; }
        @Override public void close() { FakeClient.this.listener = null; }
      });
    }

    @Override public CompletableFuture<List<BatchItemResult>> batch(
        List<BatchOperation> operations, RequestOptions options) {
      throw new UnsupportedOperationException();
    }

    @Override public CompletableFuture<FencedSession> openSession(Duration requestedTtl) {
      throw new UnsupportedOperationException();
    }
    @Override public String clusterId() { return "test"; }
    @Override public TopologySnapshot topology() { return TopologySnapshot.empty(); }
    @Override public ClientMetricsSnapshot metrics() {
      return new ClientMetricsSnapshot(0, 0, 0, 0, 0, 0, 0, 0, 0);
    }
    @Override public void close() { }

    private void emit(byte[] key, byte[] value, boolean removed) {
      CacheEventListener current = listener;
      byte[] prefix = listenerPrefix;
      if (current != null && startsWith(key, prefix)) {
        current.onEvent(new CacheEvent(
            1, watermarks.incrementAndGet(), key, value, removed));
      }
    }
    private static boolean startsWith(byte[] value, byte[] prefix) {
      return value.length >= prefix.length
          && Arrays.equals(Arrays.copyOf(value, prefix.length), prefix);
    }
    private static <T> CompletableFuture<T> completed(T value) {
      return CompletableFuture.completedFuture(value);
    }
  }
}
