package io.hydracache.client.hc2;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.net.URI;
import java.nio.file.Files;
import java.nio.file.Path;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

final class HydraCacheClientConfigTest {
  @Test
  void protocolGenerationDefaultsToCurrentAndAcceptsCompatibilityWindow(@TempDir Path temp)
      throws Exception {
    Path credential = Files.writeString(temp.resolve("credential.pem"), "unused");
    var base = HydraCacheClientConfig.builder()
        .endpoint(URI.create("https://localhost:7443"))
        .serverName("localhost")
        .trustCertificate(credential)
        .clientCertificate(credential)
        .clientPrivateKey(credential);
    assertEquals(HydraCacheClientConfig.CURRENT_PROTOCOL_GENERATION,
        base.build().protocolGeneration());
    assertEquals(HydraCacheClientConfig.MINIMUM_PROTOCOL_GENERATION,
        base.protocolGeneration(HydraCacheClientConfig.MINIMUM_PROTOCOL_GENERATION)
            .build().protocolGeneration());
    assertThrows(IllegalArgumentException.class,
        () -> base.protocolGeneration(HydraCacheClientConfig.MINIMUM_PROTOCOL_GENERATION - 1)
            .build());
  }

  @Test
  void plaintextEndpointHasNoFallback(@TempDir Path temp) throws Exception {
    Path credential = Files.writeString(temp.resolve("credential.pem"), "unused");
    var builder = HydraCacheClientConfig.builder()
        .endpoint(URI.create("http://localhost:7443"))
        .serverName("localhost")
        .trustCertificate(credential)
        .clientCertificate(credential)
        .clientPrivateKey(credential);
    assertThrows(IllegalArgumentException.class, builder::build);
  }
}
