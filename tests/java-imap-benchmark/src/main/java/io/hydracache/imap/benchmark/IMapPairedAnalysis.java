package io.hydracache.imap.benchmark;

import io.hydracache.imap.semantic.ErrorClass;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Optional;
import java.util.SplittableRandom;
import java.util.function.ToDoubleFunction;
import java.util.stream.IntStream;

/** Paired development statistics for an exact preregistered fixed-rate campaign. */
public final class IMapPairedAnalysis {
  private static final double ADVANTAGE = StrictMath.log(1.10);
  private static final double TAIL_ADVANTAGE = StrictMath.log(0.90);
  private static final double TAIL_REGRESSION = StrictMath.log(1.05);
  private static final double GOODPUT_REGRESSION = StrictMath.log(0.95);

  private IMapPairedAnalysis() {}

  public record Policy(long bootstrapSeed, int bootstrapIterations, int confidencePermille,
                       long p99LimitNanos, long p999LimitNanos) {
    public Policy {
      if (bootstrapIterations < 1000 || bootstrapIterations > 5000
          || confidencePermille < 900 || confidencePermille > 999
          || p99LimitNanos <= 0 || p999LimitNanos < p99LimitNanos) {
        throw new IllegalArgumentException("invalid preregistered IMap analysis policy");
      }
    }
  }

  public enum Verdict {
    LEFT_ADVANTAGE, RIGHT_ADVANTAGE, EQUIVALENT, INCONCLUSIVE,
    NOT_COMPARABLE, INVALID_INFRASTRUCTURE
  }

  public record Measurement(double goodput, long p99Nanos, long p999Nanos) {}
  public record Observation(int pairNumber, Measurement left, Measurement right) {}

  /** Relative effects are fractions of the left measurement, e.g. 0.10 for +10%. */
  public record Metric(double leftMedian, double rightMedian, double relativeEffect,
                       double lower, double upper) {}

  public record Statistics(List<Observation> observations, Metric goodput, Metric p99, Metric p999) {
    public Statistics { observations = List.copyOf(observations); }
  }

  public record CellReport(String cell, Verdict verdict, String reason,
                           List<IMapLoadCampaign.Attempt> attempts, Optional<Statistics> statistics,
                           List<IMapResourceAnalysis.Summary> resources) {
    public CellReport(String cell, Verdict verdict, String reason,
        List<IMapLoadCampaign.Attempt> attempts, Optional<Statistics> statistics) {
      this(cell, verdict, reason, List.copyOf(attempts), statistics,
          IMapResourceAnalysis.summarize(attempts, statistics.isPresent()));
    }

    public CellReport {
      attempts = List.copyOf(attempts);
      resources = List.copyOf(resources);
    }
  }

  public static List<CellReport> analyze(IMapLoadCampaign campaign, List<IMapLoadCampaign.Attempt> attempts) {
    var runs = campaign.runs();
    if (attempts.size() > runs.size()) throw new IllegalArgumentException("too many campaign attempts");
    attempts = List.copyOf(attempts);
    for (int i = 0; i < attempts.size(); i++) {
      if (!runs.get(i).equals(attempts.get(i).run())) {
        throw new IllegalArgumentException("campaign attempt differs from the planned run at index " + i);
      }
    }
    var reports = new ArrayList<CellReport>();
    int start = 0;
    for (var cell : campaign.cells()) {
      int end = Math.min(start + cell.runs().size(), attempts.size());
      reports.add(analyzeCell(cell, attempts.subList(start, end), campaign.analysisPolicy()));
      start = end;
    }
    return List.copyOf(reports);
  }

  private static CellReport analyzeCell(IMapLoadCampaign.Cell cell,
      List<IMapLoadCampaign.Attempt> attempts, Policy policy) {
    var observations = new ArrayList<Observation>();
    FixedRateIMapLoad.Pair first = null;
    for (var attempt : attempts) {
      if (attempt instanceof IMapLoadCampaign.Failed failed) {
        var verdict = switch (failed.failure()) {
          case NOT_COMPARABLE -> Verdict.NOT_COMPARABLE;
          case INFRASTRUCTURE -> Verdict.INVALID_INFRASTRUCTURE;
          case CANCELLED -> Verdict.INCONCLUSIVE;
        };
        return refused(cell, attempts, verdict, failed.detail());
      }
      var run = attempt.run();
      var result = ((IMapLoadCampaign.Measured) attempt).result();
      validatePair(run, result);
      if (!result.admission().equivalent()) {
        return refused(cell, attempts, Verdict.NOT_COMPARABLE, "semantic preflight failed");
      }
      if (first == null) first = result;
      if (!first.preloaded().equals(result.preloaded()) || !first.admission().equals(result.admission())) {
        return refused(cell, attempts, Verdict.NOT_COMPARABLE, "semantic admission or preload changed between pairs");
      }
      for (var trial : List.of(result.left(), result.right())) {
        var warmup = trial.warmup();
        var measured = trial.measured();
        if (warmup.lateSlots() > 0 || measured.lateSlots() > 0) {
          return refused(cell, attempts, Verdict.INVALID_INFRASTRUCTURE, "generator missed scheduled slots");
        }
        if (!warmup.errors().isEmpty() || !measured.errors().isEmpty()) {
          return refused(cell, attempts, Verdict.INCONCLUSIVE, "operation errors in the run");
        }
        if (measured.successful() == 0 || measured.endToEnd().orElseThrow().p99() == 0) {
          return refused(cell, attempts, Verdict.INCONCLUSIVE, "positive goodput and p99 required");
        }
      }
      observations.add(new Observation(run.pairNumber(), measure(result.left()), measure(result.right())));
    }
    if (attempts.size() != cell.runs().size()) {
      return refused(cell, attempts, Verdict.INCONCLUSIVE, "planned pairs are incomplete");
    }
    return compare(cell, attempts, observations, policy);
  }

  private static CellReport refused(IMapLoadCampaign.Cell cell, List<IMapLoadCampaign.Attempt> attempts,
      Verdict verdict, String reason) {
    return new CellReport(cell.name(), verdict, reason, attempts, Optional.empty());
  }

  private static void validatePair(IMapLoadCampaign.Run run, FixedRateIMapLoad.Pair result) {
    if (!run.settings().equals(result.settings())
        || !run.workloadFingerprint().equals(result.workloadFingerprint())
        || run.rightFirst() != result.rightFirst()) {
      throw new IllegalArgumentException("measurement configuration differs from planned pair");
    }
    if (!result.left().adapter().equals(result.admission().leftIdentity())
        || !result.right().adapter().equals(result.admission().rightIdentity())) {
      throw new IllegalArgumentException("measured adapters differ from semantic admission");
    }
    for (var trial : List.of(result.left(), result.right())) {
      validatePhase(trial.warmup(), run.settings().warmupRequests(), run.settings());
      validatePhase(trial.measured(), run.settings().measuredRequests(), run.settings());
    }
  }

  private static void validatePhase(FixedRateIMapLoad.Phase phase, int expected,
      FixedRateIMapLoad.Settings settings) {
    if (phase.scheduled() != expected || phase.offeredWindowNanos() != settings.offset(expected)
        || phase.elapsedNanos() < 0
        || IntStream.of(phase.completed(), phase.successful(), phase.lateSlots(), phase.capacityRejected())
            .anyMatch(count -> count < 0)
        || (long) phase.completed() + phase.lateSlots() + phase.capacityRejected() != expected) {
      throw new IllegalArgumentException("invalid IMap phase accounting");
    }
    long errors = 0;
    for (var entry : phase.errors().entrySet()) {
      if (entry.getKey() == ErrorClass.NONE || entry.getValue() <= 0) {
        throw new IllegalArgumentException("invalid IMap error count");
      }
      errors += entry.getValue();
    }
    if (errors + phase.successful() != phase.completed()) {
      throw new IllegalArgumentException("IMap completed outcomes do not add up");
    }
    for (var histogram : List.of(phase.endToEnd(), phase.queue(), phase.service())) {
      validateHistogram(histogram, phase.completed());
    }
    phase.histograms().ifPresent(raw -> {
      validateRaw(raw.endToEnd(), phase.endToEnd(), phase.completed());
      validateRaw(raw.queue(), phase.queue(), phase.completed());
      validateRaw(raw.service(), phase.service(), phase.completed());
    });
  }

  private static void validateRaw(FixedRateIMapLoad.Histogram raw,
      Optional<FixedRateIMapLoad.Latencies> reported, int completed) {
    if (raw.samples() != completed || !raw.quantiles().equals(reported)) {
      throw new IllegalArgumentException("raw IMap histogram differs from phase totals or quantiles");
    }
  }

  private static void validateHistogram(Optional<FixedRateIMapLoad.Latencies> histogram, int completed) {
    if (histogram.isPresent() != (completed > 0)) {
      throw new IllegalArgumentException("IMap latency sample presence differs from completed count");
    }
    histogram.ifPresent(latency -> {
      long[] quantiles = {latency.p50(), latency.p95(), latency.p99(), latency.p999()};
      if (quantiles[0] < 0 || IntStream.range(1, quantiles.length)
          .anyMatch(i -> quantiles[i] < quantiles[i - 1])) {
        throw new IllegalArgumentException("invalid IMap latency quantiles");
      }
    });
  }

  private static Measurement measure(FixedRateIMapLoad.Trial trial) {
    var phase = trial.measured();
    var latency = phase.endToEnd().orElseThrow();
    double goodput = phase.successful() * 1_000_000_000.0
        / Math.max(phase.offeredWindowNanos(), phase.elapsedNanos());
    return new Measurement(goodput, latency.p99(), latency.p999());
  }

  private static CellReport compare(IMapLoadCampaign.Cell cell, List<IMapLoadCampaign.Attempt> attempts,
      List<Observation> observations, Policy policy) {
    var left = observations.stream().map(Observation::left).toList();
    var right = observations.stream().map(Observation::right).toList();
    var goodput = estimate(left.stream().mapToDouble(Measurement::goodput).toArray(),
        right.stream().mapToDouble(Measurement::goodput).toArray(), policy);
    var p99 = estimate(left.stream().mapToDouble(Measurement::p99Nanos).toArray(),
        right.stream().mapToDouble(Measurement::p99Nanos).toArray(), policy);
    var p999 = estimate(left.stream().mapToDouble(Measurement::p999Nanos).toArray(),
        right.stream().mapToDouble(Measurement::p999Nanos).toArray(), policy);
    var statistics = Optional.of(new Statistics(observations, goodput.metric(), p99.metric(), p999.metric()));
    Verdict verdict = Verdict.INCONCLUSIVE;
    String reason = "effect or interval did not meet the practical thresholds";
    if (observations.stream().anyMatch(pair -> exceedsSlo(pair.left(), policy) || exceedsSlo(pair.right(), policy))) {
      reason = "p99 or p99.9 exceeds the preregistered SLO";
    } else if (wins(goodput, p99)) {
      verdict = Verdict.RIGHT_ADVANTAGE;
      reason = "right meets an advantage threshold and its regression guard";
    } else if (wins(goodput.reversed(), p99.reversed())) {
      verdict = Verdict.LEFT_ADVANTAGE;
      reason = "left meets an advantage threshold and its regression guard";
    } else if (goodput.equivalent() && p99.equivalent()) {
      verdict = Verdict.EQUIVALENT;
      reason = "goodput and p99 intervals fit the 5% practical band";
    }
    return new CellReport(cell.name(), verdict, reason, attempts, statistics);
  }

  private static boolean exceedsSlo(Measurement value, Policy policy) {
    return value.p99Nanos() > policy.p99LimitNanos() || value.p999Nanos() > policy.p999LimitNanos();
  }

  private static boolean wins(Estimate goodput, Estimate p99) {
    return goodput.effect() >= ADVANTAGE && goodput.lower() > 0 && p99.effect() <= TAIL_REGRESSION
        || p99.effect() <= TAIL_ADVANTAGE && p99.upper() < 0 && goodput.effect() >= GOODPUT_REGRESSION;
  }

  private record Estimate(double leftMedian, double rightMedian, double effect, double lower, double upper) {
    Estimate reversed() { return new Estimate(rightMedian, leftMedian, -effect, -upper, -lower); }
    boolean equivalent() { return lower >= -TAIL_REGRESSION && upper <= TAIL_REGRESSION; }
    Metric metric() {
      return new Metric(leftMedian, rightMedian, StrictMath.expm1(effect), StrictMath.expm1(lower),
          StrictMath.expm1(upper));
    }
  }

  private static Estimate estimate(double[] left, double[] right, Policy policy) {
    int count = left.length;
    var ratios = new double[count];
    // Each log ratio keeps both observations of a pair together during resampling.
    for (int i = 0; i < count; i++) ratios[i] = StrictMath.log(right[i] / left[i]);
    var walsh = new double[count * (count + 1) / 2];
    double effect = walshMedian(ratios, walsh);
    var interval = bootstrap(ratios, policy, sample -> walshMedian(sample, walsh));
    return new Estimate(median(left), median(right), effect, interval.lower(), interval.upper());
  }

  record Interval(double lower, double upper) {}

  static Interval bootstrap(double[] pairs, Policy policy, ToDoubleFunction<double[]> statistic) {
    var sample = new double[pairs.length];
    var estimates = new double[policy.bootstrapIterations()];
    var random = new SplittableRandom(policy.bootstrapSeed());
    for (int i = 0; i < estimates.length; i++) {
      for (int j = 0; j < sample.length; j++) sample[j] = pairs[random.nextInt(pairs.length)];
      estimates[i] = statistic.applyAsDouble(sample);
    }
    Arrays.sort(estimates);
    double alpha = (1000 - policy.confidencePermille()) / 2000.0;
    int low = (int) Math.floor((estimates.length - 1) * alpha);
    int high = (int) Math.ceil((estimates.length - 1) * (1 - alpha));
    return new Interval(estimates[low], estimates[high]);
  }

  private static double walshMedian(double[] ratios, double[] walsh) {
    int index = 0;
    for (int i = 0; i < ratios.length; i++) {
      for (int j = i; j < ratios.length; j++) walsh[index++] = (ratios[i] + ratios[j]) / 2;
    }
    return median(walsh);
  }

  static double median(double[] values) {
    Arrays.sort(values);
    int middle = values.length / 2;
    return values.length % 2 == 0 ? (values[middle - 1] + values[middle]) / 2 : values[middle];
  }
}
