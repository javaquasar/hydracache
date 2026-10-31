package io.hydracache.imap.hazelcast;

import com.hazelcast.core.EntryEvent;
import com.hazelcast.core.EntryListener;
import com.hazelcast.core.HazelcastInstance;
import com.hazelcast.map.IMap;
import com.hazelcast.map.MapEvent;
import io.hydracache.imap.semantic.BytesValue;
import java.time.Duration;
import java.util.*;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.locks.ReentrantLock;
import static io.hydracache.imap.hazelcast.NativeIMapObservation075.*;

/** Controlled point-writer fixture. Generations identify local registrations, never server cuts. */
public final class NativeListenerRecovery075 implements AutoCloseable {
  public enum State { HEALTHY, GAP, RECOVERING, CLOSED }
  public enum Reason { QUEUE_OVERFLOW, CORRELATION_MISMATCH, LEDGER_BOUND, PROFILE_VIOLATION,
    UNCERTAIN_WRITE, FOREIGN_STATE, MISSING_CALLBACK, DEADLINE, CANCELLED, REGISTRATION_FAILED, BASELINE_BOUND }
  public static final class Cancellation {
    private final AtomicBoolean cancelled = new AtomicBoolean();
    public void cancel() { cancelled.set(true); }
    public boolean cancelled() { return cancelled.get(); }
  }
  public record Bounds(int maxItems, long maxBytes, Duration timeout) {
    public Bounds {
      Objects.requireNonNull(timeout);
      if (maxItems < 1 || maxItems > 64 || maxBytes < 1 || maxBytes > NativeIMapProfile075.MAX_REQUEST_BYTES
          || timeout.isZero() || timeout.isNegative() || timeout.compareTo(Duration.ofSeconds(5)) > 0) throw new IllegalArgumentException("native recovery bounds");
    }
    public static Bounds smoke() { return new Bounds(64, NativeIMapProfile075.MAX_REQUEST_BYTES, Duration.ofSeconds(5)); }
  }
  public record Recovery(long oldGeneration, long newGeneration, int correlatedWrites, Snapshot baseline) {}
  record Tuple(EventKind kind, BytesValue key, BytesValue value) {}
  private final IMap<byte[], byte[]> map;
  private final int queueCapacity, ledgerCapacity;
  private final ReentrantLock writerGate = new ReentrantLock(true);
  private final Object monitor = new Object();
  private final AtomicBoolean closeRequested = new AtomicBoolean();
  private final ArrayDeque<NativeEvent> queue = new ArrayDeque<>();
  private final Set<Tuple> expected = new HashSet<>(), observed = new HashSet<>();
  private final TreeMap<BytesValue, BytesValue> owned = new TreeMap<>();
  private State state = State.HEALTHY; private Reason reason; private volatile long generation = 1; private UUID listener;
  private boolean waitingForCallbacks;
  private int observedCallbacks;
  private int removedSubscriptions;
  public NativeListenerRecovery075(HazelcastInstance client, String mapName, int queueCapacity) {
    this(NativeIMapAdapter075.officialMap(client, mapName), queueCapacity, 1024);
  }
  NativeListenerRecovery075(IMap<byte[], byte[]> map, int queueCapacity, int ledgerCapacity) {
    if (queueCapacity < 1 || queueCapacity > 1024 || ledgerCapacity < 1 || ledgerCapacity > 1024) throw new IllegalArgumentException("native listener bound");
    this.map = Objects.requireNonNull(map); this.queueCapacity = queueCapacity; this.ledgerCapacity = ledgerCapacity;
    if (!map.isEmpty()) throw new IllegalArgumentException("controlled listener map must start empty");
    listener = Objects.requireNonNull(map.addEntryListener(new Events(generation), true));
  }
  public State state() { synchronized (monitor) { return state; } }
  public Optional<Reason> reason() { synchronized (monitor) { return Optional.ofNullable(reason); } }
  public long generation() { synchronized (monitor) { return generation; } }
  public int expectedCallbacks() { synchronized (monitor) { return expected.size(); } }
  public int observedCallbacks() { synchronized (monitor) { return observedCallbacks; } }
  public int removedSubscriptions() { synchronized (monitor) { return removedSubscriptions; } }
  public int queuedEvents() { synchronized (monitor) { return queue.size(); } }
  public int ownedKeys() { writerGate.lock(); try { return owned.size(); } finally { writerGate.unlock(); } }
  public int ownedSubscriptions() { writerGate.lock(); try { return listener == null ? 0 : 1; } finally { writerGate.unlock(); } }
  public boolean waitingForCallbacks() { synchronized (monitor) { return waitingForCallbacks; } }
  private void open() { if (state == State.CLOSED || closeRequested.get()) throw new IllegalStateException("native listener closed"); }
  private void fail(Reason cause) { synchronized (monitor) { if (state != State.CLOSED) { state = State.GAP; reason = cause; monitor.notifyAll(); } } }
  private IllegalStateException refusal(Reason cause) { fail(cause); return new IllegalStateException("native recovery refused: " + cause); }
  /** Only this gate admits writes; no caller-supplied paused/quiescent flag exists. */
  public Optional<BytesValue> put(BytesValue key, BytesValue value) {
    keyBound(key); Objects.requireNonNull(value);
    if (value.size() > 65536) throw new IllegalArgumentException("native value bound");
    writerGate.lock();
    try {
      synchronized (monitor) { open(); if (state != State.HEALTHY && !(state == State.GAP && reason == Reason.QUEUE_OVERFLOW)) throw new IllegalStateException("native writer blocked by gap"); }
      var before = capture(Bounds.smoke()); if (!sameOwned(before)) throw refusal(Reason.FOREIGN_STATE);
      if (!owned.containsKey(key) && owned.size() == 64) throw new IllegalArgumentException("native cardinality bound");
      var tuple = new Tuple(owned.containsKey(key) ? EventKind.UPDATED : EventKind.ADDED, key, value);
      synchronized (monitor) {
        if (expected.size() == ledgerCapacity) throw refusal(Reason.LEDGER_BOUND);
        if (!expected.add(tuple)) throw new IllegalArgumentException("controlled workload requires unique callback tuples");
      }
      byte[] previous;
      try { previous = map.put(key.copy(), value.copy(), 0, TimeUnit.MILLISECONDS); }
      catch (RuntimeException uncertain) { fail(Reason.UNCERTAIN_WRITE); throw new IllegalStateException("native write outcome uncertain", uncertain); }
      if (!optional(previous).equals(Optional.ofNullable(owned.get(key)))) throw refusal(Reason.FOREIGN_STATE);
      owned.put(key, value); return optional(previous);
    } finally { writerGate.unlock(); }
  }
  public List<NativeEvent> drainEvents() {
    synchronized (monitor) {
      open(); if (state != State.HEALTHY) throw new IllegalStateException("native observation gap: " + reason);
      var result = List.copyOf(queue); queue.clear(); return result;
    }
  }
  private void accept(NativeEvent event) {
    synchronized (monitor) {
      if (state == State.CLOSED || closeRequested.get() || event.registrationGeneration() != generation) return;
      observedCallbacks = Math.min(ledgerCapacity + 1, observedCallbacks + 1);
      if (event.kind() != EventKind.ADDED && event.kind() != EventKind.UPDATED || event.key().isEmpty() || event.value().isEmpty()) { fail(Reason.PROFILE_VIOLATION); return; }
      var tuple = new Tuple(event.kind(), event.key().orElseThrow(), event.value().orElseThrow());
      if (!expected.contains(tuple) || !observed.add(tuple)) { fail(Reason.CORRELATION_MISMATCH); return; }
      if (queue.size() == queueCapacity) { if (state == State.HEALTHY) fail(Reason.QUEUE_OVERFLOW); }
      else queue.addLast(event);
      monitor.notifyAll();
    }
  }
  private static long remaining(long deadline) { return Math.max(0, deadline - System.nanoTime()); }
  private void continuing(long deadline, Cancellation cancellation) {
    if (closeRequested.get() || cancellation.cancelled()) throw refusal(Reason.CANCELLED);
    if (remaining(deadline) == 0) throw refusal(Reason.DEADLINE);
  }
  public Recovery recover(Bounds bounds, Cancellation cancellation) throws InterruptedException {
    Objects.requireNonNull(bounds); Objects.requireNonNull(cancellation); long deadline = System.nanoTime() + bounds.timeout().toNanos();
    try { if (!writerGate.tryLock(remaining(deadline), TimeUnit.NANOSECONDS)) throw refusal(Reason.DEADLINE); }
    catch (InterruptedException interrupted) { fail(Reason.CANCELLED); throw interrupted; }
    try {
      long old; int correlated;
      synchronized (monitor) {
        open(); if (state != State.GAP || reason != Reason.QUEUE_OVERFLOW) throw new IllegalStateException("only correlated queue overflow can recover");
        while (!expected.equals(observed)) {
          if (closeRequested.get() || cancellation.cancelled()) throw refusal(Reason.CANCELLED);
          if (reason != Reason.QUEUE_OVERFLOW) throw new IllegalStateException("native callback correlation failed");
          long left = remaining(deadline);
          if (left == 0) throw refusal(Reason.MISSING_CALLBACK);
          waitingForCallbacks = true;
          try { TimeUnit.NANOSECONDS.timedWait(monitor, Math.min(left, TimeUnit.MILLISECONDS.toNanos(10))); }
          finally { waitingForCallbacks = false; }
        }
        if (reason != Reason.QUEUE_OVERFLOW) throw new IllegalStateException("native callback correlation failed");
        continuing(deadline, cancellation); old = generation; correlated = expected.size(); state = State.RECOVERING;
      }
      try {
        if (!map.removeEntryListener(listener)) throw refusal(Reason.REGISTRATION_FAILED); listener = null;
        synchronized (monitor) { removedSubscriptions++; }
        continuing(deadline, cancellation);
        synchronized (monitor) { generation = Math.incrementExact(generation); queue.clear(); expected.clear(); observed.clear(); observedCallbacks = 0; }
        Snapshot baseline;
        try { baseline = capture(bounds, deadline); } catch (RuntimeException invalid) { throw refusal(Reason.BASELINE_BOUND); }
        if (!sameOwned(baseline)) throw refusal(Reason.FOREIGN_STATE);
        continuing(deadline, cancellation); listener = Objects.requireNonNull(map.addEntryListener(new Events(generation), true));
        synchronized (monitor) {
          continuing(deadline, cancellation);
          if (state != State.RECOVERING) throw new IllegalStateException("foreign callback during registration");
          reason = null; state = State.HEALTHY;
        }
        return new Recovery(old, generation, correlated, baseline);
      } catch (RuntimeException failure) {
        synchronized (monitor) { if (state == State.RECOVERING) fail(Reason.REGISTRATION_FAILED); }
        throw failure;
      }
    } catch (InterruptedException interrupted) { fail(Reason.CANCELLED); throw interrupted; }
    finally { writerGate.unlock(); }
  }
  private boolean sameOwned(Snapshot snapshot) {
    var entries = owned.entrySet().stream().map(e -> new Entry(e.getKey(), e.getValue())).toList(); return snapshot.digest().equals(digestEntries(entries));
  }
  private Snapshot capture(Bounds bounds) {
    return capture(bounds, System.nanoTime() + bounds.timeout().toNanos());
  }
  private Snapshot capture(Bounds bounds, long deadline) {
    var entries = new TreeMap<BytesValue, BytesValue>(); long bytes = 0; var iterator = map.iterator(16);
    while (iterator.hasNext()) {
      if (remaining(deadline) == 0) throw refusal(Reason.DEADLINE);
      if (entries.size() == bounds.maxItems()) throw new IllegalStateException("native baseline cardinality bound");
      var entry = iterator.next(); bytes = Math.addExact(bytes, 9L + entry.getKey().length + entry.getValue().length);
      if (entry.getKey().length > 1024 || entry.getValue().length > 65536 || bytes > bounds.maxBytes()) throw new IllegalStateException("native baseline payload bound");
      var view = map.getEntryView(entry.getKey());
      if (view == null || view.getTtl() != 0 && view.getTtl() != Long.MAX_VALUE) { fail(Reason.PROFILE_VIOLATION); throw new IllegalStateException("native eternal baseline violated"); }
      entries.put(BytesValue.copyOf(entry.getKey()), BytesValue.copyOf(entry.getValue()));
    }
    var values = entries.entrySet().stream().map(e -> new Entry(e.getKey(), e.getValue())).toList(); return new Snapshot(values, digestEntries(values));
  }
  @Override public void close() {
    closeRequested.set(true); synchronized (monitor) { monitor.notifyAll(); }
    writerGate.lock();
    try {
      synchronized (monitor) { if (state == State.CLOSED) return; state = State.CLOSED; }
      try { if (listener != null) {
        if (!map.removeEntryListener(listener)) throw new IllegalStateException("native subscription cleanup incomplete");
        listener = null; synchronized (monitor) { removedSubscriptions++; }
      } }
      finally { synchronized (monitor) { queue.clear(); expected.clear(); observed.clear(); owned.clear(); } }
    } finally { writerGate.unlock(); }
  }
  private final class Events implements EntryListener<byte[], byte[]> {
    private final long registration;
    Events(long registration) { this.registration = registration; }
    private void entry(EntryEvent<byte[], byte[]> event, EventKind kind) {
      byte[] key = event.getKey(), old = event.getOldValue(), value = event.getValue();
      if (key == null || key.length > 1024 || old != null && old.length > 65536 || value != null && value.length > 65536) { if (registration == generation) fail(Reason.PROFILE_VIOLATION); return; }
      accept(new NativeEvent(registration, kind, optional(key), optional(old), optional(value), 1));
    }
    public void entryAdded(EntryEvent<byte[], byte[]> event) { entry(event, EventKind.ADDED); }
    public void entryUpdated(EntryEvent<byte[], byte[]> event) { entry(event, EventKind.UPDATED); }
    public void entryRemoved(EntryEvent<byte[], byte[]> event) { entry(event, EventKind.REMOVED); }
    public void entryExpired(EntryEvent<byte[], byte[]> event) { entry(event, EventKind.EXPIRED); }
    public void entryEvicted(EntryEvent<byte[], byte[]> event) { entry(event, EventKind.EVICTED); }
    public void mapCleared(MapEvent event) { accept(new NativeEvent(registration, EventKind.MAP_CLEARED, Optional.empty(), Optional.empty(), Optional.empty(), event.getNumberOfEntriesAffected())); }
    public void mapEvicted(MapEvent event) { accept(new NativeEvent(registration, EventKind.MAP_EVICTED, Optional.empty(), Optional.empty(), Optional.empty(), event.getNumberOfEntriesAffected())); }
  }
}
