package io.hydracache.imap.semantic;

import java.util.Objects;
import java.util.Optional;
import java.util.concurrent.Flow;

/** A unicast demand-driven adapter. A pending listener source is distinct from stream completion. */
public final class IMapFlowPublisher<T> implements Flow.Publisher<T>, AutoCloseable {
  public enum State { ITEM, PENDING, END }
  public record Pull<T>(State state, Optional<T> value) {
    public Pull {
      Objects.requireNonNull(state); value = Objects.requireNonNull(value);
      if ((state == State.ITEM) != value.isPresent()) throw new IllegalArgumentException("invalid pull");
    }
    public static <T> Pull<T> item(T value) { return new Pull<>(State.ITEM, Optional.of(value)); }
    public static <T> Pull<T> pending() { return new Pull<>(State.PENDING, Optional.empty()); }
    public static <T> Pull<T> end() { return new Pull<>(State.END, Optional.empty()); }
  }
  public interface Source<T> extends AutoCloseable {
    Pull<T> poll();
    @Override void close();
  }

  private final Source<T> source;
  private final int maxPrefetch;
  private Flow.Subscriber<? super T> subscriber;
  private boolean claimed, closed, draining;
  private long demand;
  private RuntimeException cleanupFailure;

  public IMapFlowPublisher(Source<T> source, int maxPrefetch) {
    this.source = Objects.requireNonNull(source);
    if (maxPrefetch <= 0 || maxPrefetch > 256) {
      IllegalArgumentException failure = new IllegalArgumentException("prefetch bound must be in 1..256");
      try { source.close(); } catch (RuntimeException cleanup) { failure.addSuppressed(cleanup); }
      throw failure;
    }
    this.maxPrefetch = maxPrefetch;
  }
  public int maxPrefetch() { return maxPrefetch; }
  public synchronized long demand() { return demand; }
  public synchronized boolean isClosed() { return closed; }
  public synchronized Optional<RuntimeException> cleanupFailure() { return Optional.ofNullable(cleanupFailure); }

  @Override public synchronized void subscribe(Flow.Subscriber<? super T> target) {
    Objects.requireNonNull(target);
    if (claimed || closed) {
      target.onSubscribe(new Flow.Subscription() {
        @Override public void request(long ignored) {}
        @Override public void cancel() {}
      });
      target.onError(new IllegalStateException("publisher permits one subscriber"));
      return;
    }
    claimed = true; subscriber = target;
    try {
      target.onSubscribe(new Flow.Subscription() {
        @Override public void request(long count) { requestItems(count); }
        @Override public void cancel() { IMapFlowPublisher.this.close(); }
      });
    } catch (RuntimeException failure) { terminate(failure); }
  }

  private synchronized void requestItems(long count) {
    if (closed) return;
    if (count <= 0) { terminate(new IllegalArgumentException("demand must be positive")); return; }
    demand = count > Long.MAX_VALUE - demand ? Long.MAX_VALUE : demand + count;
    drain();
  }

  /** Called when an event source changes while a subscriber already has outstanding demand. */
  public synchronized void signal() { drain(); }

  private void drain() {
    if (draining || closed || subscriber == null) return;
    draining = true;
    try {
      while (!closed && demand > 0) {
        Pull<T> item = Objects.requireNonNull(source.poll(), "source poll");
        if (item.state() == State.PENDING) break;
        if (item.state() == State.END) { terminate(null); break; }
        demand--;
        subscriber.onNext(item.value().orElseThrow());
      }
    } catch (RuntimeException failure) { terminate(failure); }
    finally { draining = false; }
  }

  private void terminate(RuntimeException failure) {
    if (closed) return;
    close();
    RuntimeException terminal = failure == null ? cleanupFailure : failure;
    if (subscriber != null) {
      if (terminal == null) subscriber.onComplete();
      else subscriber.onError(terminal);
    }
  }

  @Override public synchronized void close() {
    if (closed) return;
    closed = true; demand = 0;
    try { source.close(); } catch (RuntimeException failure) { cleanupFailure = failure; }
  }
}
