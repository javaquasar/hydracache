package io.hydracache.imap.hazelcast;

import io.hydracache.imap.semantic.BytesValue;
import java.io.PrintStream;
import java.lang.management.ManagementFactory;
import java.nio.ByteBuffer;
import java.nio.file.*;
import java.time.Duration;
import java.util.*;
import java.util.concurrent.TimeUnit;

/** One JVM/member/client/map/subscription owner across every active round. */
public final class NativeSoakWorker075 {
  private NativeSoakWorker075() {}
  public record Config(long activeMs, long periodMs, long maximumRounds, int warmup, long startupMs, long seed, String source, Path control) {
    public Config {
      if (activeMs < 2 || activeMs > 86400000 || periodMs < 1 || periodMs > 60000 || activeMs / periodMs < 2
          || maximumRounds < 2 || maximumRounds > 8192 || (activeMs + periodMs - 1) / periodMs + 2 > maximumRounds
          || warmup < 1 || warmup > 16 || startupMs < 100 || startupMs > 60000 || !source.matches("[0-9a-f]{40}")
          || !control.isAbsolute() || !Files.isDirectory(control) || Files.isSymbolicLink(control)) throw new IllegalArgumentException("native soak worker bounds");
    }
    public static Config parse(String[] args) {
      if (args.length != 8) throw new IllegalArgumentException("eight native soak arguments required");
      return new Config(Long.parseLong(args[0]), Long.parseLong(args[1]), Long.parseLong(args[2]), Integer.parseInt(args[3]), Long.parseLong(args[4]), Long.parseUnsignedLong(args[5]), args[6], Path.of(args[7]));
    }
  }
  static boolean marker(Path control, String name) throws Exception {
    Path file = control.resolve(name); if (!Files.exists(file)) return false;
    try (var input = Files.newInputStream(file)) {
      if (!Arrays.equals(input.readNBytes(17), (name + "-v1").getBytes(java.nio.charset.StandardCharsets.US_ASCII))) throw new IllegalArgumentException("invalid bounded native control marker");
    }
    return true;
  }
  static String round(NativeListenerRecovery075 owner, Config config, long position) throws Exception {
    var key = BytesValue.fromHex("61");
    for (long step = 2 * position - 1; step <= 2 * position; step++) owner.put(key, BytesValue.copyOf(ByteBuffer.allocate(8).putLong(config.seed() ^ step).array()));
    long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5);
    while (owner.observedCallbacks() < 2 && System.nanoTime() < deadline) Thread.sleep(1);
    if (owner.observedCallbacks() != 2) throw new IllegalStateException("native soak callbacks incomplete");
    var cut = owner.recover(NativeListenerRecovery075.Bounds.smoke(), new NativeListenerRecovery075.Cancellation());
    if (cut.baseline().entries().size() != 1 || !Arrays.equals(cut.baseline().entries().get(0).value().copy(), ByteBuffer.allocate(8).putLong(config.seed() ^ (2 * position)).array())) throw new IllegalStateException("native soak logical value differs");
    return cut.baseline().digest();
  }
  static void emit(PrintStream out, Config config, NativeListenerRecovery075 owner, String incarnation, String phase, long round, long elapsed, String digest, String reason) {
    var memory = ManagementFactory.getMemoryMXBean().getHeapMemoryUsage();
    String cpu = ProcessHandle.current().info().totalCpuDuration().map(d -> Long.toString(d.toNanos())).orElse("null");
    long gc = ManagementFactory.getGarbageCollectorMXBeans().stream().mapToLong(bean -> Math.max(0, bean.getCollectionCount())).sum();
    out.println("{\"schema\":\"imap075-soak-frame-v1\",\"phase\":\"" + phase + "\",\"pid\":" + ProcessHandle.current().pid() + ",\"incarnation\":\"" + incarnation
        + "\",\"cohort\":\"native\",\"seed\":" + Long.toUnsignedString(config.seed()) + ",\"source_sha\":\"" + config.source() + "\",\"round\":" + round + ",\"active_elapsed_ms\":" + elapsed
        + ",\"digest\":" + (digest == null ? "null" : "\"" + digest + "\"") + ",\"owners\":{\"unit\":\"native_fixture_counts\",\"values\":" + owner.ownedKeys()
        + ",\"outcomes\":null,\"pending\":null,\"listeners\":" + owner.ownedSubscriptions() + ",\"streams\":null,\"resync\":null,\"queued\":" + owner.queuedEvents()
        + ",\"correlations\":" + owner.expectedCallbacks() + "},\"heap_bytes\":" + memory.getUsed() + ",\"rss_bytes\":null,\"gc_collections\":" + gc + ",\"cpu_ns\":" + cpu
        + ",\"threads\":" + ManagementFactory.getThreadMXBean().getThreadCount() + ",\"reason\":" + (reason == null ? "null" : "\"" + reason + "\"") + "}");
    out.flush(); if (out.checkError()) throw new IllegalStateException("native soak output failed");
  }
  public static void run(Config config, PrintStream out) throws Exception {
    String incarnation = UUID.randomUUID().toString();
    try (var cluster = new HazelcastLoopback075(); var owner = new NativeListenerRecovery075(cluster.client(), "imap075-soak-" + UUID.randomUUID(), 1)) {
      emit(out, config, owner, incarnation, "ready", 0, 0, null, null);
      long startup = System.nanoTime() + TimeUnit.MILLISECONDS.toNanos(config.startupMs());
      while (!marker(config.control(), "start")) {
        if (marker(config.control(), "cancel")) return;
        if (System.nanoTime() >= startup) throw new IllegalStateException("native start handshake deadline"); Thread.sleep(2);
      }
      String digest = null;
      for (int position = 1; position <= config.warmup(); position++) digest = round(owner, config, position);
      emit(out, config, owner, incarnation, "warmup", 0, 0, digest, null); emit(out, config, owner, incarnation, "active", 0, 0, digest, null);
      long active = System.nanoTime(), rounds = 0; String reason;
      for (;;) {
        if (marker(config.control(), "cancel")) { reason = "cancelled"; break; }
        if (System.nanoTime() - active >= TimeUnit.MILLISECONDS.toNanos(config.activeMs()) && rounds >= 2) { reason = "duration_complete"; break; }
        if (rounds == config.maximumRounds()) { reason = "round_limit"; break; }
        rounds++; digest = round(owner, config, config.warmup() + rounds);
        emit(out, config, owner, incarnation, "round", rounds, TimeUnit.NANOSECONDS.toMillis(System.nanoTime() - active), digest, null);
        long until = System.nanoTime() + TimeUnit.MILLISECONDS.toNanos(config.periodMs());
        while (System.nanoTime() < until && !marker(config.control(), "cancel")) Thread.sleep(Math.min(10, Math.max(1, TimeUnit.NANOSECONDS.toMillis(until - System.nanoTime()))));
      }
      owner.close();
      emit(out, config, owner, incarnation, "drained", rounds, TimeUnit.NANOSECONDS.toMillis(System.nanoTime() - active), digest, reason);
    }
  }
  public static void main(String[] args) throws Exception { run(Config.parse(args), System.out); }
}
