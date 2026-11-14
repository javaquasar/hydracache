package io.hydracache.imap.benchmark;

import io.hydracache.imap.semantic.AdapterUnavailableException;
import io.hydracache.imap.semantic.MapSemanticAdapter;
import io.hydracache.imap.semantic.SemanticMismatchException;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Objects;
import java.util.SplittableRandom;
import java.util.concurrent.CancellationException;
import java.util.concurrent.TimeoutException;
import java.util.function.BooleanSupplier;
import java.util.function.Function;

/** Immutable, bounded matrix of fixed-rate development runs. */
public final class IMapLoadCampaign {
  public static final int MIN_PAIRS = 5;
  public static final int MAX_PAIRS = 32;
  private static final int MAX_CELLS = 64;

  public record Variant(String name, IMapLoadWorkload workload) {
    public Variant {
      if (!name.matches("[a-z0-9][a-z0-9-]{0,47}")) {
        throw new IllegalArgumentException("invalid IMap workload name");
      }
      Objects.requireNonNull(workload, "workload");
    }
  }

  public record Parallelism(int workers, int maxInFlight) {}

  /** Digests supplied by the caller for the exact development environment. */
  public record Bindings(String leftArtifact, String rightArtifact, String tooling, String harness,
                         String configuration, String host) {
    public Bindings {
      for (String digest : List.of(leftArtifact, rightArtifact, tooling, harness, configuration, host)) {
        if (!digest.matches("[0-9a-f]{64}")) {
          throw new IllegalArgumentException("campaign bindings require SHA-256 digests");
        }
      }
    }
  }

  public record Run(String cell, int pairNumber, boolean rightFirst, String workloadFingerprint,
                    FixedRateIMapLoad.Settings settings, Bindings bindings,
                    IMapPairedAnalysis.Policy analysisPolicy) {}

  public record Cell(String name, List<Run> runs) {
    public Cell { runs = List.copyOf(runs); }
  }

  public sealed interface Attempt permits Measured, Failed {
    Run run();
    List<IMapResourceMonitor.Report> resources();
  }

  public record Measured(Run run, FixedRateIMapLoad.Pair result,
                         List<IMapResourceMonitor.Report> resources) implements Attempt {
    public Measured(Run run, FixedRateIMapLoad.Pair result) { this(run, result, List.of()); }

    public Measured {
      Objects.requireNonNull(run, "run");
      Objects.requireNonNull(result, "result");
      resources = List.copyOf(resources);
    }
  }

  public enum Failure { NOT_COMPARABLE, INFRASTRUCTURE, CANCELLED }

  public record Failed(Run run, Failure failure, String detail,
                       List<IMapResourceMonitor.Report> resources) implements Attempt {
    public Failed(Run run, Failure failure, String detail) { this(run, failure, detail, List.of()); }

    public Failed {
      Objects.requireNonNull(run, "run");
      Objects.requireNonNull(failure, "failure");
      Objects.requireNonNull(detail, "detail");
      if (detail.length() > 512) throw new IllegalArgumentException("campaign failure detail bound");
      resources = List.copyOf(resources);
    }
  }

  private final List<Cell> cells;
  private final List<Run> runs;
  private final List<IMapLoadWorkload> workloads;
  private final IMapPairedAnalysis.Policy analysis;

  private IMapLoadCampaign(List<Cell> cells, List<IMapLoadWorkload> workloads,
      IMapPairedAnalysis.Policy analysis) {
    this.cells = List.copyOf(cells);
    this.runs = this.cells.stream().flatMap(cell -> cell.runs().stream()).toList();
    this.workloads = List.copyOf(workloads);
    this.analysis = Objects.requireNonNull(analysis, "analysis");
  }

  public static IMapLoadCampaign plan(List<Variant> variants, List<Integer> rates,
      List<Parallelism> parallelism, FixedRateIMapLoad.Settings template, int pairs,
      Bindings bindings, IMapPairedAnalysis.Policy analysis) {
    variants = axis(variants);
    rates = axis(rates);
    parallelism = axis(parallelism);
    long size = (long) variants.size() * rates.size() * parallelism.size();
    if (size > MAX_CELLS || pairs < MIN_PAIRS || pairs > MAX_PAIRS) {
      throw new IllegalArgumentException("IMap campaign matrix or repeat count out of bounds");
    }
    if (variants.stream().map(Variant::name).distinct().count() != variants.size()) {
      throw new IllegalArgumentException("duplicate IMap workload name");
    }
    Objects.requireNonNull(bindings, "bindings");
    var cells = new ArrayList<Cell>();
    var workloads = new ArrayList<IMapLoadWorkload>();
    var random = new SplittableRandom(template.seed());
    boolean rightFirst = random.nextBoolean();
    for (var variant : variants) {
      String fingerprint = variant.workload().fingerprint();
      for (int rate : rates) {
        for (var limits : parallelism) {
          String name = variant.name() + "-r" + rate + "-w" + limits.workers() + "-i" + limits.maxInFlight();
          var runs = new ArrayList<Run>();
          boolean order = rightFirst;
          for (int pair = 1; pair <= pairs; pair++) {
            var settings = new FixedRateIMapLoad.Settings(random.nextLong(), rate,
                template.warmupRequests(), template.measuredRequests(), limits.workers(),
                limits.maxInFlight(), template.drainTimeoutMillis());
            runs.add(new Run(name, pair, order, fingerprint, settings, bindings, analysis));
            order = !order;
          }
          cells.add(new Cell(name, runs));
          workloads.add(variant.workload());
          rightFirst = !rightFirst;
        }
      }
    }
    return new IMapLoadCampaign(cells, workloads, analysis);
  }

  private static <T> List<T> axis(List<T> values) {
    if (values.isEmpty() || values.size() > MAX_CELLS) {
      throw new IllegalArgumentException("IMap campaign axis out of bounds");
    }
    var copy = List.copyOf(values);
    if (new HashSet<>(copy).size() != copy.size()) {
      throw new IllegalArgumentException("duplicate IMap campaign axis entry");
    }
    return copy;
  }

  public List<Cell> cells() { return cells; }
  public List<Run> runs() { return runs; }
  public IMapPairedAnalysis.Policy analysisPolicy() { return analysis; }

  /** Executes each planned pair once and retains failures in their original positions. */
  public List<Attempt> execute(Function<Run, ? extends MapSemanticAdapter> left,
      Function<Run, ? extends MapSemanticAdapter> right, BooleanSupplier cancelled) {
    return execute(left, right, cancelled, run -> IMapResourceMonitor.Targets.NONE);
  }

  public List<Attempt> execute(Function<Run, ? extends MapSemanticAdapter> left,
      Function<Run, ? extends MapSemanticAdapter> right, BooleanSupplier cancelled,
      Function<Run, IMapResourceMonitor.Targets> targets) {
    return collect(left, right, cancelled, targets, run -> null, 0).stream()
        .map(IMapCampaignPublication.PairEvidence::attempt).toList();
  }

  public List<IMapCampaignPublication.PairEvidence> executeCaptured(
      Function<Run, ? extends MapSemanticAdapter> left, Function<Run, ? extends MapSemanticAdapter> right,
      BooleanSupplier cancelled, Function<Run, IMapResourceMonitor.Targets> targets,
      Function<Run, IMapCampaignCapture.Source> capture, long idleMillis) {
    if (idleMillis < 0 || idleMillis > 60_000) throw new IllegalArgumentException("IMap post-idle interval out of bounds");
    return collect(left, right, cancelled, targets,
        run -> new IMapCampaignCapture.Collector(capture.apply(run), System::nanoTime), idleMillis);
  }

  private List<IMapCampaignPublication.PairEvidence> collect(
      Function<Run, ? extends MapSemanticAdapter> left, Function<Run, ? extends MapSemanticAdapter> right,
      BooleanSupplier cancelled, Function<Run, IMapResourceMonitor.Targets> targets,
      Function<Run, IMapCampaignCapture.Collector> captures, long idleMillis) {
    var generator = new FixedRateIMapLoad();
    var attempts = new ArrayList<IMapCampaignPublication.PairEvidence>();
    for (int index = 0; index < cells.size(); index++) {
      var workload = workloads.get(index);
      for (var run : cells.get(index).runs()) {
        var reports = new ArrayList<IMapResourceMonitor.Report>();
        try {
          var capture = captures.apply(run);
          var recorder = new IMapResourceMonitor.Recorder(targets.apply(run), reports, capture, idleMillis);
          var result = generator.runInOrder(run.settings(), workload, () -> left.apply(run),
              () -> right.apply(run), cancelled, run.rightFirst(), recorder);
          attempts.add(new IMapCampaignPublication.PairEvidence(new Measured(run, result, reports),
              java.util.Optional.ofNullable(capture).map(IMapCampaignCapture.Collector::complete)));
        } catch (SemanticMismatchException | AdapterUnavailableException failure) {
          attempts.add(failed(run, Failure.NOT_COMPARABLE, failure, reports));
        } catch (InterruptedException failure) {
          Thread.currentThread().interrupt();
          attempts.add(failed(run, Failure.CANCELLED, failure, reports));
          return List.copyOf(attempts);
        } catch (CancellationException failure) {
          attempts.add(failed(run, Failure.CANCELLED, failure, reports));
          return List.copyOf(attempts);
        } catch (TimeoutException | RuntimeException failure) {
          attempts.add(failed(run, Failure.INFRASTRUCTURE, failure, reports));
        }
      }
    }
    return List.copyOf(attempts);
  }

  private static IMapCampaignPublication.PairEvidence failed(
      Run run, Failure kind, Exception error, List<IMapResourceMonitor.Report> reports) {
    String detail = error.getClass().getSimpleName() + ": " + error.getMessage();
    return new IMapCampaignPublication.PairEvidence(
        new Failed(run, kind, detail.substring(0, Math.min(512, detail.length())), reports), java.util.Optional.empty());
  }
}
