package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Optional;
import org.junit.jupiter.api.Test;

final class BoundedIMapApiTest {
  private static final BoundedIMapApi.Bounds BOUNDS =
      new BoundedIMapApi.Bounds(8, 8, 8, 2, 2, 16, 8);

  @Test
  void boundedViewsCountsContainsAndLifecycleAreDeterministic() {
    var map = new BoundedIMapApi(BOUNDS);
    map.put(bytes("61"), bytes("31"), null, 0);
    map.put(bytes("62"), bytes("32"), 5L, 0);
    map.put(bytes("63"), bytes("33"), null, 0);

    assertEquals(3, map.size(0));
    assertFalse(map.isEmpty(0));
    var first = map.scan(BoundedIMapApi.Projection.KEYS, Optional.empty(), 2, 0);
    assertFalse(first.complete());
    assertEquals(2, first.items().size());
    var second = map.scan(
        BoundedIMapApi.Projection.ENTRIES, first.nextCursor(), 2, 0);
    assertTrue(second.complete());
    assertEquals(1, second.items().size());
    var values = map.scan(BoundedIMapApi.Projection.VALUES, Optional.empty(), 2, 0);
    assertTrue(values.items().stream().allMatch(item -> item.key().isEmpty()));

    var incomplete = map.containsValue(bytes("ff"), Optional.empty(), 2, 0);
    assertEquals(BoundedIMapApi.Match.INCOMPLETE, incomplete.match());
    assertEquals(BoundedIMapApi.Match.PRESENT,
        map.containsValue(bytes("33"), incomplete.nextCursor(), 2, 0).match());
    assertEquals(BoundedIMapApi.Match.ABSENT,
        map.containsValue(bytes("ff"), incomplete.nextCursor(), 2, 0).match());

    assertTrue(map.evict(bytes("61"), 0));
    assertFalse(map.evict(bytes("61"), 0));
    assertEquals(BoundedIMapApi.RemovalCause.EVICT, map.evictAll(0).cause());
    assertEquals(0, map.clear(0).removed());
    map.put(bytes("7a"), bytes("39"), null, 0);
    assertEquals(1, map.destroy(0).removed());
    assertTrue(map.isDestroyed());
    assertThrows(IllegalStateException.class, () -> map.size(0));
  }

  @Test
  void expiryAndMutationRejectStaleCursor() {
    var map = new BoundedIMapApi(BOUNDS);
    map.put(bytes("61"), bytes("31"), null, 0);
    map.put(bytes("62"), bytes("32"), 2L, 0);
    map.put(bytes("63"), bytes("33"), null, 0);
    var cursor = map.scan(BoundedIMapApi.Projection.KEYS, Optional.empty(), 2, 0)
        .nextCursor();
    long revision = map.revision();
    assertEquals(2, map.size(2));
    assertTrue(map.revision() > revision);
    assertThrows(IllegalStateException.class,
        () -> map.scan(BoundedIMapApi.Projection.KEYS, cursor, 2, 2));
  }

  @Test
  void limitsAndValueObjectsFailClosed() {
    assertThrows(IllegalArgumentException.class,
        () -> new BoundedIMapApi.Bounds(0, 1, 1, 1, 1, 1, 1));
    assertThrows(IllegalArgumentException.class, () -> new BoundedIMapApi.Cursor(0, 0));
    assertThrows(IllegalArgumentException.class,
        () -> new BoundedIMapApi.ScanItem(Optional.empty(), Optional.empty()));
    assertThrows(IllegalArgumentException.class,
        () -> new BoundedIMapApi.MatchResult(BoundedIMapApi.Match.INCOMPLETE, Optional.empty()));
    assertThrows(IllegalArgumentException.class,
        () -> new BoundedIMapApi.ScanPage(java.util.List.of(), Optional.empty(), false, 1));
    assertThrows(IllegalArgumentException.class,
        () -> new BoundedIMapApi.RemovalReceipt(BoundedIMapApi.RemovalCause.CLEAR, -1, 1));

    var map = new BoundedIMapApi(new BoundedIMapApi.Bounds(1, 3, 2, 1, 1, 2, 1));
    assertThrows(IllegalArgumentException.class,
        () -> map.put(bytes("6161"), bytes("31"), null, 0));
    assertThrows(IllegalArgumentException.class,
        () -> map.put(bytes("61"), bytes("31"), 0L, 0));
    map.put(bytes("61"), bytes("3131"), null, 0);
    map.put(bytes("62"), bytes("3232"), null, 0);
    assertThrows(IllegalArgumentException.class, () -> map.clear(0));
    assertEquals(2, map.size(0));
    assertThrows(IllegalArgumentException.class,
        () -> map.scan(BoundedIMapApi.Projection.ENTRIES, Optional.empty(), 1, 0));
    assertThrows(IllegalArgumentException.class,
        () -> map.containsValue(bytes("31"), Optional.empty(), 0, 0));
  }

  private static BytesValue bytes(String hex) { return BytesValue.fromHex(hex); }
}
