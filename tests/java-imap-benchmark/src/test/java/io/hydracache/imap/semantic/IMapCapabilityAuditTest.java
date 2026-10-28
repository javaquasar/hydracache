package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.*;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Map;
import java.util.Properties;
import java.util.Set;
import org.junit.jupiter.api.Test;

final class IMapCapabilityAuditTest {
  @Test void actualReferenceAndExistingSdkCapabilitiesRemainSeparate() throws Exception {
    var bounds = new BoundedIMapApi.Bounds(128, 256, 64, 64, 64, 4096, 64);
    var bounded = IMapCapabilityDescriptor.local(bounds); var unified = IMapCapabilityDescriptor.unified(bounds);
    var sdk = IMapCapabilityDescriptor.sdkActual();
    assertEquals(21, bounded.operations().size()); assertEquals(40, unified.operations().size());
    assertEquals(7, unified.interfaceFeatures().size()); assertEquals("unified_reference", unified.scope());
    assertTrue(unified.operations().containsAll(bounded.operations())); assertTrue(bounded.interfaceFeatures().isEmpty());
    assertEquals(11, sdk.operations().size()); assertTrue(sdk.supports("IMap.put")); assertFalse(sdk.supports("get_all"));
    assertEquals("sdk_existing_narrow", sdk.scope()); assertFalse(sdk.productionEnabled()); assertFalse(unified.productionEnabled());
    Path root = Path.of("").toAbsolutePath();
    while (root != null && !Files.isDirectory(root.resolve("sdks/java"))) root = root.getParent();
    assertNotNull(root);
    var properties = new Properties();
    try (var input = Files.newInputStream(root.resolve("sdks/java/hydracache-hazelcast-facade/src/main/resources/META-INF/hydracache/hazelcast-capabilities.properties"))) { properties.load(input); }
    assertEquals(String.join(",", new java.util.TreeSet<>(sdk.operations())), java.util.Arrays.stream(properties.getProperty("supported").split(",(?=[A-Z])")).sorted().collect(java.util.stream.Collectors.joining(",")));
    assertThrows(IllegalArgumentException.class, () -> new IMapCapabilityDescriptor(Set.of("a"), Set.of("a"), Map.of(), Set.of(), Set.of(), "fabricated", Set.of()));
    assertThrows(UnsupportedOperationException.class, () -> unified.interfaceFeatures().add("production"));
  }
}
