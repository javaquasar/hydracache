package io.hydracache.imap.hazelcast;
import static org.junit.jupiter.api.Assertions.*;
import org.junit.jupiter.api.Test;
final class NativeExtendedProbeMain075Test {
  @Test void actualProbeUsesPackagedPublicNativeContract() throws Exception {
    assertThrows(IllegalArgumentException.class, () -> NativeExtendedProbeMain075.main(new String[]{"foreign"}));
    try (var fixture = new HazelcastLoopback075()) {
      var json = NativeExtendedProbeMain075.observe(fixture);
      assertTrue(json.contains("\"second_put_kinds\":[\"replaced\",\"inserted\"]"));
      assertTrue(json.contains("\"removed_previous\":[\"61\",null]"));
      assertTrue(json.contains("\"clear_event\":\"map_cleared\",\"clear_affected\":1"));
      assertTrue(json.contains("\"point_calls\":3"));
    }
  }
}
