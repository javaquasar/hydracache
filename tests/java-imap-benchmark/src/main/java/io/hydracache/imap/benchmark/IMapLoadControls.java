package io.hydracache.imap.benchmark;

import io.hydracache.imap.semantic.MapSemanticAdapter;
import java.util.ArrayList;
import java.util.List;
import java.util.OptionalInt;
import java.util.function.BooleanSupplier;
import java.util.function.Function;

/** Local A/A and ascending fixed-rate saturation diagnostics. */
public final class IMapLoadControls {
  private IMapLoadControls() {}

  public static List<IMapPairedAnalysis.CellReport> sameAdapter(IMapLoadCampaign campaign,
      Function<IMapLoadCampaign.Run, ? extends MapSemanticAdapter> factory, BooleanSupplier cancelled) {
    return IMapPairedAnalysis.analyze(campaign, campaign.execute(factory, factory, cancelled));
  }

  public enum Limit { STABLE, INCOMPLETE, INFRASTRUCTURE, CAPACITY, OPERATION_ERRORS, LATENCY }
  public record Step(int rate, Limit limit) {}
  public record Saturation(List<Step> steps, OptionalInt lastSustainableRate) {
    public Saturation { steps = List.copyOf(steps); }
  }

  /** All offered rates must be preregistered for the same workload and concurrency level. */
  public static Saturation saturation(IMapLoadCampaign campaign, List<IMapLoadCampaign.Attempt> attempts) {
    var reports = IMapPairedAnalysis.analyze(campaign, attempts);
    var steps = new ArrayList<Step>();
    var first = campaign.runs().get(0);
    int previous = 0;
    var sustainable = OptionalInt.empty();
    boolean prefixStable = true;
    for (int index = 0; index < reports.size(); index++) {
      var run = campaign.cells().get(index).runs().get(0);
      int rate = run.settings().requestsPerSecond();
      if (!shape(run).equals(shape(first)) || rate <= previous) {
        throw new IllegalArgumentException("IMap saturation needs one ascending load ladder");
      }
      previous = rate;
      var limit = limit(reports.get(index), campaign.cells().get(index).runs().size(), campaign.analysisPolicy());
      steps.add(new Step(rate, limit));
      prefixStable &= limit == Limit.STABLE;
      if (prefixStable) sustainable = OptionalInt.of(rate);
    }
    return new Saturation(steps, sustainable);
  }

  private record Shape(String fingerprint, int workers, int inFlight) {}

  private static Shape shape(IMapLoadCampaign.Run run) {
    return new Shape(run.workloadFingerprint(), run.settings().workers(), run.settings().maxInFlight());
  }

  private static Limit limit(IMapPairedAnalysis.CellReport cell, int expected, IMapPairedAnalysis.Policy policy) {
    for (var attempt : cell.attempts()) {
      if (attempt instanceof IMapLoadCampaign.Failed) return Limit.INFRASTRUCTURE;
      var pair = ((IMapLoadCampaign.Measured) attempt).result();
      for (var trial : List.of(pair.left(), pair.right())) {
        for (var phase : List.of(trial.warmup(), trial.measured())) {
          if (phase.lateSlots() > 0) return Limit.INFRASTRUCTURE;
          if (!phase.errors().isEmpty()) return Limit.OPERATION_ERRORS;
          if (phase.capacityRejected() > 0) return Limit.CAPACITY;
        }
        var quantiles = trial.measured().endToEnd().orElseThrow();
        if (quantiles.p99() > policy.p99LimitNanos() || quantiles.p999() > policy.p999LimitNanos()) return Limit.LATENCY;
      }
    }
    if (cell.attempts().size() < expected) return Limit.INCOMPLETE;
    return cell.statistics().isPresent() ? Limit.STABLE : Limit.INFRASTRUCTURE;
  }
}
