package io.hydracache.imap.benchmark;

import java.io.IOException;
import java.util.ArrayList;
import java.util.EnumMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;
import java.util.OptionalLong;
import java.util.concurrent.TimeoutException;

/** Phase boundary observations. Each metric is reported with its owner and scope. */
public final class IMapResourceMonitor {
  private IMapResourceMonitor() {}

  public enum Side { PAIR, LEFT, RIGHT }
  public enum Phase { PREFLIGHT, PRELOAD, WARMUP, MEASUREMENT, CLEANUP, POST_IDLE }
  public enum Scope { PROCESS, JVM, INTERFACE, CGROUP }
  public enum Status { AVAILABLE, UNAVAILABLE, COUNTER_RESET, SOURCE_CHANGED }

  public enum Metric {
    CPU_NANOS(Scope.PROCESS, true),
    RSS_BYTES(Scope.PROCESS, false),
    PROCESS_THREADS(Scope.PROCESS, false),
    JVM_HEAP_USED_BYTES(Scope.JVM, false),
    JVM_HEAP_COMMITTED_BYTES(Scope.JVM, false),
    JVM_GC_COLLECTIONS(Scope.JVM, true),
    JVM_GC_MILLIS(Scope.JVM, true),
    JVM_THREADS(Scope.JVM, false),
    RX_BYTES(Scope.INTERFACE, true),
    TX_BYTES(Scope.INTERFACE, true),
    RX_PACKETS(Scope.INTERFACE, true),
    TX_PACKETS(Scope.INTERFACE, true),
    PSS_BYTES(Scope.PROCESS, false),
    VOLUNTARY_CONTEXT_SWITCHES(Scope.PROCESS, true),
    INVOLUNTARY_CONTEXT_SWITCHES(Scope.PROCESS, true),
    CGROUP_MEMORY_BYTES(Scope.CGROUP, false);

    private final Scope scope;
    private final boolean counter;

    Metric(Scope scope, boolean counter) {
      this.scope = scope;
      this.counter = counter;
    }

    boolean isCounter() { return counter; }
    Scope scope() { return scope; }
  }

  public record Snapshot(String identity, Map<Metric, Long> values) {
    public Snapshot {
      if (identity.isBlank() || identity.length() > 192) {
        throw new IllegalArgumentException("invalid resource owner identity");
      }
      values = Map.copyOf(values);
      if (values.values().stream().anyMatch(value -> value < 0)) {
        throw new IllegalArgumentException("negative resource reading");
      }
    }
  }

  @FunctionalInterface public interface Source {
    Snapshot read() throws IOException;
  }

  public record Probe(String label, Scope scope, Source source) {
    public Probe {
      requireLabel(label);
      Objects.requireNonNull(scope, "scope");
      Objects.requireNonNull(source, "source");
    }
  }

  static void requireLabel(String label) {
    if (!label.matches("[a-zA-Z0-9][a-zA-Z0-9_-]{0,47}")) {
      throw new IllegalArgumentException("invalid resource probe label");
    }
  }

  /** Shared probes cover the load generator. Side probes can name separate server owners. */
  public record Targets(List<Probe> shared, List<Probe> left, List<Probe> right) {
    public static final Targets NONE = new Targets(List.of(), List.of(), List.of());

    public Targets {
      shared = bounded(shared);
      left = bounded(left);
      right = bounded(right);
      combine(shared, left);
      combine(shared, right);
    }

    private static List<Probe> bounded(List<Probe> probes) {
      if (probes.size() > 8) throw new IllegalArgumentException("resource probe count bound");
      return List.copyOf(probes);
    }

    private static List<Probe> combine(List<Probe> shared, List<Probe> side) {
      var result = new ArrayList<>(shared);
      result.addAll(side);
      var labels = new HashSet<String>();
      for (var probe : result) {
        if (!labels.add(probe.label())) throw new IllegalArgumentException("duplicate resource probe label");
      }
      return List.copyOf(result);
    }

    List<Probe> forSide(Side side) {
      return switch (side) {
        case PAIR -> shared;
        case LEFT -> combine(shared, left);
        case RIGHT -> combine(shared, right);
      };
    }
  }

  public record Reading(long sampledAtNanos, Optional<Snapshot> snapshot, String error) {}
  public record Delta(OptionalLong before, OptionalLong after, OptionalLong change, Status status) {}

  public record Observation(String label, Scope scope, Reading before, Reading after,
                            Map<Metric, Delta> metrics) {
    public Observation {
      requireLabel(label);
      metrics = Map.copyOf(metrics);
    }
  }

  public record Report(Side side, Phase phase, long elapsedNanos, boolean completed,
                       List<Observation> observations) {
    public Report { observations = List.copyOf(observations); }
  }

  @FunctionalInterface interface Action<T> {
    T run() throws InterruptedException, TimeoutException;
  }

  static final class Recorder {
    private final Targets targets;
    private final List<Report> reports;
    private final IMapCampaignCapture.Collector capture;
    final long idleMillis;

    Recorder(Targets targets, List<Report> reports) {
      this(targets, reports, null, 0);
    }

    Recorder(Targets targets, List<Report> reports, IMapCampaignCapture.Collector capture, long idleMillis) {
      this.targets = Objects.requireNonNull(targets, "targets");
      this.reports = reports;
      this.capture = capture;
      this.idleMillis = idleMillis;
    }

    <T> T measure(Side side, io.hydracache.imap.semantic.MapSemanticAdapter adapter, Action<T> action)
        throws InterruptedException, TimeoutException {
      return capture == null ? action.run() : capture.measure(side, adapter, action);
    }

    <T> T around(Side side, Phase phase, Action<T> action) throws InterruptedException, TimeoutException {
      var probes = targets.forSide(side);
      if (probes.isEmpty()) return action.run();
      var before = probes.stream().map(IMapResourceMonitor::sample).toList();
      long started = System.nanoTime();
      boolean completed = false;
      try {
        T value = action.run();
        completed = true;
        return value;
      } finally {
        long elapsed = System.nanoTime() - started;
        var observations = new ArrayList<Observation>();
        for (int i = 0; i < probes.size(); i++) {
          var probe = probes.get(i);
          observations.add(compare(probe, before.get(i), sample(probe)));
        }
        reports.add(new Report(side, phase, elapsed, completed, observations));
      }
    }
  }

  private static Reading sample(Probe probe) {
    try {
      var snapshot = probe.source().read();
      if (!inScope(snapshot, probe.scope())) {
        throw new IllegalArgumentException("resource metric has the wrong scope");
      }
      return new Reading(System.nanoTime(), Optional.of(snapshot), "");
    } catch (IOException | RuntimeException error) {
      String reason = error.getClass().getSimpleName() + ": " + error.getMessage();
      return new Reading(System.nanoTime(), Optional.empty(), reason.substring(0, Math.min(256, reason.length())));
    }
  }

  private static Observation compare(Probe probe, Reading before, Reading after) {
    return new Observation(probe.label(), probe.scope(), before, after, metrics(probe.scope(), before, after));
  }

  private static boolean inScope(Snapshot snapshot, Scope scope) {
    return snapshot.values().keySet().stream().allMatch(metric -> metric.scope == scope);
  }

  static boolean consistent(Observation observation, long elapsedNanos) {
    return observation.after().sampledAtNanos() - observation.before().sampledAtNanos() >= elapsedNanos
        && List.of(observation.before(), observation.after()).stream().flatMap(reading -> reading.snapshot().stream())
            .allMatch(snapshot -> inScope(snapshot, observation.scope()))
        && metrics(observation.scope(), observation.before(), observation.after()).entrySet()
            .containsAll(observation.metrics().entrySet());
  }

  private static Map<Metric, Delta> metrics(Scope scope, Reading before, Reading after) {
    var metrics = new EnumMap<Metric, Delta>(Metric.class);
    boolean changed = before.snapshot().isPresent() && after.snapshot().isPresent()
        && !before.snapshot().orElseThrow().identity().equals(after.snapshot().orElseThrow().identity());
    for (var metric : Metric.values()) {
      if (metric.scope == scope) {
        var left = value(before, metric);
        var right = value(after, metric);
        metrics.put(metric, delta(metric, left, right, changed));
      }
    }
    return metrics;
  }

  private static OptionalLong value(Reading reading, Metric metric) {
    var value = reading.snapshot().map(snapshot -> snapshot.values().get(metric));
    return value.isPresent() ? OptionalLong.of(value.orElseThrow()) : OptionalLong.empty();
  }

  private static Delta delta(Metric metric, OptionalLong before, OptionalLong after, boolean changed) {
    if (changed) return new Delta(before, after, OptionalLong.empty(), Status.SOURCE_CHANGED);
    if (before.isEmpty() || after.isEmpty()) {
      return new Delta(before, after, OptionalLong.empty(), Status.UNAVAILABLE);
    }
    long difference = after.orElseThrow() - before.orElseThrow();
    if (metric.isCounter() && difference < 0) {
      return new Delta(before, after, OptionalLong.empty(), Status.COUNTER_RESET);
    }
    return new Delta(before, after, OptionalLong.of(difference), Status.AVAILABLE);
  }
}
