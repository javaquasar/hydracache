package io.hydracache.imap.benchmark;

import io.hydracache.imap.benchmark.IMapResourceMonitor.Metric;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Phase;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Scope;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Side;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Status;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.OptionalDouble;
import java.util.TreeMap;

/** Diagnostic paired resource summaries for a campaign cell. */
public final class IMapResourceAnalysis {
  private IMapResourceAnalysis() {}

  public enum Value { DELTA, PHASE_END, PER_SUCCESS }

  public record Series(Phase phase, String label, Scope scope, Metric metric, Value value) {}

  public record Sample(int pairNumber, OptionalDouble left, OptionalDouble right) {
    boolean complete() { return left.isPresent() && right.isPresent(); }
  }

  /** Effect and interval are right minus left, in the series' own units. */
  public record Comparison(double leftMedian, double rightMedian, double pairedMedianDifference,
                           double lowerDifference, double upperDifference) {}

  public record Summary(Series series, List<Sample> samples, Optional<Comparison> comparison) {
    public Summary { samples = List.copyOf(samples); }
  }

  static List<Summary> summarize(List<IMapLoadCampaign.Attempt> attempts, boolean comparable) {
    var order = Comparator.comparing(Series::phase).thenComparing(Series::label).thenComparing(Series::scope)
        .thenComparing(Series::metric).thenComparing(Series::value);
    var series = new TreeMap<Series, List<Sample>>(order);
    for (var attempt : attempts) {
      var left = values(attempt, Side.LEFT);
      var right = values(attempt, Side.RIGHT);
      var keys = new HashSet<>(left.keySet());
      keys.addAll(right.keySet());
      for (var key : keys) {
        var sample = new Sample(attempt.run().pairNumber(), left.getOrDefault(key, OptionalDouble.empty()),
            right.getOrDefault(key, OptionalDouble.empty()));
        series.computeIfAbsent(key, ignored -> new ArrayList<>()).add(sample);
      }
    }
    return series.entrySet().stream().map(entry -> {
      var samples = entry.getValue();
      Optional<Comparison> comparison = Optional.empty();
      if (comparable && samples.size() == attempts.size() && samples.stream().allMatch(Sample::complete)) {
        comparison = Optional.of(compare(samples, attempts.get(0).run().analysisPolicy()));
      }
      return new Summary(entry.getKey(), samples, comparison);
    }).toList();
  }

  private static Map<Series, OptionalDouble> values(IMapLoadCampaign.Attempt attempt, Side side) {
    var values = new HashMap<Series, OptionalDouble>();
    for (var report : attempt.resources()) {
      if (report.side() != side) continue;
      int successes = successes(attempt, side, report.phase());
      for (var observation : report.observations()) {
        for (var entry : observation.metrics().entrySet()) {
          var metric = entry.getKey();
          var delta = entry.getValue();
          boolean usable = report.completed() && delta.status() == Status.AVAILABLE;
          var change = usable ? delta.change().stream().asDoubleStream().findFirst() : OptionalDouble.empty();
          var key = new Series(report.phase(), observation.label(), observation.scope(), metric, Value.DELTA);
          if (values.putIfAbsent(key, change) != null) {
            throw new IllegalArgumentException("duplicate IMap resource observation for " + side + " " + key);
          }
          if (!metric.isCounter()) {
            var end = usable ? delta.after().stream().asDoubleStream().findFirst() : OptionalDouble.empty();
            values.put(new Series(report.phase(), observation.label(), observation.scope(), metric, Value.PHASE_END), end);
          }
          if (perSuccess(report.phase(), metric)) {
            var normalized = successes > 0 && change.isPresent()
                ? OptionalDouble.of(change.orElseThrow() / successes) : OptionalDouble.empty();
            values.put(new Series(report.phase(), observation.label(), observation.scope(), metric, Value.PER_SUCCESS),
                normalized);
          }
        }
      }
    }
    return values;
  }

  private static int successes(IMapLoadCampaign.Attempt attempt, Side side, Phase phase) {
    if (!(attempt instanceof IMapLoadCampaign.Measured measured)) return 0;
    var trial = side == Side.LEFT ? measured.result().left() : measured.result().right();
    return switch (phase) {
      case WARMUP -> trial.warmup().successful();
      case MEASUREMENT -> trial.measured().successful();
      default -> 0;
    };
  }

  private static boolean perSuccess(Phase phase, Metric metric) {
    if (phase != Phase.WARMUP && phase != Phase.MEASUREMENT) return false;
    return switch (metric) {
      case CPU_NANOS, RX_BYTES, TX_BYTES, RX_PACKETS, TX_PACKETS -> true;
      default -> false;
    };
  }

  private static Comparison compare(List<Sample> samples, IMapPairedAnalysis.Policy policy) {
    var left = new double[samples.size()];
    var right = new double[samples.size()];
    var differences = new double[samples.size()];
    for (int i = 0; i < samples.size(); i++) {
      var sample = samples.get(i);
      left[i] = sample.left().orElseThrow();
      right[i] = sample.right().orElseThrow();
      differences[i] = right[i] - left[i];
    }
    double effect = IMapPairedAnalysis.median(differences.clone());
    var interval = IMapPairedAnalysis.bootstrap(differences, policy, IMapPairedAnalysis::median);
    return new Comparison(IMapPairedAnalysis.median(left), IMapPairedAnalysis.median(right),
        effect, interval.lower(), interval.upper());
  }
}
