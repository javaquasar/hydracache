package io.hydracache.hazelcast;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import io.hydracache.client.hc2.HydraCacheClientConfig;
import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.time.Duration;
import java.util.concurrent.TimeUnit;

/** Production-daemon fixture used by the facade's release-gated borrowed suite. */
final class FacadeDaemonProcess implements AutoCloseable {
  private final Process process;
  private final BufferedReader output;
  private final BufferedWriter input;
  private final int port;
  private final Path ca;
  private final Path clientCertificate;
  private final Path clientKey;
  private final Path secondClientCertificate;
  private final Path secondClientKey;
  private boolean drained;

  private FacadeDaemonProcess(
      Process process, BufferedReader output, BufferedWriter input, int port,
      Path ca, Path clientCertificate, Path clientKey,
      Path secondClientCertificate, Path secondClientKey) {
    this.process = process;
    this.output = output;
    this.input = input;
    this.port = port;
    this.ca = ca;
    this.clientCertificate = clientCertificate;
    this.clientKey = clientKey;
    this.secondClientCertificate = secondClientCertificate;
    this.secondClientKey = secondClientKey;
  }

  static FacadeDaemonProcess start(Path credentialsDirectory) throws IOException {
    String fixture = System.getenv("HC2_JAVA_INTEROP_SERVER");
    String daemon = System.getenv("HC2_JAVA_DAEMON");
    assertNotNull(fixture, "HC2_JAVA_INTEROP_SERVER must identify the Rust fixture");
    assertNotNull(daemon, "HC2_JAVA_DAEMON must identify the production daemon");
    Process process = new ProcessBuilder(fixture, "--credentials-dir",
        credentialsDirectory.toString(), "--daemon", daemon)
        .redirectErrorStream(true)
        .start();
    try {
      BufferedReader output = new BufferedReader(
          new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8));
      BufferedWriter input = new BufferedWriter(
          new OutputStreamWriter(process.getOutputStream(), StandardCharsets.UTF_8));
      String ready = output.readLine();
      assertNotNull(ready, "production daemon fixture exited before readiness");
      String[] fields = ready.split("\\t", -1);
      assertEquals("READY_DAEMON_V1", fields[0], "unexpected daemon readiness: " + ready);
      assertEquals(8, fields.length, "malformed daemon readiness: " + ready);
      return new FacadeDaemonProcess(process, output, input, Integer.parseInt(fields[1]),
          Path.of(fields[3]), Path.of(fields[4]), Path.of(fields[5]),
          Path.of(fields[6]), Path.of(fields[7]));
    } catch (IOException | RuntimeException | Error failure) {
      process.destroyForcibly();
      throw failure;
    }
  }

  HydraCacheClientConfig config(String clientId) {
    return config(clientId, false);
  }

  HydraCacheClientConfig config(String clientId, boolean secondIdentity) {
    return HydraCacheClientConfig.builder()
        .endpoint(java.net.URI.create("https://localhost:" + port))
        .serverName("localhost")
        .trustCertificate(ca)
        .clientCertificate(secondIdentity ? secondClientCertificate : clientCertificate)
        .clientPrivateKey(secondIdentity ? secondClientKey : clientKey)
        .clientId(clientId)
        .tenant("hazelcast-facade-interop")
        .connectTimeout(Duration.ofSeconds(5))
        .defaultRequestTimeout(Duration.ofSeconds(2))
        .build();
  }

  String drain() throws Exception {
    if (!drained) {
      input.write("DRAIN\n");
      input.flush();
      drained = true;
    }
    String line = output.readLine();
    assertNotNull(line, "production daemon fixture exited without a close receipt");
    assertTrue(line.startsWith("CLOSED_DAEMON\t"), line);
    assertTrue(process.waitFor(10, TimeUnit.SECONDS), "production daemon did not terminate");
    assertEquals(0, process.exitValue(), "production daemon fixture failed: " + line);
    return line;
  }

  @Override public void close() {
    if (!process.isAlive()) return;
    try {
      drain();
    } catch (Exception failure) {
      process.destroyForcibly();
      try {
        process.waitFor(5, TimeUnit.SECONDS);
      } catch (InterruptedException interrupted) {
        Thread.currentThread().interrupt();
      }
    }
  }
}
