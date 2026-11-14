package io.hydracache.imap.benchmark;

import static io.hydracache.imap.benchmark.IMapResourceMonitor.Metric.*;

import io.hydracache.imap.benchmark.IMapResourceMonitor.Metric;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Probe;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Scope;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Snapshot;
import java.io.IOException;
import java.lang.management.GarbageCollectorMXBean;
import java.lang.management.ManagementFactory;
import java.lang.management.MemoryUsage;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.attribute.BasicFileAttributes;
import java.time.Duration;
import java.util.EnumMap;
import java.util.Map;
import java.util.Optional;
import java.util.function.Supplier;
import java.util.regex.Pattern;

/** Local JVM, process, and interface probes. Other owners can supply a ResourceMonitor.Source. */
public final class IMapResourceProbes {
  private static final Pattern RSS = Pattern.compile("(?m)^VmRSS:[ \\t]+([0-9]+)[ \\t]+kB[ \\t]*$");
  private static final Pattern THREADS = Pattern.compile("(?m)^Threads:[ \\t]+([0-9]+)[ \\t]*$");
  private static final Pattern PSS = Pattern.compile("(?m)^Pss:[ \\t]+([0-9]+)[ \\t]+kB[ \\t]*$");
  private static final Pattern VOLUNTARY = Pattern.compile("(?m)^voluntary_ctxt_switches:[ \\t]+([0-9]+)[ \\t]*$");
  private static final Pattern INVOLUNTARY = Pattern.compile("(?m)^nonvoluntary_ctxt_switches:[ \\t]+([0-9]+)[ \\t]*$");
  private static final Map<Metric, String> NETWORK = Map.of(
      RX_BYTES, "rx_bytes", TX_BYTES, "tx_bytes", RX_PACKETS, "rx_packets", TX_PACKETS, "tx_packets");

  private IMapResourceProbes() {}

  public static Probe currentJvm(String label) {
    var runtime = ManagementFactory.getRuntimeMXBean();
    String identity = "jvm:" + ProcessHandle.current().pid() + "@" + runtime.getStartTime();
    return new Probe(label, Scope.JVM, () -> {
      var gc = ManagementFactory.getGarbageCollectorMXBeans();
      return jvmSnapshot(identity, ManagementFactory.getMemoryMXBean().getHeapMemoryUsage(),
          ManagementFactory.getThreadMXBean().getThreadCount(),
          gc.stream().mapToLong(GarbageCollectorMXBean::getCollectionCount).toArray(),
          gc.stream().mapToLong(GarbageCollectorMXBean::getCollectionTime).toArray());
    });
  }

  static Snapshot jvmSnapshot(String identity, MemoryUsage heap, int threads, long[] counts, long[] millis) {
    var values = new EnumMap<Metric, Long>(Metric.class);
    values.put(JVM_HEAP_USED_BYTES, heap.getUsed());
    values.put(JVM_HEAP_COMMITTED_BYTES, heap.getCommitted());
    values.put(JVM_THREADS, (long) threads);
    sumGc(counts).ifPresent(value -> values.put(JVM_GC_COLLECTIONS, value));
    sumGc(millis).ifPresent(value -> values.put(JVM_GC_MILLIS, value));
    return new Snapshot(identity, values);
  }

  private static Optional<Long> sumGc(long[] values) {
    if (values.length == 0) return Optional.empty();
    long sum = 0;
    for (long value : values) {
      if (value < 0) return Optional.empty();
      sum = Math.addExact(sum, value);
    }
    return Optional.of(sum);
  }

  public static Probe linuxProcess(String label, ProcessHandle process) {
    return linuxProcess(label, process.pid(), process::info, Path.of("/proc"));
  }

  static Probe linuxProcess(String label, long pid, Supplier<ProcessHandle.Info> info, Path proc) {
    return new Probe(label, Scope.PROCESS, () -> {
      var before = info.get();
      var born = before.startInstant().orElseThrow(() -> new IOException("process birth time unavailable"));
      var status = boundedAscii(proc.resolve(Long.toString(pid)).resolve("status"), 16 * 1024);
      var values = new EnumMap<Metric, Long>(Metric.class);
      before.totalCpuDuration().map(Duration::toNanos).ifPresent(value -> values.put(CPU_NANOS, value));
      readStatus(status, RSS).ifPresent(value -> values.put(RSS_BYTES, Math.multiplyExact(value, 1024)));
      readStatus(status, THREADS).ifPresent(value -> values.put(PROCESS_THREADS, value));
      readStatus(status, VOLUNTARY).ifPresent(value -> values.put(VOLUNTARY_CONTEXT_SWITCHES, value));
      readStatus(status, INVOLUNTARY).ifPresent(value -> values.put(INVOLUNTARY_CONTEXT_SWITCHES, value));
      if (!info.get().startInstant().equals(Optional.of(born))) {
        throw new IOException("process owner changed during sample");
      }
      return new Snapshot("linux-pid:" + pid + "@" + born, values);
    });
  }

  public static Probe linuxPss(String label, ProcessHandle process) {
    return linuxPss(label, process.pid(), process::info, Path.of("/proc"));
  }

  static Probe linuxPss(String label, long pid, Supplier<ProcessHandle.Info> info, Path proc) {
    return new Probe(label, Scope.PROCESS, () -> {
      var born = info.get().startInstant().orElseThrow(() -> new IOException("process birth time unavailable"));
      var text = boundedAscii(proc.resolve(Long.toString(pid)).resolve("smaps_rollup"), 16 * 1024);
      long kb = readStatus(text, PSS).orElseThrow(() -> new IOException("Linux PSS unavailable"));
      if (!info.get().startInstant().equals(Optional.of(born))) {
        throw new IOException("process owner changed during PSS sample");
      }
      return new Snapshot("linux-pid:" + pid + "@" + born, Map.of(PSS_BYTES, Math.multiplyExact(kb, 1024)));
    });
  }

  /** The caller selects a cgroup v2 directory such as the dedicated service scope. */
  public static Probe linuxCgroupMemory(String label, Path directory) {
    return linuxCgroupMemory(label, directory,
        path -> Files.readAttributes(path, BasicFileAttributes.class).fileKey());
  }

  @FunctionalInterface interface CgroupIdentity {
    Object read(Path path) throws IOException;
  }

  static Probe linuxCgroupMemory(String label, Path directory, CgroupIdentity identity) {
    return new Probe(label, Scope.CGROUP, () -> {
      var cgroup = directory.toRealPath();
      var before = identity.read(cgroup);
      if (before == null) throw new IOException("cgroup identity unavailable");
      long bytes = readCounter(cgroup.resolve("memory.current"));
      var after = identity.read(cgroup);
      if (!before.equals(after)) throw new IOException("cgroup changed during sample");
      return new Snapshot("linux-cgroup:" + before, Map.of(CGROUP_MEMORY_BYTES, bytes));
    });
  }

  private static Optional<Long> readStatus(String status, Pattern field) throws IOException {
    var matcher = field.matcher(status);
    if (!matcher.find()) return Optional.empty();
    long result = Long.parseLong(matcher.group(1));
    if (matcher.find()) throw new IOException("duplicate proc status field");
    return Optional.of(result);
  }

  public static Probe linuxInterface(String label, String name) {
    return linuxInterface(label, name, Path.of("/sys/class/net"));
  }

  static Probe linuxInterface(String label, String name, Path interfaces) {
    if (!name.matches("[a-zA-Z0-9][a-zA-Z0-9_.:-]{0,63}")) {
      throw new IllegalArgumentException("invalid Linux interface name");
    }
    var device = interfaces.resolve(name);
    return new Probe(label, Scope.INTERFACE, () -> {
      long index = readCounter(device.resolve("ifindex"));
      var values = new EnumMap<Metric, Long>(Metric.class);
      for (var entry : NETWORK.entrySet()) {
        values.put(entry.getKey(), readCounter(device.resolve("statistics").resolve(entry.getValue())));
      }
      return new Snapshot("linux-interface:" + name + "@" + index, values);
    });
  }

  private static long readCounter(Path path) throws IOException {
    return Long.parseLong(boundedAscii(path, 64).strip());
  }

  private static String boundedAscii(Path path, int limit) throws IOException {
    try (var input = Files.newInputStream(path)) {
      byte[] bytes = input.readNBytes(limit + 1);
      if (bytes.length > limit) throw new IOException("resource counter input bound");
      return new String(bytes, StandardCharsets.US_ASCII);
    }
  }

  public static Probe windowsProcess(String label, long pid) {
    return windowsProcess(label, pid, WindowsIMapResourceQuery.SYSTEM);
  }

  static Probe windowsProcess(String label, long pid, WindowsIMapResourceQuery.Query query) {
    if (pid <= 0) throw new IllegalArgumentException("positive process ID required");
    return new Probe(label, Scope.PROCESS, () -> {
      var words = fields(query.read(WindowsIMapResourceQuery.PROCESS, Long.toString(pid)));
      long born = Long.parseLong(words[0]);
      return new Snapshot("windows-pid:" + pid + "@" + born, Map.of(
          CPU_NANOS, Math.multiplyExact(Long.parseLong(words[1]), 100),
          RSS_BYTES, Long.parseLong(words[2]), PROCESS_THREADS, Long.parseLong(words[3])));
    });
  }

  public static Probe windowsInterface(String label, String interfaceId) {
    return windowsInterface(label, interfaceId, WindowsIMapResourceQuery.SYSTEM);
  }

  static Probe windowsInterface(String label, String interfaceId, WindowsIMapResourceQuery.Query query) {
    if (!interfaceId.matches("[a-zA-Z0-9{}_.:-]{1,128}")) {
      throw new IllegalArgumentException("invalid Windows interface ID");
    }
    return new Probe(label, Scope.INTERFACE, () -> {
      var words = fields(query.read(WindowsIMapResourceQuery.INTERFACE, interfaceId));
      return new Snapshot("windows-interface:" + interfaceId, Map.of(
          RX_BYTES, Long.parseLong(words[0]), TX_BYTES, Long.parseLong(words[1]),
          RX_PACKETS, Long.parseLong(words[2]), TX_PACKETS, Long.parseLong(words[3])));
    });
  }

  private static String[] fields(String output) throws IOException {
    String[] words = output.strip().split("\\s+");
    if (words.length != 4) throw new IOException("invalid Windows resource sample");
    return words;
  }
}
