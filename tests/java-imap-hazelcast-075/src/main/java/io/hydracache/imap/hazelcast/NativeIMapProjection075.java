package io.hydracache.imap.hazelcast;

import io.hydracache.imap.semantic.BytesValue;
import java.util.*;
import static io.hydracache.imap.hazelcast.NativeIMapObservation075.*;

/** Explicit projection of actual point helper returns, never an ExtendedMapSemanticAdapter. */
public final class NativeIMapProjection075 {
  private NativeIMapProjection075() {}
  public record Item(int index, BytesValue key, String kind, Optional<BytesValue> previous) {}
  public static List<Item> detailed(NativeIMapProfile075 profile, Result result) {
    Objects.requireNonNull(profile);
    if (result.origin() != Origin.CLIENT_POINT_HELPER || !(result.rawReturn() instanceof PointReturns returns)
        || result.operation() != NativeIMapProfile075.Operation.PUT_DETAILED && result.operation() != NativeIMapProfile075.Operation.REMOVE_DETAILED
        || result.nativeCalls().isEmpty() || returns.items().size() != result.nativeCalls().getAsInt()) throw new IllegalArgumentException("detailed native evidence unavailable");
    var keys = new HashSet<BytesValue>(); var projected = new ArrayList<Item>();
    for (var item : returns.items()) {
      if (item.index() != projected.size() || !keys.add(item.key())) throw new IllegalArgumentException("native detailed shape");
      String kind = result.operation() == NativeIMapProfile075.Operation.PUT_DETAILED
          ? item.previous().isPresent() ? "replaced" : "inserted" : item.previous().isPresent() ? "removed" : "absent";
      projected.add(new Item(item.index(), item.key(), kind, item.previous()));
    }
    return List.copyOf(projected);
  }
  /** Compare each key's callback order; cross-key arrival ordering is intentionally immaterial. */
  public static boolean samePerKeyEvents(List<NativeEvent> left, List<NativeEvent> right) {
    return group(left).equals(group(right));
  }
  private static Map<Optional<BytesValue>, List<NativeEvent>> group(List<NativeEvent> events) {
    if (events.size() > 1024) throw new IllegalArgumentException("native event comparison bound");
    var grouped = new HashMap<Optional<BytesValue>, List<NativeEvent>>();
    for (var event : events) grouped.computeIfAbsent(event.key(), ignored -> new ArrayList<>()).add(event);
    return grouped;
  }
}
