package io.hydracache.client.hc2;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.time.Duration;
import java.util.concurrent.TimeUnit;

final class DaemonProcess implements AutoCloseable {
  private final Process process;
  private final BufferedReader output;
  private final BufferedWriter input;
  final int port;
  final Path ca;
  final Path clientCertificate;
  final Path clientKey;
  private boolean drained;

  private DaemonProcess(
      Process process, BufferedReader output, BufferedWriter input, int port,
      Path ca, Path clientCertificate, Path clientKey) {
    this.process = process;
    this.output = output;
    this.input = input;
    this.port = port;
    this.ca = ca;
    this.clientCertificate = clientCertificate;
    this.clientKey = clientKey;
  }

  static DaemonProcess start(Path credentialsDirectory) throws IOException {
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
      assertEquals("READY_DAEMON", fields[0], "unexpected daemon readiness: " + ready);
      assertEquals(6, fields.length, "malformed daemon readiness: " + ready);
      return new DaemonProcess(process, output, input, Integer.parseInt(fields[1]),
          Path.of(fields[3]), Path.of(fields[4]), Path.of(fields[5]));
    } catch (IOException | RuntimeException | Error failure) {
      process.destroyForcibly();
      throw failure;
    }
  }

  HydraCacheClientConfig config() {
    return HydraCacheClientConfig.builder()
        .endpoint(java.net.URI.create("https://localhost:" + port))
        .serverName("localhost")
        .trustCertificate(ca)
        .clientCertificate(clientCertificate)
        .clientPrivateKey(clientKey)
        .clientId("external-java-daemon-consumer")
        .tenant("daemon-interop")
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
    assertTrue(process.waitFor(10, TimeUnit.SECONDS), "production daemon fixture did not terminate");
    assertEquals(0, process.exitValue(), "production daemon fixture failed: " + line);
    return line;
  }

  @Override
  public void close() {
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
