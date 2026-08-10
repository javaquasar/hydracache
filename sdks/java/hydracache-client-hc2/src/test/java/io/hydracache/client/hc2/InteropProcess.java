package io.hydracache.client.hc2;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.time.Duration;
import java.util.concurrent.TimeUnit;

final class InteropProcess implements AutoCloseable {
  private final Process process;
  private final BufferedReader output;
  final int port;
  final Path ca;
  final Path clientCertificate;
  final Path clientKey;

  private InteropProcess(
      Process process, BufferedReader output, int port, Path ca, Path clientCertificate, Path clientKey) {
    this.process = process;
    this.output = output;
    this.port = port;
    this.ca = ca;
    this.clientCertificate = clientCertificate;
    this.clientKey = clientKey;
  }

  static InteropProcess start(Path credentialsDirectory) throws IOException {
    return start(credentialsDirectory, "normal");
  }

  static InteropProcess start(Path credentialsDirectory, String profile) throws IOException {
    String binary = System.getenv("HC2_JAVA_INTEROP_SERVER");
    assertNotNull(binary, "HC2_JAVA_INTEROP_SERVER must identify the separately built Rust server");
    Process process = new ProcessBuilder(binary, "--credentials-dir", credentialsDirectory.toString(),
        "--profile", profile)
        .redirectErrorStream(true)
        .start();
    try {
      BufferedReader output = new BufferedReader(
          new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8));
      String ready = output.readLine();
      assertNotNull(ready, "interop server exited before readiness");
      String[] fields = ready.split("\\t", -1);
      assertEquals("READY", fields[0], "unexpected interop readiness: " + ready);
      assertEquals(5, fields.length, "malformed interop readiness: " + ready);
      return new InteropProcess(process, output, Integer.parseInt(fields[1]),
          Path.of(fields[2]), Path.of(fields[3]), Path.of(fields[4]));
    } catch (IOException | RuntimeException | Error failure) {
      process.destroyForcibly();
      throw failure;
    }
  }

  HydraCacheClientConfig config(String serverName) {
    return HydraCacheClientConfig.builder()
        .endpoint(java.net.URI.create("https://localhost:" + port))
        .serverName(serverName)
        .trustCertificate(ca)
        .clientCertificate(clientCertificate)
        .clientPrivateKey(clientKey)
        .clientId("external-java-consumer")
        .tenant("interop")
        .connectTimeout(Duration.ofSeconds(5))
        .defaultRequestTimeout(Duration.ofSeconds(2))
        .build();
  }

  String awaitClosed() throws Exception {
    String line;
    while ((line = output.readLine()) != null) {
      if (line.startsWith("CLOSED\t")) {
        assertTrue(process.waitFor(10, TimeUnit.SECONDS), "interop server did not terminate");
        assertEquals(0, process.exitValue(), "interop server failed: " + line);
        return line;
      }
    }
    throw new AssertionError("interop server exited without a CLOSED receipt");
  }

  @Override
  public void close() {
    if (process.isAlive()) {
      process.destroyForcibly();
      try {
        process.waitFor(5, TimeUnit.SECONDS);
      } catch (InterruptedException interrupted) {
        Thread.currentThread().interrupt();
      }
    }
  }
}
