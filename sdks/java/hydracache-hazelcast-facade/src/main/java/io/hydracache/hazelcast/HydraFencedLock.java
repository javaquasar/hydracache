package io.hydracache.hazelcast;

import io.hydracache.client.hc2.HydraCacheClient;
import io.hydracache.client.hc2.RequestOptions;
import java.time.Duration;
import java.util.OptionalLong;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import java.util.concurrent.atomic.AtomicLong;

/** Non-reentrant single-key fenced lock. Persist the fence beside protected writes. */
public final class HydraFencedLock<K> implements AutoCloseable {
  private static final long RETRY_MILLIS = 25;
  private final HydraCacheClient client;
  private final byte[] key;
  private final Duration requestTimeout;
  private final AtomicLong heldFence = new AtomicLong();

  HydraFencedLock(HydraCacheClient client, byte[] key, Duration requestTimeout) {
    this.client = java.util.Objects.requireNonNull(client, "client");
    this.key = key.clone();
    this.requestTimeout = HydraCacheInstance.requirePositive(requestTimeout, "requestTimeout");
  }

  public OptionalLong tryLock(Duration lease) {
    ensureNotHeld();
    var result = await(client.tryLock(key, positive(lease, "lease"), options()), requestTimeout);
    if (!result.acquired()) return OptionalLong.empty();
    if (!heldFence.compareAndSet(0, result.fence())) {
      throw new HydraCacheFacadeException("lock facade is non-reentrant");
    }
    return OptionalLong.of(result.fence());
  }

  public OptionalLong tryLock(Duration wait, Duration lease) {
    ensureNotHeld();
    positive(wait, "wait");
    positive(lease, "lease");
    long deadline = System.nanoTime() + wait.toNanos();
    do {
      OptionalLong result = tryLock(lease);
      if (result.isPresent()) return result;
      long remaining = deadline - System.nanoTime();
      if (remaining <= 0) break;
      try {
        TimeUnit.NANOSECONDS.sleep(Math.min(
            remaining, TimeUnit.MILLISECONDS.toNanos(RETRY_MILLIS)));
      } catch (InterruptedException error) {
        Thread.currentThread().interrupt();
        throw new HydraCacheFacadeException("lock wait was interrupted", error);
      }
    } while (true);
    return OptionalLong.empty();
  }

  public long lockAndGetFence(Duration wait, Duration lease) {
    return tryLock(wait, lease).orElseThrow(
        () -> new HydraCacheFacadeException("fenced lock wait expired"));
  }

  public void renew(Duration lease) {
    long fence = requiredFence();
    if (!await(client.renewLock(key, fence, positive(lease, "lease"), options()), requestTimeout)
        .applied()) {
      heldFence.compareAndSet(fence, 0);
      throw new HydraCacheFacadeException("fenced lock lease was not renewed");
    }
  }

  public void unlock() {
    long fence = requiredFence();
    if (!await(client.unlock(key, fence, options()), requestTimeout).applied()) {
      throw new HydraCacheFacadeException("fenced lock was not released");
    }
    heldFence.compareAndSet(fence, 0);
  }

  public boolean isLocked() {
    return await(client.lockOwnership(key, options()), requestTimeout).locked();
  }

  public OptionalLong getFence() {
    var ownership = await(client.lockOwnership(key, options()), requestTimeout);
    return ownership.locked() ? OptionalLong.of(ownership.fence()) : OptionalLong.empty();
  }

  public OptionalLong heldFence() {
    long value = heldFence.get();
    return value == 0 ? OptionalLong.empty() : OptionalLong.of(value);
  }

  @Override public void close() {
    if (heldFence.get() != 0) unlock();
  }

  private RequestOptions options() { return RequestOptions.withTimeout(requestTimeout); }
  private void ensureNotHeld() {
    if (heldFence.get() != 0) throw new HydraCacheFacadeException("lock facade is non-reentrant");
  }
  private long requiredFence() {
    long value = heldFence.get();
    if (value == 0) throw new IllegalStateException("this facade does not hold the lock");
    return value;
  }
  private static Duration positive(Duration value, String name) {
    return HydraCacheInstance.requirePositive(value, name);
  }

  private static <T> T await(CompletableFuture<T> future, Duration timeout) {
    try {
      return future.get(timeout.toMillis(), TimeUnit.MILLISECONDS);
    } catch (TimeoutException error) {
      future.cancel(true);
      throw new HydraCacheFacadeException("HC/2 operation exceeded the facade deadline", error);
    } catch (InterruptedException error) {
      Thread.currentThread().interrupt();
      throw new HydraCacheFacadeException("HC/2 operation was interrupted", error);
    } catch (ExecutionException error) {
      Throwable cause = error.getCause();
      if (cause instanceof RuntimeException runtime) throw runtime;
      throw new HydraCacheFacadeException("HC/2 operation failed", cause);
    }
  }
}
