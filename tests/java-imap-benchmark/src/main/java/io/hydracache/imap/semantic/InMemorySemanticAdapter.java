package io.hydracache.imap.semantic;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.EnumSet;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.TreeMap;

/** Deterministic reference/fake adapter for validating the harness itself. */
public final class InMemorySemanticAdapter implements MapSemanticAdapter {
  public enum Defect { NONE, DROP_GET_AND_PUT }

  private final AdapterIdentity identity;
  private final Set<Operation> capabilities;
  private final Defect defect;
  private final TreeMap<BytesValue, StoredValue> values = new TreeMap<>();
  private final List<MapEvent> events = new ArrayList<>();
  private long logicalTick;

  private InMemorySemanticAdapter(String name, Set<Operation> capabilities, Defect defect) {
    identity = new AdapterIdentity("in-memory", name, "test-reference");
    this.capabilities = Set.copyOf(capabilities);
    this.defect = defect;
  }

  public static InMemorySemanticAdapter strict(String name) {
    return new InMemorySemanticAdapter(name, EnumSet.allOf(Operation.class), Defect.NONE);
  }

  public static InMemorySemanticAdapter withDefect(String name, Defect defect) {
    return new InMemorySemanticAdapter(name, EnumSet.allOf(Operation.class), defect);
  }

  public static InMemorySemanticAdapter withCapabilities(String name, Set<Operation> capabilities) {
    return new InMemorySemanticAdapter(name, capabilities, Defect.NONE);
  }

  @Override public AdapterIdentity identity() { return identity; }
  @Override public Set<Operation> capabilities() { return capabilities; }

  @Override
  public synchronized OperationOutcome execute(ScenarioStep step) {
    if (!capabilities.contains(step.operation())) {
      return OperationOutcome.error(ErrorClass.UNSUPPORTED, "operation not declared by adapter");
    }
    return switch (step.operation()) {
      case GET -> get(step.key());
      case CONTAINS_KEY -> containsKey(step.key());
      case PUT -> put(step.key(), step.value(), step.ttl(), false, false);
      case PUT_IF_ABSENT -> putIfAbsent(step);
      case REPLACE -> replace(step);
      case REPLACE_IF_PRESENT -> replaceIfPresent(step);
      case GET_AND_PUT -> put(step.key(), step.value(), step.ttl(), true,
          defect == Defect.DROP_GET_AND_PUT);
      case GET_AND_REMOVE -> remove(step.key());
      case REMOVE_IF_VALUE -> removeIfValue(step);
      case GET_ALL -> getAll(step.keys());
      case PUT_ALL -> putAll(step);
      case REMOVE_ALL -> removeAll(step.keys());
      case SET_TTL -> setTtl(step);
      case REMAINING_TTL -> remainingTtl(step.key());
      case LISTENER_GAP -> listenerGap(step.key());
      case ADVANCE -> advance(step.advanceTicks());
    };
  }

  @Override
  public synchronized List<MapEvent> drainEvents() {
    List<MapEvent> copy = List.copyOf(events);
    events.clear();
    return copy;
  }

  @Override
  public synchronized StateSnapshot snapshot() {
    purgeAllExpired();
    MessageDigest digest = sha256();
    for (Map.Entry<BytesValue, StoredValue> entry : values.entrySet()) {
      update(digest, entry.getKey().copy());
      update(digest, entry.getValue().value().copy());
      Long expiry = entry.getValue().expiresAt();
      digest.update((byte) (expiry == null ? 0 : 1));
      if (expiry != null) digest.update(ByteBuffer.allocate(Long.BYTES).putLong(expiry).array());
    }
    return new StateSnapshot(hex(digest.digest()), values.size(), logicalTick);
  }

  private OperationOutcome get(BytesValue key) {
    StoredValue stored = live(key);
    if (stored == null) return absent();
    return OperationOutcome.point(
        OutcomeKind.PRESENT, Optional.of(stored.value()), ttlState(stored));
  }

  private OperationOutcome containsKey(BytesValue key) {
    StoredValue stored = live(key);
    return OperationOutcome.point(stored == null ? OutcomeKind.ABSENT : OutcomeKind.PRESENT,
        Optional.empty(), stored == null ? TtlState.absent() : ttlState(stored));
  }

  private OperationOutcome putIfAbsent(ScenarioStep step) {
    StoredValue current = live(step.key());
    if (current != null) {
      return OperationOutcome.point(
          OutcomeKind.PRESENT, Optional.of(current.value()), ttlState(current));
    }
    return put(step.key(), step.value(), step.ttl(), true, false);
  }

  private OperationOutcome replace(ScenarioStep step) {
    StoredValue current = live(step.key());
    if (current == null) return absent();
    if (!current.value().equals(step.expected().orElseThrow())) {
      return OperationOutcome.point(
          OutcomeKind.MISMATCH, Optional.of(current.value()), ttlState(current));
    }
    return put(step.key(), step.value(), step.ttl(), true, false);
  }

  private OperationOutcome replaceIfPresent(ScenarioStep step) {
    StoredValue current = live(step.key());
    if (current == null) return absent();
    return put(step.key(), step.value(), step.ttl(), true, false);
  }

  private OperationOutcome setTtl(ScenarioStep step) {
    StoredValue current = live(step.key());
    if (current == null) return absent();
    StoredValue updated = new StoredValue(current.value(), expiryFor(step.ttl(), current));
    values.put(step.key(), updated);
    events.add(new MapEvent(MapEvent.Kind.UPDATED, step.key(), Optional.of(current.value()),
        logicalTick));
    return OperationOutcome.point(OutcomeKind.PRESENT, Optional.empty(), ttlState(updated));
  }

  private OperationOutcome remainingTtl(BytesValue key) {
    StoredValue current = live(key);
    if (current == null) return absent();
    return OperationOutcome.point(OutcomeKind.PRESENT, Optional.empty(), ttlState(current));
  }

  private OperationOutcome listenerGap(BytesValue key) {
    events.add(new MapEvent(MapEvent.Kind.GAP, key, Optional.empty(), logicalTick));
    return OperationOutcome.advanced();
  }

  private OperationOutcome put(
      BytesValue key, BytesValue value, TtlDirective ttl, boolean returnPrevious, boolean dropWrite) {
    StoredValue current = live(key);
    TtlState priorTtl = current == null ? TtlState.absent() : ttlState(current);
    Optional<BytesValue> previous = returnPrevious && current != null
        ? Optional.of(current.value()) : Optional.empty();
    OutcomeKind kind = current == null ? OutcomeKind.INSERTED : OutcomeKind.REPLACED;
    if (!dropWrite) {
      Long expiresAt = expiryFor(ttl, current);
      values.put(key, new StoredValue(value, expiresAt));
      events.add(new MapEvent(current == null ? MapEvent.Kind.ADDED : MapEvent.Kind.UPDATED,
          key, Optional.of(value), logicalTick));
    }
    return OperationOutcome.point(kind, previous, dropWrite ? priorTtl : ttlState(values.get(key)));
  }

  private OperationOutcome remove(BytesValue key) {
    StoredValue current = live(key);
    if (current == null) return absent();
    values.remove(key);
    events.add(new MapEvent(MapEvent.Kind.REMOVED, key, Optional.empty(), logicalTick));
    return OperationOutcome.point(
        OutcomeKind.REMOVED, Optional.of(current.value()), TtlState.absent());
  }

  private OperationOutcome removeIfValue(ScenarioStep step) {
    StoredValue current = live(step.key());
    if (current == null) return absent();
    if (!current.value().equals(step.expected().orElseThrow())) {
      return OperationOutcome.point(
          OutcomeKind.MISMATCH, Optional.of(current.value()), ttlState(current));
    }
    return remove(step.key());
  }

  private OperationOutcome getAll(List<BytesValue> keys) {
    var items = new ArrayList<ItemOutcome>(keys.size());
    for (int index = 0; index < keys.size(); index++) {
      BytesValue key = keys.get(index);
      StoredValue stored = live(key);
      items.add(stored == null
          ? new ItemOutcome(index, key, OutcomeKind.ABSENT, Optional.empty(), TtlState.absent())
          : new ItemOutcome(index, key, OutcomeKind.PRESENT, Optional.of(stored.value()),
              ttlState(stored)));
    }
    return OperationOutcome.bulk(items);
  }

  private OperationOutcome putAll(ScenarioStep step) {
    var seen = new HashSet<BytesValue>();
    for (ScenarioEntry entry : step.entries()) {
      if (!seen.add(entry.key())) {
        return OperationOutcome.error(ErrorClass.DUPLICATE_KEY,
            "duplicate key rejected before mutation");
      }
    }
    var items = new ArrayList<ItemOutcome>(step.entries().size());
    for (int index = 0; index < step.entries().size(); index++) {
      ScenarioEntry entry = step.entries().get(index);
      StoredValue current = live(entry.key());
      OutcomeKind kind = current == null ? OutcomeKind.INSERTED : OutcomeKind.REPLACED;
      Long expiry = expiryFor(step.ttl(), current);
      StoredValue stored = new StoredValue(entry.value(), expiry);
      values.put(entry.key(), stored);
      events.add(new MapEvent(current == null ? MapEvent.Kind.ADDED : MapEvent.Kind.UPDATED,
          entry.key(), Optional.of(entry.value()), logicalTick));
      items.add(new ItemOutcome(index, entry.key(), kind, Optional.empty(), ttlState(stored)));
    }
    return OperationOutcome.bulk(items);
  }

  private OperationOutcome removeAll(List<BytesValue> keys) {
    var items = new ArrayList<ItemOutcome>(keys.size());
    for (int index = 0; index < keys.size(); index++) {
      BytesValue key = keys.get(index);
      StoredValue current = live(key);
      if (current == null) {
        items.add(new ItemOutcome(
            index, key, OutcomeKind.ABSENT, Optional.empty(), TtlState.absent()));
      } else {
        values.remove(key);
        events.add(new MapEvent(MapEvent.Kind.REMOVED, key, Optional.empty(), logicalTick));
        items.add(new ItemOutcome(index, key, OutcomeKind.REMOVED,
            Optional.of(current.value()), TtlState.absent()));
      }
    }
    return OperationOutcome.bulk(items);
  }

  private OperationOutcome advance(long ticks) {
    logicalTick = Math.addExact(logicalTick, ticks);
    purgeAllExpired();
    return OperationOutcome.advanced();
  }

  private StoredValue live(BytesValue key) {
    StoredValue stored = values.get(key);
    if (stored != null && stored.expiresAt() != null && stored.expiresAt() <= logicalTick) {
      values.remove(key);
      events.add(new MapEvent(MapEvent.Kind.EXPIRED, key, Optional.empty(), logicalTick));
      return null;
    }
    return stored;
  }

  private void purgeAllExpired() {
    for (BytesValue key : List.copyOf(values.keySet())) live(key);
  }

  private Long expiryFor(TtlDirective directive, StoredValue current) {
    return switch (directive.kind()) {
      case PRESERVE -> current == null ? null : current.expiresAt();
      case ETERNAL -> null;
      case EXPIRE_AFTER -> Math.addExact(logicalTick, directive.ticks());
    };
  }

  private TtlState ttlState(StoredValue stored) {
    if (stored.expiresAt() == null) return TtlState.eternal();
    return TtlState.expiring(stored.expiresAt() - logicalTick);
  }

  private static OperationOutcome absent() {
    return OperationOutcome.point(OutcomeKind.ABSENT, Optional.empty(), TtlState.absent());
  }

  private static MessageDigest sha256() {
    try {
      return MessageDigest.getInstance("SHA-256");
    } catch (NoSuchAlgorithmException error) {
      throw new IllegalStateException("SHA-256 unavailable", error);
    }
  }

  private static void update(MessageDigest digest, byte[] value) {
    digest.update(ByteBuffer.allocate(Integer.BYTES).putInt(value.length).array());
    digest.update(value);
  }

  private static String hex(byte[] value) {
    return java.util.HexFormat.of().formatHex(value).toLowerCase(java.util.Locale.ROOT);
  }

  private record StoredValue(BytesValue value, Long expiresAt) {}
}
