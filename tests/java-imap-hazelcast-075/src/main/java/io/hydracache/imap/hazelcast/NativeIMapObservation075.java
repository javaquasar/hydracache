package io.hydracache.imap.hazelcast;

import io.hydracache.imap.semantic.BytesValue;
import java.nio.ByteBuffer;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.*;

/** Actual native observations. There are deliberately no Hydra owner/token/watermark fields. */
public final class NativeIMapObservation075 {
  private NativeIMapObservation075() {}
  public enum Origin { NATIVE_SINGLE, CLIENT_POINT_HELPER, CLIENT_READ_HELPER, NATIVE_AGGREGATE }
  public record Entry(BytesValue key, BytesValue value) { public Entry { Objects.requireNonNull(key); Objects.requireNonNull(value); } }
  public record Request(NativeIMapProfile075.Operation operation, List<BytesValue> keys, List<Entry> entries,
      Optional<BytesValue> argument, int maxItems) {
    public Request {
      Objects.requireNonNull(operation); Objects.requireNonNull(argument);
      if (keys.size() > NativeIMapProfile075.MAX_KEYS || entries.size() > NativeIMapProfile075.MAX_KEYS)
        throw new IllegalArgumentException("native request count bound");
      keys = List.copyOf(keys); entries = List.copyOf(entries);
      if (maxItems < 1 || maxItems > NativeIMapProfile075.MAX_KEYS) throw new IllegalArgumentException("native scan bound");
      long bytes = argument.map(BytesValue::size).orElse(0);
      if (argument.filter(v -> v.size() > NativeIMapProfile075.MAX_VALUE_BYTES).isPresent()) throw new IllegalArgumentException("native argument bound");
      var seen = new HashSet<BytesValue>();
      for (var key : keys) { keyBound(key); if (!seen.add(key)) throw new IllegalArgumentException("duplicate native key"); bytes += key.size() + 16L; }
      for (var entry : entries) {
        keyBound(entry.key()); if (entry.value().size() > NativeIMapProfile075.MAX_VALUE_BYTES) throw new IllegalArgumentException("native value bound");
        if (!seen.add(entry.key())) throw new IllegalArgumentException("duplicate native key"); bytes += entry.key().size() + entry.value().size() + 32L;
      }
      if (bytes > NativeIMapProfile075.MAX_REQUEST_BYTES) throw new IllegalArgumentException("native request byte bound");
      boolean shape = switch (operation) {
        case SIZE, IS_EMPTY, SCAN, CLEAR, EVICT_ALL -> keys.isEmpty() && entries.isEmpty() && argument.isEmpty();
        case GET_OR_DEFAULT, CONTAINS_ENTRY -> keys.size() == 1 && entries.isEmpty() && argument.isPresent();
        case PUT_DETAILED, PUT_ALL_DIAGNOSTIC -> keys.isEmpty() && !entries.isEmpty() && argument.isEmpty();
        case REMOVE_DETAILED, ENTRY_VIEWS -> !keys.isEmpty() && entries.isEmpty() && argument.isEmpty();
        case EVICT -> keys.size() == 1 && entries.isEmpty() && argument.isEmpty();
      };
      if (!shape) throw new IllegalArgumentException("native operation shape");
    }
    public static Request empty(NativeIMapProfile075.Operation op) { return new Request(op, List.of(), List.of(), Optional.empty(), 64); }
  }
  public sealed interface RawReturn permits NumberReturn, BooleanReturn, ValueReturn, PointReturns, ViewsReturn, VoidReturn {}
  public record NumberReturn(long value) implements RawReturn {}
  public record BooleanReturn(boolean value) implements RawReturn {}
  public record ValueReturn(Optional<BytesValue> value) implements RawReturn { public ValueReturn { Objects.requireNonNull(value); } }
  public record PointReturn(int index, BytesValue key, Optional<BytesValue> previous) {
    public PointReturn { Objects.requireNonNull(key); Objects.requireNonNull(previous); }
  }
  public record PointReturns(List<PointReturn> items) implements RawReturn { public PointReturns { items = List.copyOf(items); } }
  public record NativeView(BytesValue key, BytesValue value, long nativeVersion, long nativeTtlMillis) {}
  public record ViewsReturn(List<NativeView> entries) implements RawReturn { public ViewsReturn { entries = List.copyOf(entries); } }
  public record VoidReturn() implements RawReturn {}
  public record Result(NativeIMapProfile075.Operation operation, Origin origin, OptionalInt nativeCalls, RawReturn rawReturn) {
    public Result { Objects.requireNonNull(operation); Objects.requireNonNull(origin); Objects.requireNonNull(rawReturn); Objects.requireNonNull(nativeCalls); if (nativeCalls.isPresent() && (nativeCalls.getAsInt() < 1 || nativeCalls.getAsInt() > 64)) throw new IllegalArgumentException("native call count"); }
  }
  public enum EventKind { ADDED, UPDATED, REMOVED, EXPIRED, EVICTED, MAP_CLEARED, MAP_EVICTED }
  public record NativeEvent(long registrationGeneration, EventKind kind, Optional<BytesValue> key,
      Optional<BytesValue> oldValue, Optional<BytesValue> value, int affectedEntries) {
    public NativeEvent {
      Objects.requireNonNull(kind); Objects.requireNonNull(key); Objects.requireNonNull(oldValue); Objects.requireNonNull(value);
      boolean aggregate = kind == EventKind.MAP_CLEARED || kind == EventKind.MAP_EVICTED;
      if (registrationGeneration < 1 || affectedEntries < 0 || aggregate == key.isPresent()
          || aggregate && (oldValue.isPresent() || value.isPresent())) throw new IllegalArgumentException("native event shape");
    }
  }
  public record Snapshot(List<Entry> entries, String digest) {
    public Snapshot {
      entries = List.copyOf(entries);
      if (entries.size() > 64 || !digest.equals(digestEntries(entries))) throw new IllegalArgumentException("native snapshot binding");
    }
  }
  static void keyBound(BytesValue key) { if (Objects.requireNonNull(key).size() > NativeIMapProfile075.MAX_KEY_BYTES) throw new IllegalArgumentException("native key bound"); }
  static Optional<BytesValue> optional(byte[] bytes) { return Optional.ofNullable(bytes).map(BytesValue::copyOf); }
  public static String digestEntries(List<Entry> entries) {
    var ordered = new TreeMap<BytesValue, BytesValue>();
    for (var entry : entries) {
      keyBound(entry.key());
      if (entry.value().size() > NativeIMapProfile075.MAX_VALUE_BYTES || ordered.put(entry.key(), entry.value()) != null) throw new IllegalArgumentException("native snapshot entry");
    }
    try {
      var digest = MessageDigest.getInstance("SHA-256");
      ordered.forEach((key, value) -> { for (var bytes : List.of(key.copy(), value.copy())) { digest.update(ByteBuffer.allocate(4).putInt(bytes.length).array()); digest.update(bytes); } digest.update((byte) 0); });
      return HexFormat.of().formatHex(digest.digest());
    } catch (NoSuchAlgorithmException unavailable) { throw new IllegalStateException(unavailable); }
  }
}
