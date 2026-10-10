package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

final class ProductAdapterScaffoldTest {
  @Test
  void productAdaptersAreFailClosedUntilThePublishedV074SurfaceIsFrozen() {
    for (MapSemanticAdapter adapter : new MapSemanticAdapter[] {
        new HydraCacheSemanticAdapter(), new HazelcastSemanticAdapter()
    }) {
      try (adapter) {
        assertTrue(adapter.capabilities().isEmpty());
        AdapterUnavailableException error = assertThrows(
            AdapterUnavailableException.class,
            () -> adapter.execute(ScenarioStep.get("probe", BytesValue.fromHex("00"))));
        assertTrue(error.getMessage().contains("blocked-by-0.74"));
      }
    }
  }

  @Test
  void stubsDeclareDistinctProductIdentities() {
    assertEquals("hydracache", new HydraCacheSemanticAdapter().identity().product());
    assertEquals("hazelcast", new HazelcastSemanticAdapter().identity().product());
  }
}
