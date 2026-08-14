package io.hydracache.consumer;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import io.hydracache.client.hc2.HydraCacheClient;
import io.hydracache.client.hc2.HydraCacheClientConfig;
import io.hydracache.client.hc2.RequestOptions;
import io.hydracache.client.hc2.internal.wire.ClientEnvelope;
import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.io.PrintWriter;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.concurrent.TimeUnit;
import java.util.jar.JarFile;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

final class ExternalConsumerTest {
  @Test
  void retainedGeneratedMessagePreservesUnknownFields() throws Exception {
    byte[] futureEnvelope = new byte[] {(byte) 0xfa, 0x03, 0x03, 'n', 'e', 'w'};
    ClientEnvelope decoded = ClientEnvelope.parseFrom(futureEnvelope);
    assertTrue(decoded.getUnknownFields().asMap().containsKey(63));
    assertArrayEquals(futureEnvelope, decoded.toByteArray());
  }

  @Test
  void installedSdkBuildsAndRunsWithoutRepositorySources(@TempDir Path temp) throws Exception {
    String binary = System.getenv("HC2_JAVA_INTEROP_SERVER");
    assertNotNull(binary, "HC2_JAVA_INTEROP_SERVER is required");
    Path credentials = Files.createDirectory(temp.resolve("credentials"));
    Process server = new ProcessBuilder(binary, "--credentials-dir", credentials.toString())
        .redirectErrorStream(true).start();
    try {
      BufferedReader output = new BufferedReader(
          new InputStreamReader(server.getInputStream(), StandardCharsets.UTF_8));
      String[] ready = output.readLine().split("\\t", -1);
      assertEquals("READY", ready[0]);
      assertEquals(5, ready.length);

      HydraCacheClientConfig config = HydraCacheClientConfig.builder()
          .endpoint(URI.create("https://localhost:" + ready[1]))
          .serverName("localhost")
          .trustCertificate(Path.of(ready[2]))
          .clientCertificate(Path.of(ready[3]))
          .clientPrivateKey(Path.of(ready[4]))
          .clientId("standalone-consumer")
          .tenant("consumer")
          .build();
      try (HydraCacheClient client = HydraCacheClient.connect(config)) {
        byte[] key = "consumer-key".getBytes(StandardCharsets.UTF_8);
        byte[] value = "consumer-value".getBytes(StandardCharsets.UTF_8);
        RequestOptions options = RequestOptions.withTimeout(Duration.ofSeconds(2));
        assertTrue(client.put(key, value, Duration.ZERO, options).join().applied());
        assertArrayEquals(value, client.get(key, options).join().orElseThrow().value());
      }

      String closed = output.readLine();
      assertTrue(closed.startsWith("CLOSED\t"), closed);
      assertTrue(closed.contains("active_subscriptions=0"), closed);
      assertTrue(closed.contains("active_sessions=0"), closed);
      assertTrue(server.waitFor(10, TimeUnit.SECONDS));
      assertEquals(0, server.exitValue());
    } finally {
      if (server.isAlive()) server.destroyForcibly();
    }
  }

  @Test
  void retainedSdkRunsAgainstCurrentProductionDaemon(@TempDir Path temp) throws Exception {
    Assumptions.assumeTrue(
        Boolean.getBoolean("hc2.compat.production.required"),
        "production compatibility execution is owned by client-plane-compat-check");
    String fixture = System.getenv("HC2_COMPAT_INTEROP_SERVER");
    String daemon = System.getenv("HC2_COMPAT_PRODUCTION_DAEMON");
    assertNotNull(fixture, "HC2_COMPAT_INTEROP_SERVER is required");
    assertNotNull(daemon, "HC2_COMPAT_PRODUCTION_DAEMON is required");
    Path credentials = Files.createDirectory(temp.resolve("production-credentials"));
    Process server = new ProcessBuilder(
        fixture, "--credentials-dir", credentials.toString(), "--daemon", daemon)
        .redirectErrorStream(true).start();
    try {
      BufferedReader output = new BufferedReader(
          new InputStreamReader(server.getInputStream(), StandardCharsets.UTF_8));
      String[] ready = output.readLine().split("\\t", -1);
      assertEquals("READY_DAEMON_V1", ready[0]);
      assertEquals(8, ready.length);

      HydraCacheClientConfig config = HydraCacheClientConfig.builder()
          .endpoint(URI.create("https://localhost:" + ready[1]))
          .serverName("localhost")
          .trustCertificate(Path.of(ready[3]))
          .clientCertificate(Path.of(ready[4]))
          .clientPrivateKey(Path.of(ready[5]))
          .clientId("retained-java-client")
          .tenant("compat-tenant")
          .build();
      try (HydraCacheClient client = HydraCacheClient.connect(config)) {
        byte[] key = "retained-java/key".getBytes(StandardCharsets.UTF_8);
        byte[] value = "retained-java-value".getBytes(StandardCharsets.UTF_8);
        RequestOptions options = RequestOptions.withTimeout(Duration.ofSeconds(2));
        assertTrue(client.put(key, value, Duration.ZERO, options).join().applied());
        assertArrayEquals(value, client.get(key, options).join().orElseThrow().value());
      }

      PrintWriter input = new PrintWriter(
          new OutputStreamWriter(server.getOutputStream(), StandardCharsets.UTF_8), true);
      input.println("DRAIN");
      assertEquals("CLOSED_DAEMON\tstatus=ok", output.readLine());
      assertTrue(server.waitFor(10, TimeUnit.SECONDS));
      assertEquals(0, server.exitValue());
    } finally {
      if (server.isAlive()) server.destroyForcibly();
    }
  }

  @Test
  void installedJarCarriesPreviewProtocolMetadata() throws Exception {
    Path location = Path.of(HydraCacheClient.class.getProtectionDomain().getCodeSource().getLocation().toURI());
    assertTrue(location.toString().endsWith(".jar"), "consumer must resolve an installed jar: " + location);
    try (JarFile jar = new JarFile(location.toFile())) {
      var attributes = jar.getManifest().getMainAttributes();
      assertEquals("io.hydracache.client.hc2", attributes.getValue("Automatic-Module-Name"));
      assertEquals(System.getProperty("hc2.expected.protocol.generation"),
          attributes.getValue("HydraCache-Protocol-Generation"));
      assertEquals("preview", attributes.getValue("HydraCache-Api-Stability"));
    }
  }
}
