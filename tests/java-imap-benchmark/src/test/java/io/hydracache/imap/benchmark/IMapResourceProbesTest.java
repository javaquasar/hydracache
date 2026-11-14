package io.hydracache.imap.benchmark;

import static io.hydracache.imap.benchmark.IMapResourceMonitor.Metric.*;
import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;
import java.lang.management.MemoryUsage;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.time.Instant;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.Timeout;
import org.junit.jupiter.api.io.TempDir;

@Timeout(20)
final class IMapResourceProbesTest {
  @TempDir Path temporary;

  @Test void observesRealJvmAndKeepsUnsupportedGcCountersAbsent() throws Exception {
    var reading = IMapResourceProbes.currentJvm("generator").source().read();
    assertTrue(reading.values().get(JVM_HEAP_COMMITTED_BYTES) > 0);
    assertTrue(reading.values().get(JVM_THREADS) > 0);
    assertTrue(reading.identity().startsWith("jvm:" + ProcessHandle.current().pid() + "@"));

    var memory = new MemoryUsage(-1, 20, 40, 100);
    var defined = IMapResourceProbes.jvmSnapshot("jvm", memory, 4, new long[]{2, 3}, new long[]{4, 6});
    assertEquals(Map.of(JVM_HEAP_USED_BYTES, 20L, JVM_HEAP_COMMITTED_BYTES, 40L,
        JVM_THREADS, 4L, JVM_GC_COLLECTIONS, 5L, JVM_GC_MILLIS, 10L), defined.values());
    var partial = IMapResourceProbes.jvmSnapshot("jvm", memory, 4, new long[]{2, -1}, new long[0]);
    assertFalse(partial.values().containsKey(JVM_GC_COLLECTIONS));
    assertFalse(partial.values().containsKey(JVM_GC_MILLIS));
  }

  @Test void readsLinuxRssThreadsCpuAndChecksProcessIncarnation() throws Exception {
    var status = Files.createDirectories(temporary.resolve("17")).resolve("status");
    Files.writeString(status, "Name:\tjava\nVmRSS:\t1234 kB\nThreads:\t42\n");
    var info = new Info(Optional.of(Instant.EPOCH), Optional.of(Duration.ofMillis(5)));
    var probe = IMapResourceProbes.linuxProcess("daemon", 17, () -> info, temporary);
    assertEquals(Map.of(CPU_NANOS, 5_000_000L, RSS_BYTES, 1234L * 1024, PROCESS_THREADS, 42L),
        probe.source().read().values());
    Files.writeString(status, "Name:\tjava\nThreads:\t42\n");
    assertEquals(Map.of(CPU_NANOS, 5_000_000L, PROCESS_THREADS, 42L), probe.source().read().values());
    var changes = new AtomicInteger();
    var replaced = IMapResourceProbes.linuxProcess("daemon", 17,
        () -> changes.getAndIncrement() == 0 ? info : new Info(Optional.of(Instant.ofEpochSecond(1)), Optional.empty()),
        temporary);
    assertThrows(IOException.class, replaced.source()::read);
    var absent = IMapResourceProbes.linuxProcess("daemon", 17,
        () -> new Info(Optional.empty(), Optional.empty()), temporary);
    assertThrows(IOException.class, absent.source()::read);
    Files.writeString(status, "Threads:\t42\nThreads:\t43\n");
    assertThrows(IOException.class, probe.source()::read);
    Files.writeString(status, "x".repeat(16 * 1024 + 1));
    assertThrows(IOException.class, probe.source()::read);
  }

  @Test void readsBoundedLinuxInterfaceCounters() throws Exception {
    var device = temporary.resolve("eth0");
    var stats = Files.createDirectories(device.resolve("statistics"));
    Files.writeString(device.resolve("ifindex"), "4\n");
    Files.writeString(stats.resolve("rx_bytes"), "12345\n");
    Files.writeString(stats.resolve("tx_bytes"), "67890\n");
    Files.writeString(stats.resolve("rx_packets"), "34\n");
    Files.writeString(stats.resolve("tx_packets"), "56\n");
    var probe = IMapResourceProbes.linuxInterface("link", "eth0", temporary);
    var sample = probe.source().read();
    assertEquals("linux-interface:eth0@4", sample.identity());
    assertEquals(Map.of(RX_BYTES, 12345L, TX_BYTES, 67890L, RX_PACKETS, 34L, TX_PACKETS, 56L), sample.values());
    assertThrows(IllegalArgumentException.class, () -> IMapResourceProbes.linuxInterface("link", "../eth0", temporary));
    Files.writeString(stats.resolve("rx_bytes"), "9".repeat(65));
    assertThrows(IOException.class, probe.source()::read);
  }

  @Test void linuxPssCgroupMemoryAndContextSwitchesAreIndependent() throws Exception {
    var dir = Files.createDirectories(temporary.resolve("17"));
    var info = new Info(Optional.of(Instant.EPOCH), Optional.empty());
    Files.writeString(dir.resolve("status"), "voluntary_ctxt_switches:\t12\nnonvoluntary_ctxt_switches:\t3\n");
    var process = IMapResourceProbes.linuxProcess("process", 17, () -> info, temporary).source().read();
    assertEquals(Map.of(VOLUNTARY_CONTEXT_SWITCHES, 12L, INVOLUNTARY_CONTEXT_SWITCHES, 3L), process.values());
    var rollup = dir.resolve("smaps_rollup");
    var pss = IMapResourceProbes.linuxPss("pss", 17, () -> info, temporary);
    Files.writeString(rollup, "Pss:\t42 kB\n");
    assertEquals(Map.of(PSS_BYTES, 43_008L), pss.source().read().values());
    var changes = new AtomicInteger();
    var replaced = IMapResourceProbes.linuxPss("pss", 17, () -> changes.getAndIncrement() == 0 ? info :
        new Info(Optional.empty(), Optional.empty()), temporary);
    assertThrows(IOException.class, replaced.source()::read);
    assertThrows(IOException.class, IMapResourceProbes.linuxPss("pss", 17,
        () -> new Info(Optional.empty(), Optional.empty()), temporary).source()::read);
    Files.writeString(rollup, "Rss:\t100 kB\n");
    assertThrows(IOException.class, pss.source()::read);
    assertEquals(IMapResourceMonitor.Scope.PROCESS,
        IMapResourceProbes.linuxPss("local", ProcessHandle.current()).scope());
    Files.writeString(dir.resolve("memory.current"), "123456\n");
    var cgroup = IMapResourceProbes.linuxCgroupMemory("group", dir);
    assertEquals(IMapResourceMonitor.Scope.CGROUP, cgroup.scope());
    if (Files.readAttributes(dir, java.nio.file.attribute.BasicFileAttributes.class).fileKey() != null) {
      assertEquals(Map.of(CGROUP_MEMORY_BYTES, 123456L), cgroup.source().read().values());
    } else {
      assertThrows(IOException.class, cgroup.source()::read);
    }
    assertEquals(Map.of(CGROUP_MEMORY_BYTES, 123456L),
        IMapResourceProbes.linuxCgroupMemory("group", dir, ignored -> "id").source().read().values());
    assertThrows(IOException.class, IMapResourceProbes.linuxCgroupMemory("group", dir, ignored -> null).source()::read);
    var incarnation = new AtomicInteger();
    assertThrows(IOException.class, IMapResourceProbes.linuxCgroupMemory("group", dir,
        ignored -> incarnation.incrementAndGet()).source()::read);
  }

  @Test void windowsMappingBindsProcessAndInterfaceAndRefusesMalformedOutput() throws Exception {
    var process = IMapResourceProbes.windowsProcess("daemon", 99, (script, argument) -> {
      assertEquals(WindowsIMapResourceQuery.PROCESS, script);
      assertEquals("99", argument);
      return "123 456 789 10\r\n";
    }).source().read();
    assertEquals("windows-pid:99@123", process.identity());
    assertEquals(Map.of(CPU_NANOS, 45600L, RSS_BYTES, 789L, PROCESS_THREADS, 10L), process.values());
    var network = IMapResourceProbes.windowsInterface("nic", "{A0-B1}", (script, argument) -> {
      assertEquals(WindowsIMapResourceQuery.INTERFACE, script);
      assertEquals("{A0-B1}", argument);
      return "1 2 3 4";
    }).source().read();
    assertEquals("windows-interface:{A0-B1}", network.identity());
    assertEquals(Map.of(RX_BYTES, 1L, TX_BYTES, 2L, RX_PACKETS, 3L, TX_PACKETS, 4L), network.values());
    assertThrows(IllegalArgumentException.class,
        () -> IMapResourceProbes.windowsProcess("daemon", 0, (s, a) -> ""));
    assertThrows(IllegalArgumentException.class,
        () -> IMapResourceProbes.windowsInterface("link", "'bad", (s, a) -> ""));
    assertThrows(IOException.class,
        () -> IMapResourceProbes.windowsInterface("link", "id", (s, a) -> "bad reply").source().read());
    assertThrows(NumberFormatException.class,
        () -> IMapResourceProbes.windowsProcess("daemon", 1, (s, a) -> "id 2 3 4").source().read());
  }

  @Test void localPlatformProbesReadRealProcessAndInterface() throws Exception {
    var process = ProcessHandle.current();
    var linuxProcess = IMapResourceProbes.linuxProcess("java", process);
    var linuxNetwork = IMapResourceProbes.linuxInterface("loopback", "lo");
    var windowsProcess = IMapResourceProbes.windowsProcess("java", process.pid());
    boolean windows = System.getProperty("os.name").startsWith("Windows");
    IMapResourceMonitor.Probe localProcess;
    IMapResourceMonitor.Probe localNetwork;
    if (windows) {
      var id = WindowsIMapResourceQuery.SYSTEM.read("""
          $a = [Net.NetworkInformation.NetworkInterface]::GetAllNetworkInterfaces() |
            Where-Object { $_.NetworkInterfaceType -eq 'Loopback' } | Select-Object -First 1
          [Console]::WriteLine($a.Id)
          """, "").strip();
      localProcess = windowsProcess;
      localNetwork = IMapResourceProbes.windowsInterface("loopback", id);
      assertThrows(IOException.class, linuxProcess.source()::read);
      assertThrows(IOException.class, linuxNetwork.source()::read);
    } else {
      localProcess = linuxProcess;
      localNetwork = linuxNetwork;
      assertThrows(IOException.class, windowsProcess.source()::read);
    }
    assertThrows(IOException.class,
        IMapResourceProbes.windowsInterface("absent", "{00000000-0000-0000-0000-000000000000}").source()::read);
    var reports = new ArrayList<IMapResourceMonitor.Report>();
    var targets = new IMapResourceMonitor.Targets(
        List.of(localProcess, IMapResourceProbes.currentJvm("jvm"), localNetwork), List.of(), List.of());
    var monitor = new IMapResourceMonitor.Recorder(targets, reports);
    monitor.around(IMapResourceMonitor.Side.LEFT, IMapResourceMonitor.Phase.MEASUREMENT, () -> {
      exchangeLoopback();
      return null;
    });
    var observed = reports.get(0).observations();
    assertProcess(observed.get(0).before().snapshot().orElseThrow());
    assertProcess(observed.get(0).after().snapshot().orElseThrow());
    assertTrue(observed.get(0).metrics().get(CPU_NANOS).change().orElseThrow() >= 0);
    assertTrue(observed.get(1).metrics().get(JVM_THREADS).after().orElseThrow() > 0);
    assertTraffic(observed.get(2).before().snapshot().orElseThrow());
    assertTraffic(observed.get(2).after().snapshot().orElseThrow());
    assertTrue(observed.get(2).metrics().get(RX_BYTES).change().orElseThrow() >= 0);
    if (!windows) {
      assertTrue(observed.get(2).metrics().get(RX_BYTES).change().orElseThrow() >= 16_384);
      assertTrue(observed.get(2).metrics().get(TX_PACKETS).change().orElseThrow() > 0);
    }
  }

  @Test void passesTheResourceSelectorSeparatelyFromTheBoundedCommand() throws Exception {
    var query = WindowsIMapResourceQuery.withRunner((builder, argument, timeout) -> {
      assertTrue(builder.command().get(0).endsWith("powershell.exe"));
      assertEquals(List.of("-NoLogo", "-NoProfile", "-NonInteractive", "-WindowStyle",
          "Hidden", "-Command", WindowsIMapResourceQuery.INTERFACE), builder.command().subList(1, 8));
      assertEquals("{ID-OF-NETWORK}", argument);
      assertEquals(3000, timeout);
      return "1 2 3 4";
    });
    assertEquals("1 2 3 4", query.read(WindowsIMapResourceQuery.INTERFACE, "{ID-OF-NETWORK}"));
  }

  private static void exchangeLoopback() {
    var address = InetAddress.getLoopbackAddress();
    try (var listener = new ServerSocket(0, 1, address); var client = new Socket()) {
      listener.setSoTimeout(2000);
      client.connect(new InetSocketAddress(address, listener.getLocalPort()), 2000);
      try (var server = listener.accept()) {
        server.setSoTimeout(2000);
        var data = new byte[16_384];
        client.getOutputStream().write(data);
        assertArrayEquals(data, server.getInputStream().readNBytes(data.length));
      }
    } catch (IOException failure) {
      throw new AssertionError("loopback transfer failed", failure);
    }
  }

  @Test void externalCounterCommandIsBoundedAndPreservesInterrupt() throws Exception {
    // These cases verify the child outcome, so allow for a cold JVM on a loaded runner.
    long fixtureTimeoutMillis = 15_000;
    assertEquals("42", WindowsIMapResourceQuery.run(command("echo"), "42", fixtureTimeoutMillis));
    assertTrue(assertThrows(IOException.class,
        () -> WindowsIMapResourceQuery.run(command("bad"), "", fixtureTimeoutMillis)).getMessage().contains("failed"));
    assertTrue(assertThrows(IOException.class,
        () -> WindowsIMapResourceQuery.run(command("large"), "", fixtureTimeoutMillis)).getMessage().contains("output bound"));
    assertTrue(assertThrows(IOException.class,
        () -> WindowsIMapResourceQuery.run(command("wait"), "", 30)).getMessage().contains("timed out"));
    Thread.currentThread().interrupt();
    try {
      assertTrue(assertThrows(IOException.class,
          () -> WindowsIMapResourceQuery.run(command("wait"), "", 3000)).getMessage().contains("interrupted"));
      assertTrue(Thread.currentThread().isInterrupted());
    } finally {
      Thread.interrupted();
    }
  }

  private static void assertProcess(IMapResourceMonitor.Snapshot reading) {
    assertTrue(reading.values().get(CPU_NANOS) >= 0);
    assertTrue(reading.values().get(RSS_BYTES) > 0);
    assertTrue(reading.values().get(PROCESS_THREADS) > 0);
  }

  private static void assertTraffic(IMapResourceMonitor.Snapshot reading) {
    for (var counter : List.of(RX_BYTES, TX_BYTES, RX_PACKETS, TX_PACKETS)) {
      assertTrue(reading.values().get(counter) >= 0);
    }
  }

  private static ProcessBuilder command(String mode) {
    var java = Path.of(System.getProperty("java.home"), "bin", "java");
    return new ProcessBuilder(java.toString(), "-cp", System.getProperty("java.class.path"),
        CounterCommand.class.getName(), mode);
  }

  public static final class CounterCommand {
    public static void main(String[] args) throws Exception {
      switch (args[0]) {
        case "echo" -> System.out.print(System.getenv(WindowsIMapResourceQuery.ARGUMENT));
        case "bad" -> System.exit(3);
        case "large" -> System.out.print("x".repeat(1025));
        case "wait" -> Thread.sleep(30_000);
        default -> throw new IllegalArgumentException();
      }
    }
  }

  private record Info(Optional<Instant> startInstant, Optional<Duration> totalCpuDuration)
      implements ProcessHandle.Info {
    @Override public Optional<String> command() { return Optional.empty(); }
    @Override public Optional<String> commandLine() { return Optional.empty(); }
    @Override public Optional<String[]> arguments() { return Optional.empty(); }
    @Override public Optional<String> user() { return Optional.empty(); }
  }
}
