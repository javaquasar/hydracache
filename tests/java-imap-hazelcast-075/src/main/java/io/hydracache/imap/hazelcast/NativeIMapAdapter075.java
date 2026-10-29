package io.hydracache.imap.hazelcast;

import com.hazelcast.core.EntryEvent;
import com.hazelcast.core.EntryListener;
import com.hazelcast.core.HazelcastInstance;
import com.hazelcast.map.IMap;
import com.hazelcast.map.MapEvent;
import io.hydracache.imap.semantic.BytesValue;
import java.util.*;
import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.atomic.AtomicReference;
import static io.hydracache.imap.hazelcast.NativeIMapObservation075.*;
import static io.hydracache.imap.hazelcast.NativeIMapProfile075.Operation.*;

/** Bounded, exclusively owned native fixture. All writes go through this adapter's writer monitor. */
public final class NativeIMapAdapter075 implements AutoCloseable {
  public enum Gap { QUEUE_OVERFLOW, PROFILE_VIOLATION, UNCERTAIN_WRITE }
  public static final class UncertainWrite extends IllegalStateException {
    private final List<PointReturn> completed;
    private final int attemptedIndex;
    UncertainWrite(List<PointReturn> completed, int attemptedIndex, RuntimeException cause) {
      super("native write outcome uncertain at item " + attemptedIndex, cause); this.completed = List.copyOf(completed); this.attemptedIndex = attemptedIndex;
    }
    public List<PointReturn> completed() { return completed; }
    public int attemptedIndex() { return attemptedIndex; }
  }
  private final IMap<byte[], byte[]> map;
  private final NativeIMapProfile075 profile;
  private final ArrayBlockingQueue<NativeEvent> events;
  private final AtomicReference<Gap> gap = new AtomicReference<>();
  private UUID listener;
  private long generation = 1;
  private boolean closed;
  public NativeIMapAdapter075(HazelcastInstance client, String mapName, NativeIMapProfile075 profile) {
    this(client, mapName, profile, 64);
  }
  NativeIMapAdapter075(HazelcastInstance client, String mapName, NativeIMapProfile075 profile, int queueCapacity) {
    this(officialMap(client, mapName), profile, queueCapacity);
  }
  private static IMap<byte[], byte[]> officialMap(HazelcastInstance client, String mapName) {
    Objects.requireNonNull(client); HazelcastArtifactPin.verifyLoadedArtifact();
    if (mapName == null || !mapName.matches("imap075-[a-zA-Z0-9-]{1,80}")) throw new IllegalArgumentException("native fixture bounds");
    if (com.hazelcast.client.HazelcastClient.getHazelcastClientByName(client.getName()) != client) throw new IllegalArgumentException("official client required");
    return client.getMap(mapName);
  }
  NativeIMapAdapter075(IMap<byte[], byte[]> map, NativeIMapProfile075 profile, int queueCapacity) {
    this.profile = Objects.requireNonNull(profile); this.map = Objects.requireNonNull(map);
    if (queueCapacity < 1 || queueCapacity > 1024) throw new IllegalArgumentException("native queue bound");
    if (!map.isEmpty()) throw new IllegalArgumentException("native map must start empty");
    events = new ArrayBlockingQueue<>(queueCapacity); listener = map.addEntryListener(new NativeEvents(generation), true);
  }
  public NativeIMapProfile075 profile() { return profile; }
  public synchronized long registrationGeneration() { return generation; }
  public Optional<Gap> gap() { return Optional.ofNullable(gap.get()); }
  private void open() { if (closed) throw new IllegalStateException("native adapter closed"); }
  private Result result(Request request, Origin origin, int calls, RawReturn value) { return new Result(request.operation(), origin, OptionalInt.of(calls), value); }
  /** Request constructors validate the whole immutable request before any map invocation. */
  public synchronized Result execute(Request request) {
    open(); Objects.requireNonNull(request);
    var before = snapshot();
    var futureKeys = new HashSet<BytesValue>(); before.entries().forEach(e -> futureKeys.add(e.key()));
    if (request.operation() == PUT_DETAILED || request.operation() == PUT_ALL_DIAGNOSTIC) request.entries().forEach(e -> futureKeys.add(e.key()));
    if (futureKeys.size() > 64) throw new IllegalArgumentException("native prospective cardinality bound");
    return switch (request.operation()) {
      case SIZE -> result(request, Origin.NATIVE_SINGLE, 1, new NumberReturn(map.size()));
      case IS_EMPTY -> result(request, Origin.NATIVE_SINGLE, 1, new BooleanReturn(map.isEmpty()));
      case GET_OR_DEFAULT -> {
        byte[] value = map.get(request.keys().get(0).copy());
        yield result(request, Origin.CLIENT_READ_HELPER, 1, new ValueReturn(value == null ? request.argument() : optional(value)));
      }
      case CONTAINS_ENTRY -> result(request, Origin.CLIENT_READ_HELPER, 1,
          new BooleanReturn(optional(map.get(request.keys().get(0).copy())).equals(request.argument())));
      case PUT_DETAILED, REMOVE_DETAILED -> {
        var completed = new ArrayList<PointReturn>(); int count = request.operation() == PUT_DETAILED ? request.entries().size() : request.keys().size();
        for (int index = 0; index < count; index++) {
          var key = request.operation() == PUT_DETAILED ? request.entries().get(index).key() : request.keys().get(index);
          try {
            byte[] previous = request.operation() == PUT_DETAILED ? map.put(key.copy(), request.entries().get(index).value().copy(), 0, java.util.concurrent.TimeUnit.MILLISECONDS) : map.remove(key.copy());
            completed.add(new PointReturn(index, key, optional(previous)));
          } catch (RuntimeException uncertain) { gap.set(Gap.UNCERTAIN_WRITE); throw new UncertainWrite(completed, index, uncertain); }
        }
        yield result(request, Origin.CLIENT_POINT_HELPER, count, new PointReturns(completed));
      }
      case PUT_ALL_DIAGNOSTIC -> {
        var values = new LinkedHashMap<byte[], byte[]>(); request.entries().forEach(e -> values.put(e.key().copy(), e.value().copy()));
        try { map.putAll(values); } catch (RuntimeException uncertain) { gap.set(Gap.UNCERTAIN_WRITE); throw new UncertainWrite(List.of(), 0, uncertain); }
        yield result(request, Origin.NATIVE_AGGREGATE, 1, new VoidReturn());
      }
      case ENTRY_VIEWS -> result(request, Origin.CLIENT_READ_HELPER, request.keys().size(), new ViewsReturn(views(request.keys())));
      case SCAN -> {
        if (before.entries().size() > request.maxItems()) throw new IllegalArgumentException("native scan result bound");
        yield new Result(request.operation(), Origin.CLIENT_READ_HELPER, OptionalInt.empty(), new ViewsReturn(views(before.entries().stream().map(Entry::key).toList())));
      }
      case CLEAR, EVICT, EVICT_ALL -> {
        try {
          if (request.operation() == EVICT) yield result(request, Origin.NATIVE_SINGLE, 1, new BooleanReturn(map.evict(request.keys().get(0).copy())));
          if (request.operation() == CLEAR) map.clear(); else map.evictAll();
          yield result(request, Origin.NATIVE_AGGREGATE, 1, new VoidReturn());
        } catch (RuntimeException uncertain) { gap.set(Gap.UNCERTAIN_WRITE); throw new UncertainWrite(List.of(), 0, uncertain); }
      }
    };
  }
  private List<NativeView> views(List<BytesValue> keys) {
    var views = new ArrayList<NativeView>();
    for (var key : keys) {
      var view = map.getEntryView(key.copy());
      if (view != null) { eternal(view.getTtl()); views.add(new NativeView(key, BytesValue.copyOf(view.getValue()), view.getVersion(), view.getTtl())); }
    }
    return List.copyOf(views);
  }
  private void eternal(long ttl) {
    if (ttl != 0 && ttl != Long.MAX_VALUE) { gap.set(Gap.PROFILE_VIOLATION); throw new IllegalStateException("native eternal profile violated"); }
  }
  public synchronized Snapshot snapshot() {
    open(); var entries = new TreeMap<BytesValue, BytesValue>(); var iterator = map.iterator(16);
    while (iterator.hasNext()) {
      if (entries.size() == 64) throw new IllegalStateException("native snapshot cardinality bound");
      var entry = iterator.next();
      if (entry.getKey().length > 1024 || entry.getValue().length > 65536) { gap.set(Gap.PROFILE_VIOLATION); throw new IllegalStateException("native snapshot payload bound"); }
      var view = map.getEntryView(entry.getKey());
      if (view == null) throw new IllegalStateException("native snapshot requires quiescence"); eternal(view.getTtl());
      entries.put(BytesValue.copyOf(entry.getKey()), BytesValue.copyOf(entry.getValue()));
    }
    var values = entries.entrySet().stream().map(e -> new Entry(e.getKey(), e.getValue())).toList();
    return new Snapshot(values, digestEntries(values));
  }
  public synchronized List<NativeEvent> drainEvents() {
    open(); if (gap.get() != null) throw new IllegalStateException("native observation gap: " + gap.get());
    var drained = new ArrayList<NativeEvent>(); events.drainTo(drained); return List.copyOf(drained);
  }
  @Override public synchronized void close() {
    if (!closed) { closed = true; try { map.removeEntryListener(listener); } finally { events.clear(); } }
  }
  private final class NativeEvents implements EntryListener<byte[], byte[]> {
    private final long registration;
    NativeEvents(long registration) { this.registration = registration; }
    private void add(EntryEvent<byte[], byte[]> event, EventKind kind) {
      byte[] key = event.getKey(), value = event.getValue(), old = event.getOldValue();
      if (key == null || key.length > 1024 || value != null && value.length > 65536 || old != null && old.length > 65536) { gap.set(Gap.PROFILE_VIOLATION); return; }
      offer(new NativeEvent(registration, kind, optional(key), optional(old), optional(value), 1));
    }
    public void entryAdded(EntryEvent<byte[], byte[]> event) { add(event, EventKind.ADDED); }
    public void entryUpdated(EntryEvent<byte[], byte[]> event) { add(event, EventKind.UPDATED); }
    public void entryRemoved(EntryEvent<byte[], byte[]> event) { add(event, EventKind.REMOVED); }
    public void entryExpired(EntryEvent<byte[], byte[]> event) { gap.set(Gap.PROFILE_VIOLATION); add(event, EventKind.EXPIRED); }
    public void entryEvicted(EntryEvent<byte[], byte[]> event) { add(event, EventKind.EVICTED); }
    public void mapCleared(MapEvent event) { offer(new NativeEvent(registration, EventKind.MAP_CLEARED, Optional.empty(), Optional.empty(), Optional.empty(), event.getNumberOfEntriesAffected())); }
    public void mapEvicted(MapEvent event) { offer(new NativeEvent(registration, EventKind.MAP_EVICTED, Optional.empty(), Optional.empty(), Optional.empty(), event.getNumberOfEntriesAffected())); }
  }
  private void offer(NativeEvent event) { if (!events.offer(event)) gap.compareAndSet(null, Gap.QUEUE_OVERFLOW); }
}
