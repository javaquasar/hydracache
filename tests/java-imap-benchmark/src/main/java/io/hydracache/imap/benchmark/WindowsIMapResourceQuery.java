package io.hydracache.imap.benchmark;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.concurrent.TimeUnit;

/** Read-only, bounded Windows process and interface queries. */
final class WindowsIMapResourceQuery {
  private WindowsIMapResourceQuery() {}

  static final String ARGUMENT = "HC_IMAP_RESOURCE_ARG";
  static final String PROCESS = """
      $ErrorActionPreference = 'Stop'
      $p = [Diagnostics.Process]::GetProcessById([int]$env:HC_IMAP_RESOURCE_ARG)
      [Console]::WriteLine(($p.StartTime.ToUniversalTime().Ticks, $p.TotalProcessorTime.Ticks,
        $p.WorkingSet64, $p.Threads.Count -join ' '))
      """;
  static final String INTERFACE = """
      $ErrorActionPreference = 'Stop'
      $nic = @([Net.NetworkInformation.NetworkInterface]::GetAllNetworkInterfaces() |
        Where-Object { $_.Id -ieq $env:HC_IMAP_RESOURCE_ARG })
      if ($nic.Count -ne 1) { throw 'Interface unavailable' }
      $s = $nic[0].GetIPStatistics()
      [Console]::WriteLine(($s.BytesReceived, $s.BytesSent,
        ($s.UnicastPacketsReceived + $s.NonUnicastPacketsReceived),
        ($s.UnicastPacketsSent + $s.NonUnicastPacketsSent) -join ' '))
      """;

  @FunctionalInterface interface Query {
    String read(String script, String argument) throws IOException;
  }

  @FunctionalInterface interface CommandRunner {
    String run(ProcessBuilder builder, String argument, long timeoutMillis) throws IOException;
  }

  static final Query SYSTEM = withRunner(WindowsIMapResourceQuery::run);

  static Query withRunner(CommandRunner runner) {
    var windows = System.getenv().getOrDefault("SystemRoot", "C:\\Windows");
    var shell = Path.of(windows, "System32", "WindowsPowerShell", "v1.0", "powershell.exe");
    return (script, argument) -> runner.run(new ProcessBuilder(shell.toString(),
        "-NoLogo", "-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", script),
        argument, 3000);
  }

  static String run(ProcessBuilder builder, String argument, long timeoutMillis) throws IOException {
    builder.environment().put(ARGUMENT, argument);
    builder.redirectErrorStream(true);
    var process = builder.start();
    try (var output = process.getInputStream()) {
      if (!process.waitFor(timeoutMillis, TimeUnit.MILLISECONDS)) {
        throw new IOException("Windows resource query timed out");
      }
      if (process.exitValue() != 0) throw new IOException("Windows resource query failed");
      byte[] bytes = output.readNBytes(1025);
      if (bytes.length > 1024) throw new IOException("Windows resource query output bound");
      return new String(bytes, StandardCharsets.US_ASCII);
    } catch (InterruptedException interrupted) {
      Thread.currentThread().interrupt();
      throw new IOException("Windows resource query interrupted", interrupted);
    } finally {
      process.destroyForcibly();
    }
  }
}
