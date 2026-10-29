package io.hydracache.imap.hazelcast;

import com.hazelcast.core.EntryEvent;
import com.hazelcast.core.EntryListener;
import com.hazelcast.core.HazelcastInstance;
import com.hazelcast.map.IMap;
import com.hazelcast.map.MapEvent;
import io.hydracache.imap.semantic.*;
import java.nio.ByteBuffer;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.*;
import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.atomic.AtomicBoolean;

/** Real official-client adapter for exclusive, sequential, eternal-value diagnostic scenarios.
 * It makes no retained-identity retry, TLS, backup, timing, or S2 qualification claim.
 * Conditional failure observations include a subsequent native read, so this profile is not
 * admitted for concurrent histories. Native listener events are never synthesized from writes.
 */
public final class HazelcastProductAdapter075 implements MapSemanticAdapter {
  private enum NativeOperation { GET, CONTAINS_KEY, PUT, PUT_IF_ABSENT, REPLACE, REPLACE_IF_PRESENT, GET_AND_PUT, GET_AND_REMOVE, REMOVE_IF_VALUE, GET_ALL }
  private static final Set<Operation> OPS = Set.of(Operation.GET, Operation.CONTAINS_KEY, Operation.PUT,
      Operation.PUT_IF_ABSENT, Operation.REPLACE, Operation.REPLACE_IF_PRESENT, Operation.GET_AND_PUT,
      Operation.GET_AND_REMOVE, Operation.REMOVE_IF_VALUE, Operation.GET_ALL);
  private final IMap<byte[], byte[]> map;
  private final UUID listener;
  private final ArrayBlockingQueue<io.hydracache.imap.semantic.MapEvent> events = new ArrayBlockingQueue<>(64);
  private final AtomicBoolean overflow = new AtomicBoolean();
  private boolean closed;

  public HazelcastProductAdapter075(HazelcastInstance officialClient, String mapName) {
    HazelcastArtifactPin.verifyLoadedArtifact();
    Objects.requireNonNull(officialClient);
    if (mapName == null || !mapName.matches("imap075-[a-zA-Z0-9-]{1,80}")) throw new IllegalArgumentException("isolated map name required");
    if (com.hazelcast.client.HazelcastClient.getHazelcastClientByName(officialClient.getName()) != officialClient)
      throw new IllegalArgumentException("an official Hazelcast client is required");
    map = officialClient.getMap(mapName);
    if (!map.isEmpty()) throw new IllegalArgumentException("diagnostic map must start empty");
    listener = map.addEntryListener(new NativeEvents(), true);
  }

  @Override public AdapterIdentity identity() {
    return new AdapterIdentity("hazelcast", "official-client-" + HazelcastArtifactPin.VERSION,
        "diagnostic-exclusive-eternal-not-release-qualified");
  }
  @Override public Set<Operation> capabilities() { return OPS; }
  @Override public synchronized OperationOutcome execute(ScenarioStep step) {
    requireOpen(); Objects.requireNonNull(step);
    if (step.ttl().kind() == TtlDirective.Kind.EXPIRE_AFTER)
      return OperationOutcome.error(ErrorClass.UNSUPPORTED, "native wall-clock TTL/logical advance/bulk mutations are outside this diagnostic profile");
    NativeOperation nativeOperation;
    try { nativeOperation=NativeOperation.valueOf(step.operation().name()); }
    catch(IllegalArgumentException unsupported) {return OperationOutcome.error(ErrorClass.UNSUPPORTED,"operation outside the native diagnostic profile");}
    if (step.keys().size() > 64 || step.entries().size() > 1 || step.keys().stream().anyMatch(key -> key.size() > 1024)
        || step.entries().stream().anyMatch(entry -> entry.key().size() > 1024 || entry.value().size() > 65536)
        || step.expected().stream().anyMatch(value -> value.size() > 65536))
      return OperationOutcome.error(ErrorClass.SIZE, "diagnostic request bound");
    byte[] key = step.key().copy();
    if (new HashSet<>(step.keys()).size() != step.keys().size())
      return OperationOutcome.error(ErrorClass.DUPLICATE_KEY, "duplicate key rejected before mutation");
    for (var observedKey : step.operation() == Operation.GET_ALL ? step.keys() : List.of(step.key())) {
      var view = map.getEntryView(observedKey.copy());
      if (view != null && view.getTtl() != 0 && view.getTtl() != Long.MAX_VALUE) return OperationOutcome.error(ErrorClass.UNSUPPORTED, "finite native TTL is outside the eternal profile");
    }
    if (step.ttl().kind() == TtlDirective.Kind.PRESERVE
        && Set.of(Operation.PUT, Operation.PUT_IF_ABSENT, Operation.GET_AND_PUT).contains(step.operation())
        && !map.containsKey(key)) return OperationOutcome.error(ErrorClass.INVALID_ARGUMENT, "preserve requires existing entry");
    return switch (nativeOperation) {
      case GET: yield present(map.get(key), true);
      case CONTAINS_KEY: yield present(map.get(key), false);
      case GET_ALL: {
        Set<byte[]> keys = new HashSet<>(); step.keys().forEach(value -> keys.add(value.copy()));
        Map<BytesValue, byte[]> values = new HashMap<>(); map.getAll(keys).forEach((k, v) -> values.put(BytesValue.copyOf(k), v));
        List<ItemOutcome> items = new ArrayList<>();
        for (int i = 0; i < step.keys().size(); i++) {
          var k = step.keys().get(i); var result = present(values.get(k), true);
          items.add(new ItemOutcome(i, k, result.kind(), result.value(), result.ttl()));
        }
        yield OperationOutcome.bulk(items);
      }
      case PUT, GET_AND_PUT, PUT_IF_ABSENT: {
        if (map.size() >= 64 && !map.containsKey(key)) yield OperationOutcome.error(ErrorClass.SIZE, "diagnostic cardinality bound");
        byte[] previous = step.operation() == Operation.PUT_IF_ABSENT
            ? map.putIfAbsent(key, step.value().copy(), 0, java.util.concurrent.TimeUnit.MILLISECONDS) : map.put(key, step.value().copy(), 0, java.util.concurrent.TimeUnit.MILLISECONDS);
        if (step.operation() == Operation.PUT_IF_ABSENT && previous != null) yield present(previous, true);
        yield OperationOutcome.point(previous == null ? OutcomeKind.INSERTED : OutcomeKind.REPLACED,
            step.operation() == Operation.GET_AND_PUT ? optional(previous) : Optional.empty(), TtlState.eternal());
      }
      case REPLACE: {
        if (map.replace(key, step.expected().orElseThrow().copy(), step.value().copy()))
          yield OperationOutcome.point(OutcomeKind.REPLACED, step.expected(), TtlState.eternal());
        byte[] current = map.get(key);
        yield current == null ? absent() : OperationOutcome.point(OutcomeKind.MISMATCH, optional(current), TtlState.eternal());
      }
      case REPLACE_IF_PRESENT: {
        byte[] previous = map.replace(key, step.value().copy());
        yield previous == null ? absent() : OperationOutcome.point(OutcomeKind.REPLACED, optional(previous), TtlState.eternal());
      }
      case GET_AND_REMOVE: {
        byte[] previous = map.remove(key);
        yield previous == null ? absent() : OperationOutcome.point(OutcomeKind.REMOVED, optional(previous), TtlState.absent());
      }
      case REMOVE_IF_VALUE: {
        if (map.remove(key, step.expected().orElseThrow().copy()))
          yield OperationOutcome.point(OutcomeKind.REMOVED, step.expected(), TtlState.absent());
        byte[] current = map.get(key);
        yield current == null ? absent() : OperationOutcome.point(OutcomeKind.MISMATCH, optional(current), TtlState.eternal());
      }

    };
  }

  @Override public synchronized List<io.hydracache.imap.semantic.MapEvent> drainEvents() {
    requireOpen();
    if (overflow.get()) throw new IllegalStateException("native listener gap: diagnostic queue overflow");
    var drained = new ArrayList<io.hydracache.imap.semantic.MapEvent>(); events.drainTo(drained); return List.copyOf(drained);
  }
  @Override public synchronized StateSnapshot snapshot() {
    requireOpen(); var ordered = new TreeMap<BytesValue, BytesValue>();
    var iterator = map.iterator(16);
    while (iterator.hasNext()) {
      if (ordered.size() == 64) throw new IllegalStateException("diagnostic snapshot cardinality bound");
      var entry = iterator.next();
      if (entry.getKey().length > 1024 || entry.getValue().length > 65536) throw new IllegalStateException("diagnostic snapshot payload bound");
      var view = map.getEntryView(entry.getKey());
      if (view == null || view.getTtl() != 0 && view.getTtl() != Long.MAX_VALUE)
        throw new IllegalStateException("diagnostic profile requires stable eternal entries");
      ordered.put(BytesValue.copyOf(entry.getKey()), BytesValue.copyOf(entry.getValue()));
    }
    MessageDigest digest;
    try { digest = MessageDigest.getInstance("SHA-256"); } catch (NoSuchAlgorithmException failure) { throw new IllegalStateException(failure); }
    ordered.forEach((key, value) -> { update(digest, key.copy()); update(digest, value.copy()); digest.update((byte) 0); });
    return new StateSnapshot(HexFormat.of().formatHex(digest.digest()), ordered.size(), 0);
  }
  @Override public synchronized void close() { if (!closed) { map.removeEntryListener(listener); closed = true; events.clear(); } }
  private void requireOpen() { if (closed) throw new IllegalStateException("adapter closed"); }
  private static Optional<BytesValue> optional(byte[] value) { return value == null ? Optional.empty() : Optional.of(BytesValue.copyOf(value)); }
  private static OperationOutcome absent() { return OperationOutcome.point(OutcomeKind.ABSENT, Optional.empty(), TtlState.absent()); }
  private static OperationOutcome present(byte[] value, boolean include) {
    return value == null ? absent() : OperationOutcome.point(OutcomeKind.PRESENT, include ? optional(value) : Optional.empty(), TtlState.eternal());
  }
  private static void update(MessageDigest digest, byte[] bytes) { digest.update(ByteBuffer.allocate(4).putInt(bytes.length).array()); digest.update(bytes); }
  private final class NativeEvents implements EntryListener<byte[], byte[]> {
    private void add(EntryEvent<byte[], byte[]> event, io.hydracache.imap.semantic.MapEvent.Kind kind, boolean value) {
      byte[] key = event.getKey(); byte[] payload = event.getValue();
      if (key.length > 1024 || (payload != null && payload.length > 65536)
          || !events.offer(new io.hydracache.imap.semantic.MapEvent(kind, BytesValue.copyOf(key), value ? optional(payload) : Optional.empty(), 0))) overflow.set(true);
    }
    public void entryAdded(EntryEvent<byte[], byte[]> event) { add(event, io.hydracache.imap.semantic.MapEvent.Kind.ADDED, true); }
    public void entryUpdated(EntryEvent<byte[], byte[]> event) { add(event, io.hydracache.imap.semantic.MapEvent.Kind.UPDATED, true); }
    public void entryRemoved(EntryEvent<byte[], byte[]> event) { add(event, io.hydracache.imap.semantic.MapEvent.Kind.REMOVED, false); }
    public void entryExpired(EntryEvent<byte[], byte[]> event) { overflow.set(true); }
    public void entryEvicted(EntryEvent<byte[], byte[]> event) { overflow.set(true); }
    public void mapEvicted(MapEvent event) { overflow.set(true); }
    public void mapCleared(MapEvent event) { overflow.set(true); }
  }
}
