package io.hydracache.consumer;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;

import io.hydracache.hazelcast.HydraCacheInstance;
import io.hydracache.hazelcast.HydraCodec;
import java.io.InputStream;
import java.net.URL;
import java.util.Properties;
import java.util.jar.JarFile;
import org.junit.jupiter.api.Test;

final class ExternalFacadeConsumerTest {
  @Test void facadeResolvesFromAnInstalledJarWithExplicitCompatibilityMetadata() throws Exception {
    URL source = HydraCacheInstance.class.getProtectionDomain().getCodeSource().getLocation();
    assertEquals("file", source.getProtocol());
    assertFalse(source.getPath().contains("target/classes"), source.toString());
    try (JarFile jar = new JarFile(new java.io.File(source.toURI()))) {
      assertEquals("false", jar.getManifest().getMainAttributes()
          .getValue("Hazelcast-Wire-Compatible"));
    }
    try (InputStream stream = HydraCacheInstance.class.getClassLoader().getResourceAsStream(
        "META-INF/hydracache/hazelcast-capabilities.properties")) {
      assertNotNull(stream);
      Properties properties = new Properties();
      properties.load(stream);
      assertEquals("false", properties.getProperty("wireCompatibility"));
    }
    assertEquals("value", HydraCodec.utf8().decode(HydraCodec.utf8().encode("value")));
  }
}
