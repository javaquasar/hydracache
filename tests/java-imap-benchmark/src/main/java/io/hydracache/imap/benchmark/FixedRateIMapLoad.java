package io.hydracache.imap.benchmark;

import io.hydracache.imap.benchmark.IMapResourceMonitor.Recorder;
import io.hydracache.imap.benchmark.IMapResourceMonitor.Side;
import io.hydracache.imap.semantic.AdapterIdentity;
import io.hydracache.imap.semantic.ErrorClass;
import io.hydracache.imap.semantic.MapSemanticAdapter;
import io.hydracache.imap.semantic.OperationOutcome;
import io.hydracache.imap.semantic.OracleReport;
import io.hydracache.imap.semantic.OutcomeKind;
import io.hydracache.imap.semantic.ScenarioStep;
import io.hydracache.imap.semantic.SemanticMismatchException;
import io.hydracache.imap.semantic.SemanticOracle;
import io.hydracache.imap.semantic.StateSnapshot;
import java.util.ArrayList;
import java.util.Collections;
import java.util.EnumMap;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;
import java.util.Set;
import java.util.SplittableRandom;
import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.CancellationException;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.Executor;
import java.util.concurrent.ExecutorCompletionService;
import java.util.concurrent.ThreadPoolExecutor;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import java.util.concurrent.locks.LockSupport;
import java.util.function.BooleanSupplier;
import java.util.function.Supplier;

/**
 * Common development IMap load generator. Uses fresh adapters for semantic preflight and
 * measurement. Adapter factories and cleanup must be bounded, and adapters must support
 * concurrent calls and terminate in-flight work on interruption or their own deadline.
 */
public final class FixedRateIMapLoad {
  private static final long SECOND = 1_000_000_000L;
  private static final long WAIT_SLICE = 1_000_000L;
  private static final StateSnapshot EMPTY = new StateSnapshot(
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", 0, 0);

  public record Settings(long seed, int requestsPerSecond, int warmupRequests, int measuredRequests,
                         int workers, int maxInFlight, int drainTimeoutMillis) {
    public Settings {
      bounded(requestsPerSecond, 1, 1_000_000);
      bounded(warmupRequests, 0, 1_000_000);
      bounded(measuredRequests, 1, 1_000_000);
      bounded(workers, 1, 128);
      bounded(maxInFlight, workers, 4096);
      bounded(drainTimeoutMillis, 1, 60_000);
      if ((long) Math.max(warmupRequests, measuredRequests) > 600L * requestsPerSecond) {
        throw new IllegalArgumentException("IMap load phase exceeds ten minutes");
      }
    }

    private static void bounded(int value, int lower, int upper) {
      if (value < lower || value > upper) throw new IllegalArgumentException("IMap load setting out of bounds");
    }

    long offset(int slot) {
      return slot * SECOND / requestsPerSecond;
    }
  }

  /** All four values are histogram bucket upper bounds in nanoseconds. */
  public record Latencies(long p50, long p95, long p99, long p999) {}

  public record Bucket(long upperNanos, int count) {}

  public record Histogram(int samples, List<Bucket> buckets) {
    public Histogram {
      buckets = List.copyOf(buckets);
      long total = 0;
      long previous = -1;
      for (var bucket : buckets) {
        if (bucket.upperNanos() <= previous || bucket.count() <= 0) {
          throw new IllegalArgumentException("invalid IMap histogram bucket");
        }
        previous = bucket.upperNanos();
        total += bucket.count();
      }
      if (total != samples || buckets.size() > 57 * 128) {
        throw new IllegalArgumentException("invalid IMap histogram sample count");
      }
    }

    public Optional<Latencies> quantiles() {
      if (samples == 0) return Optional.empty();
      return Optional.of(new Latencies(quantile(500), quantile(950), quantile(990), quantile(999)));
    }

    private long quantile(int permille) {
      long rank = (samples * (long) permille + 999) / 1000;
      int index = 0;
      while (rank > buckets.get(index).count()) {
        rank -= buckets.get(index++).count();
      }
      return buckets.get(index).upperNanos();
    }
  }

  public record Histograms(Histogram endToEnd, Histogram queue, Histogram service) {}

  public record Phase(
      int scheduled, int completed, int successful, int lateSlots, int capacityRejected,
      long offeredWindowNanos, long elapsedNanos, Map<ErrorClass, Integer> errors,
      Optional<Latencies> endToEnd, Optional<Latencies> queue, Optional<Latencies> service,
      Optional<Histograms> histograms) {
    public Phase(int scheduled, int completed, int successful, int lateSlots, int capacityRejected,
        long offeredWindowNanos, long elapsedNanos, Map<ErrorClass, Integer> errors,
        Optional<Latencies> endToEnd, Optional<Latencies> queue, Optional<Latencies> service) {
      this(scheduled, completed, successful, lateSlots, capacityRejected, offeredWindowNanos,
          elapsedNanos, errors, endToEnd, queue, service, Optional.empty());
    }

    public Phase {
      errors = Map.copyOf(errors);
      Objects.requireNonNull(histograms);
    }
  }

  public record Trial(AdapterIdentity adapter, Phase warmup, Phase measured) {}
  public record Pair(Settings settings, String workloadFingerprint, boolean rightFirst,
                     OracleReport admission, StateSnapshot preloaded, Trial left, Trial right) {}

  interface Clock {
    long now();
    void pause(long nanos) throws InterruptedException;
  }

  private static final Clock SYSTEM_CLOCK = new Clock() {
    @Override public long now() { return System.nanoTime(); }
    @Override public void pause(long nanos) {
      LockSupport.parkNanos(nanos);
    }
  };

  public Pair run(Settings settings, IMapLoadWorkload workload,
      Supplier<? extends MapSemanticAdapter> leftFactory,
      Supplier<? extends MapSemanticAdapter> rightFactory, BooleanSupplier cancelled)
      throws InterruptedException, TimeoutException {
    return runInOrder(settings, workload, leftFactory, rightFactory, cancelled, false);
  }

  Pair runInOrder(Settings settings, IMapLoadWorkload workload,
      Supplier<? extends MapSemanticAdapter> leftFactory,
      Supplier<? extends MapSemanticAdapter> rightFactory, BooleanSupplier cancelled,
      boolean rightFirst) throws InterruptedException, TimeoutException {
    return runInOrder(settings, workload, leftFactory, rightFactory, cancelled, rightFirst,
        new Recorder(IMapResourceMonitor.Targets.NONE, new ArrayList<>()));
  }

  Pair runInOrder(Settings settings, IMapLoadWorkload workload,
      Supplier<? extends MapSemanticAdapter> leftFactory,
      Supplier<? extends MapSemanticAdapter> rightFactory, BooleanSupplier cancelled,
      boolean rightFirst, Recorder resources) throws InterruptedException, TimeoutException {
    Set<MapSemanticAdapter> instances = Collections.newSetFromMap(new IdentityHashMap<>());
    checkActive(cancelled);
    String fingerprint = workload.fingerprint();
    var admission = resources.around(Side.PAIR, IMapResourceMonitor.Phase.PREFLIGHT, () -> {
      try (var left = acquire(leftFactory, instances); var right = acquire(rightFactory, instances)) {
        requireFresh(left);
        requireFresh(right);
        var result = new SemanticOracle().compare(workload.preflight(), left, right);
        result.requireEquivalent();
        return result;
      }
    });
    checkActive(cancelled);
    try (var left = acquire(leftFactory, instances); var right = acquire(rightFactory, instances)) {
      requireAdmitted(left, admission.leftIdentity(), workload);
      requireAdmitted(right, admission.rightIdentity(), workload);
      var preloaded = resources.around(Side.LEFT, IMapResourceMonitor.Phase.PRELOAD,
          () -> preload(left, workload, cancelled));
      var rightPreloaded = resources.around(Side.RIGHT, IMapResourceMonitor.Phase.PRELOAD,
          () -> preload(right, workload, cancelled));
      if (!preloaded.equals(rightPreloaded)) {
        throw new SemanticMismatchException("preloaded map states differ");
      }
      Trial leftResult;
      Trial rightResult;
      if (rightFirst) {
        rightResult = trial(settings, workload.requests(), right, cancelled, Side.RIGHT, resources);
        leftResult = trial(settings, workload.requests(), left, cancelled, Side.LEFT, resources);
      } else {
        leftResult = trial(settings, workload.requests(), left, cancelled, Side.LEFT, resources);
        rightResult = trial(settings, workload.requests(), right, cancelled, Side.RIGHT, resources);
      }
      return new Pair(settings, fingerprint, rightFirst, admission, preloaded, leftResult, rightResult);
    }
  }

  private static MapSemanticAdapter acquire(
      Supplier<? extends MapSemanticAdapter> factory, Set<MapSemanticAdapter> instances) {
    var adapter = Objects.requireNonNull(factory.get(), "adapter");
    if (!instances.add(adapter)) throw new SemanticMismatchException("adapter instance reused");
    return adapter;
  }

  private static void requireFresh(MapSemanticAdapter adapter) {
    if (!EMPTY.equals(adapter.snapshot()) || !adapter.drainEvents().isEmpty()) {
      throw new SemanticMismatchException("load adapter must start empty and quiescent");
    }
  }

  private static void requireAdmitted(
      MapSemanticAdapter adapter, AdapterIdentity identity, IMapLoadWorkload workload) {
    requireFresh(adapter);
    if (!identity.equals(adapter.identity())
        || workload.preflight().steps().stream()
            .anyMatch(step -> !adapter.capabilities().contains(step.operation()))) {
      throw new SemanticMismatchException("load adapter differs from semantic preflight");
    }
  }

  private static StateSnapshot preload(MapSemanticAdapter adapter, IMapLoadWorkload workload,
      BooleanSupplier cancelled) throws InterruptedException {
    for (var request : workload.preload()) {
      checkActive(cancelled);
      if (errorClass(adapter.execute(request)) != ErrorClass.NONE) {
        throw new SemanticMismatchException("preload failed at " + request.id());
      }
      adapter.drainEvents();
    }
    return adapter.snapshot();
  }

  private static Trial trial(Settings settings, List<ScenarioStep> requests,
      MapSemanticAdapter adapter, BooleanSupplier cancelled, Side side, Recorder resources)
      throws InterruptedException, TimeoutException {
    var executor = new ThreadPoolExecutor(settings.workers(), settings.workers(), 0, TimeUnit.SECONDS,
        new ArrayBlockingQueue<>(settings.maxInFlight()), task -> {
          var thread = new Thread(task, "imap-load-worker");
          thread.setDaemon(true);
          return thread;
        });
    try {
      executor.prestartAllCoreThreads();
      var random = new SplittableRandom(settings.seed());
      var warmup = observedPhase(resources, side, IMapResourceMonitor.Phase.WARMUP, executor,
          () -> runPhase(settings, settings.warmupRequests(), requests, adapter, cancelled,
              executor, SYSTEM_CLOCK, random.split()));
      var measured = resources.measure(side, adapter, () -> observedPhase(resources, side,
          IMapResourceMonitor.Phase.MEASUREMENT, executor,
          () -> runPhase(settings, settings.measuredRequests(), requests, adapter, cancelled,
              executor, SYSTEM_CLOCK, random)));
      if (resources.idleMillis > 0) {
        resources.around(side, IMapResourceMonitor.Phase.POST_IDLE, () -> {
          long start = System.nanoTime();
          long interval = TimeUnit.MILLISECONDS.toNanos(resources.idleMillis);
          long remaining;
          while ((remaining = interval - (System.nanoTime() - start)) > 0) {
            checkActive(cancelled);
            LockSupport.parkNanos(Math.min(remaining, TimeUnit.MILLISECONDS.toNanos(10)));
          }
          checkActive(cancelled);
          return null;
        });
      }
      return new Trial(adapter.identity(), warmup, measured);
    } finally {
      resources.around(side, IMapResourceMonitor.Phase.CLEANUP, () -> {
        executor.shutdownNow();
        if (!executor.awaitTermination(settings.drainTimeoutMillis(), TimeUnit.MILLISECONDS)) {
          throw new TimeoutException("IMap load workers did not terminate");
        }
        return null;
      });
    }
  }

  private static Phase observedPhase(Recorder resources, Side side, IMapResourceMonitor.Phase phase,
      ThreadPoolExecutor executor, IMapResourceMonitor.Action<Phase> action)
      throws InterruptedException, TimeoutException {
    return resources.around(side, phase, () -> {
      try {
        return action.run();
      } catch (InterruptedException | TimeoutException | RuntimeException failure) {
        executor.shutdownNow();
        throw failure;
      }
    });
  }

  static Phase runPhase(Settings settings, int slots, List<ScenarioStep> requests,
      MapSemanticAdapter adapter, BooleanSupplier cancelled, Executor executor, Clock clock,
      SplittableRandom random) throws InterruptedException, TimeoutException {
    var completions = new ExecutorCompletionService<Sample>(executor);
    var results = new Results();
    long window = settings.offset(slots);
    long deadline = window + TimeUnit.MILLISECONDS.toNanos(settings.drainTimeoutMillis());
    long start = clock.now();
    for (int slot = 0; slot < slots; slot++) {
      long due = settings.offset(slot);
      long next = settings.offset(slot + 1);
      // Consume the same random draw even if the corresponding request is dropped.
      var request = requests.get(random.nextInt(requests.size()));
      while (true) {
        checkActive(cancelled);
        results.collect(completions, deadline);
        long elapsed = clock.now() - start;
        if (elapsed >= due) {
          if (elapsed >= next) {
            results.lateSlots++;
          } else if (results.inFlight == settings.maxInFlight()) {
            results.capacityRejected++;
          } else {
            completions.submit(() -> {
              long entered = clock.now() - start;
              var outcome = adapter.execute(request);
              long finished = clock.now() - start;
              adapter.drainEvents();
              return new Sample(entered - due, finished - entered, errorClass(outcome),
                  clock.now() - start);
            });
            results.inFlight++;
          }
          break;
        }
        clock.pause(Math.min(due - elapsed, WAIT_SLICE));
      }
    }
    while (true) {
      checkActive(cancelled);
      results.collect(completions, deadline);
      if (results.inFlight == 0) break;
      long remaining = deadline - (clock.now() - start);
      if (remaining <= 0) throw new TimeoutException("IMap load phase did not drain");
      clock.pause(Math.min(remaining, WAIT_SLICE));
    }
    return results.finish(slots, window, clock.now() - start);
  }

  private static void checkActive(BooleanSupplier cancelled) throws InterruptedException {
    if (Thread.interrupted()) throw new InterruptedException("IMap load interrupted");
    if (cancelled.getAsBoolean()) throw new CancellationException("IMap load cancelled");
  }

  private static ErrorClass errorClass(OperationOutcome outcome) {
    if (outcome.kind() == OutcomeKind.ERROR && outcome.errorClass() == ErrorClass.NONE) {
      throw new IllegalStateException("adapter reported an unclassified error");
    }
    for (var item : outcome.items()) {
      if (item.kind() == OutcomeKind.ERROR) return ErrorClass.PARTIAL;
    }
    return outcome.errorClass();
  }

  private record Sample(long queued, long service, ErrorClass error, long retiredAt) {}

  private static final class Results {
    private final LoadLatencyHistogram endToEnd = new LoadLatencyHistogram();
    private final LoadLatencyHistogram queue = new LoadLatencyHistogram();
    private final LoadLatencyHistogram service = new LoadLatencyHistogram();
    private final Map<ErrorClass, Integer> errors = new EnumMap<>(ErrorClass.class);
    private int inFlight, completed, successful, lateSlots, capacityRejected;

    void collect(ExecutorCompletionService<Sample> completions, long deadline)
        throws InterruptedException, TimeoutException {
      for (var completedRequest = completions.poll(); completedRequest != null;
          completedRequest = completions.poll()) {
        Sample sample;
        try {
          sample = completedRequest.get();
        } catch (ExecutionException failure) {
          throw new IllegalStateException("IMap load invocation failed", failure.getCause());
        }
        if (sample.retiredAt() > deadline) {
          throw new TimeoutException("IMap load invocation exceeded the phase deadline");
        }
        inFlight--;
        completed++;
        endToEnd.record(sample.queued() + sample.service());
        queue.record(sample.queued());
        service.record(sample.service());
        if (sample.error() == ErrorClass.NONE) successful++;
        else errors.merge(sample.error(), 1, Integer::sum);
      }
    }

    Phase finish(int slots, long window, long elapsed) {
      var raw = new Histograms(endToEnd.snapshot(), queue.snapshot(), service.snapshot());
      return new Phase(slots, completed, successful, lateSlots, capacityRejected, window, elapsed,
          errors, raw.endToEnd().quantiles(), raw.queue().quantiles(), raw.service().quantiles(), Optional.of(raw));
    }
  }
}
