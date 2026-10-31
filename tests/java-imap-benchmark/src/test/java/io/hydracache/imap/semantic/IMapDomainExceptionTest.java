package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.*;

import java.util.Map;
import org.junit.jupiter.api.Test;

final class IMapDomainExceptionTest {
  @Test
  void typedFailuresExposeStableSharedKindsWithoutCollapsingDistinctReasons() {
    var cases = Map.ofEntries(
        Map.entry("mutation conflict", "mutation_identity_conflict"),
        Map.entry("keyBytes exceeds 1", "key_too_large"),
        Map.entry("valueBytes exceeds 1", "value_too_large"),
        Map.entry("responseBytes exceeds 1", "response_too_large"),
        Map.entry("response byte bound", "response_too_large"),
        Map.entry("requestEntries exceeds 1", "request_too_large"),
        Map.entry("request byte bound", "request_too_large"),
        Map.entry("bulkMutationItems exceeds 1", "request_too_large"),
        Map.entry("dedupEntries exceeds 1", "idempotency_capacity_exhausted"),
        Map.entry("logical time regressed", "clock_regressed"),
        Map.entry("entries exceeds 1", "capacity"),
        Map.entry("stale scan cursor", "scan_cursor_stale"),
        Map.entry("foreign or invalid cursor", "cursor_mismatch"),
        Map.entry("map is destroyed", "map_destroyed"),
        Map.entry("invalid ttl", "invalid_argument"));
    for (var entry : cases.entrySet()) assertEquals(entry.getValue(),
        IMapDomainException.translate(new IllegalArgumentException(entry.getKey())).kind());
    assertEquals("capacity", IMapDomainException.translate(new ArithmeticException()).kind());
    assertEquals("invalid_argument", IMapDomainException.translate(new NullPointerException()).kind());
    var original = new IMapDomainException(IMapDomainException.Kind.UNSUPPORTED_OPERATION, "unsupported");
    assertSame(original, IMapDomainException.translate(original));
    assertEquals(IMapDomainException.Kind.UNSUPPORTED_OPERATION, original.error());
  }
}
