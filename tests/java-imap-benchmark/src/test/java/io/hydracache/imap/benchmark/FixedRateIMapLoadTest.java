package io.hydracache.imap.benchmark;

import static io.hydracache.imap.benchmark.IMapLoadWorkloadTest.trace;
import static io.hydracache.imap.benchmark.IMapLoadWorkloadTest.write;
import static org.junit.jupiter.api.Assertions.*;

import io.hydracache.imap.semantic.AdapterIdentity;
import io.hydracache.imap.semantic.AdapterUnavailableException;
import io.hydracache.imap.semantic.BytesValue;
import io.hydracache.imap.semantic.ErrorClass;
import io.hydracache.imap.semantic.HydraCacheSemanticAdapter;
import io.hydracache.imap.semantic.InMemorySemanticAdapter;
import io.hydracache.imap.semantic.ItemOutcome;
import io.hydracache.imap.semantic.MapEvent;
import io.hydracache.imap.semantic.MapSemanticAdapter;
import io.hydracache.imap.semantic.Operation;
import io.hydracache.imap.semantic.OperationOutcome;
import io.hydracache.imap.semantic.OutcomeKind;
import io.hydracache.imap.semantic.ScenarioStep;
import io.hydracache.imap.semantic.SemanticMismatchException;
import io.hydracache.imap.semantic.StateSnapshot;
import io.hydracache.imap.semantic.TtlState;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.SplittableRandom;
import java.util.concurrent.CancellationException;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.Executor;
import java.util.concurrent.Semaphore;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.function.Function;
import java.util.function.Supplier;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.Timeout;

@Timeout(10)
final class FixedRateIMapLoadTest {
  private static final long MS = 1_000_000;
  private static final BytesValue KEY = BytesValue.fromHex("00ff");
  private static final BytesValue VALUE = BytesValue.fromHex("ff00");
  private static final ScenarioStep GET = ScenarioStep.get("get", KEY);
  private static final ScenarioStep PUT = write("put", KEY, VALUE, Operation.PUT, Optional.empty());
  private static final OperationOutcome PRESENT = OperationOutcome.point(
      OutcomeKind.PRESENT, Optional.of(VALUE), TtlState.eternal());

  @Test void validatesSettingsAndUsesRationalScheduleWithoutDrift() {
    assertThrows(IllegalArgumentException.class, () -> new FixedRateIMapLoad.Settings(1, 0, 0, 1, 1, 1, 1));
    assertThrows(IllegalArgumentException.class, () -> new FixedRateIMapLoad.Settings(1, 1_000_001, 0, 1, 1, 1, 1));
    assertThrows(IllegalArgumentException.class, () -> new FixedRateIMapLoad.Settings(1, 1, -1, 1, 1, 1, 1));
    assertThrows(IllegalArgumentException.class, () -> new FixedRateIMapLoad.Settings(1, 1, 0, 0, 1, 1, 1));
    assertThrows(IllegalArgumentException.class, () -> new FixedRateIMapLoad.Settings(1, 1, 0, 1, 129, 129, 1));
    assertThrows(IllegalArgumentException.class, () -> new FixedRateIMapLoad.Settings(1, 1, 0, 1, 2, 1, 1));
    assertThrows(IllegalArgumentException.class, () -> new FixedRateIMapLoad.Settings(1, 1, 0, 1, 1, 1, 60_001));
    assertThrows(IllegalArgumentException.class, () -> new FixedRateIMapLoad.Settings(1, 1, 601, 1, 1, 1, 1));
    var settings = new FixedRateIMapLoad.Settings(117, 3, 0, 3, 1, 1, 50);
    assertEquals(List.of(0L, 333_333_333L, 666_666_666L, 1_000_000_000L),
        List.of(settings.offset(0), settings.offset(1), settings.offset(2), settings.offset(3)));
  }

  @Test void scheduledQueueAndServiceLatencyUseSeparateClocks() throws Exception {
    var clock = new ManualClock();
    var starts = new ArrayList<Long>();
    var adapter = adapter(step -> {
      starts.add(clock.now);
      clock.now += 200;
      return PRESENT;
    });
    var phase = runPhase(3, clock, adapter, Runnable::run, List.of(GET));
    assertEquals(List.of(0L, MS, 2 * MS), starts);
    assertEquals(3, phase.completed());
    assertEquals(3, phase.successful());
    assertEquals(3 * MS, phase.offeredWindowNanos());
    assertEquals(2 * MS + 200, phase.elapsedNanos());
    assertEquals(new FixedRateIMapLoad.Latencies(0, 0, 0, 0), phase.queue().orElseThrow());
    var expected = new FixedRateIMapLoad.Latencies(200, 200, 200, 200);
    assertEquals(expected, phase.service().orElseThrow());
    assertEquals(expected, phase.endToEnd().orElseThrow());
    assertEquals(Map.of(), phase.errors());
  }

  @Test void overloadDropsSlotsAndPreservesSeededChoiceForTheSurvivingSlots() throws Exception {
    var clock = new ManualClock();
    var offered = new ArrayList<ScenarioStep>();
    var requests = List.of(GET, PUT, ScenarioStep.get("another", BytesValue.fromHex("01")));
    var held = new ArrayList<Runnable>();
    clock.afterPause = () -> {
      if (clock.now >= 3 * MS) {
        held.forEach(Runnable::run);
        held.clear();
      }
    };
    Executor executor = work -> {
      if (clock.now < 3 * MS) held.add(work);
      else work.run();
    };
    var adapter = adapter(step -> {
      offered.add(step);
      return PRESENT;
    });
    var phase = runPhase(4, clock, adapter, executor, requests);
    assertEquals(4, phase.scheduled());
    assertEquals(2, phase.capacityRejected());
    assertEquals(0, phase.lateSlots());
    assertEquals(2, phase.completed());
    var random = new SplittableRandom(117);
    var all = java.util.stream.IntStream.range(0, 4)
        .mapToObj(slot -> requests.get(random.nextInt(requests.size()))).toList();
    assertEquals(List.of(all.get(0), all.get(3)), offered);
    assertTrue(phase.endToEnd().orElseThrow().p99() >= 3 * MS);
    assertEquals(phase.endToEnd(), phase.queue());
    assertEquals(0, phase.service().orElseThrow().p99());
  }

  @Test void schedulerDoesNotCatchUpWithABurstAfterOverrun() throws Exception {
    var clock = new ManualClock();
    var starts = new ArrayList<Long>();
    var adapter = adapter(step -> {
      starts.add(clock.now);
      if (starts.size() == 1) clock.now += 5 * MS;
      return PRESENT;
    });
    var phase = runPhase(6, clock, adapter, Runnable::run, List.of(GET));
    assertEquals(List.of(0L, 5 * MS), starts);
    assertEquals(4, phase.lateSlots());
    assertEquals(0, phase.capacityRejected());
    assertEquals(2, phase.completed());
  }

  @Test void emptyPhaseHasNoLatencyAndDrainHasABoundedDeadline() throws Exception {
    var clock = new ManualClock();
    var adapter = adapter(step -> fail("no request should reach the adapter"));
    var empty = runPhase(0, clock, adapter, Runnable::run, List.of(GET));
    assertEquals(Optional.empty(), empty.endToEnd());
    assertEquals(Optional.empty(), empty.queue());
    assertEquals(Optional.empty(), empty.service());
    assertEquals(0, empty.offeredWindowNanos());
    var error = assertThrows(TimeoutException.class,
        () -> runPhase(1, clock, adapter, task -> {}, List.of(GET)));
    assertTrue(error.getMessage().contains("did not drain"));
    assertEquals(3 * MS, clock.now);
  }

  @Test void lateCompletionCannotPassDrainJustBecauseItIsAlreadyQueued() {
    var clock = new ManualClock();
    var adapter = adapter(step -> {
      clock.now += 4 * MS;
      return PRESENT;
    });
    var error = assertThrows(TimeoutException.class,
        () -> runPhase(1, clock, adapter, Runnable::run, List.of(GET)));
    assertTrue(error.getMessage().contains("exceeded the phase deadline"));
  }

  @Test void classifiesPointAndPartialBulkErrorsAndRefusesMalformedOutcomes() throws Exception {
    var clock = new ManualClock();
    var ordinal = new AtomicInteger();
    var adapter = adapter(step -> switch (ordinal.getAndIncrement()) {
      case 0 -> OperationOutcome.error(ErrorClass.AUTHORIZATION, "denied");
      case 1 -> OperationOutcome.bulk(List.of(new ItemOutcome(
          0, KEY, OutcomeKind.ERROR, Optional.empty(), TtlState.absent())));
      default -> OperationOutcome.bulk(List.of(new ItemOutcome(
          0, KEY, OutcomeKind.PRESENT, Optional.of(VALUE), TtlState.eternal())));
    });
    var phase = runPhase(3, clock, adapter, Runnable::run, List.of(GET));
    assertEquals(3, phase.completed());
    assertEquals(1, phase.successful());
    assertEquals(Map.of(ErrorClass.AUTHORIZATION, 1, ErrorClass.PARTIAL, 1), phase.errors());
    var malformed = assertThrows(IllegalStateException.class,
        () -> runPhase(1, new ManualClock(), adapter(step -> OperationOutcome.error(ErrorClass.NONE, "")),
            Runnable::run, List.of(GET)));
    assertEquals("adapter reported an unclassified error", malformed.getCause().getMessage());
    var transport = new IllegalArgumentException("transport failed");
    assertSame(transport, assertThrows(IllegalStateException.class,
        () -> runPhase(1, new ManualClock(), adapter(step -> { throw transport; }),
            Runnable::run, List.of(GET))).getCause());
  }

  @Test void sharedEntryPointUsesFourFreshAdaptersAndPreloadsBeforeWarmup() throws Exception {
    var opened = new ArrayList<TestAdapter>();
    var result = new FixedRateIMapLoad().run(new FixedRateIMapLoad.Settings(117, 20, 2, 2, 2, 2, 1000),
        new IMapLoadWorkload(trace(PUT, GET), List.of(PUT), List.of(GET)),
        trackingFactory("left", opened), trackingFactory("right", opened), () -> false);
    assertTrue(result.admission().equivalent());
    assertEquals(1, result.preloaded().liveCardinality());
    assertEquals(4, opened.size());
    assertTrue(opened.stream().allMatch(adapter -> adapter.closed));
    assertEquals("left", result.left().adapter().implementation());
    assertEquals("right", result.right().adapter().implementation());
    for (var trial : List.of(result.left(), result.right())) {
      assertEquals(2, trial.warmup().scheduled());
      assertEquals(2, trial.measured().scheduled());
      assertEquals(2, trial.measured().completed() + trial.measured().lateSlots()
          + trial.measured().capacityRejected());
      assertEquals(Map.of(), trial.measured().errors());
    }
    assertTrue(opened.get(2).workerCalls.get() > 0);
    assertTrue(opened.get(3).workerCalls.get() > 0);
  }

  @Test void semanticDivergenceAndBlockedProductStopBeforeTimedFactories() {
    var getAndPut = write("get-and-put", KEY, VALUE, Operation.GET_AND_PUT, Optional.empty());
    var opened = new ArrayList<TestAdapter>();
    var workload = new IMapLoadWorkload(trace(getAndPut), List.of(), List.of(getAndPut));
    assertThrows(SemanticMismatchException.class, () -> new FixedRateIMapLoad().run(
        slowSettings(), workload, trackingFactory("left", opened),
        () -> InMemorySemanticAdapter.withDefect("broken", InMemorySemanticAdapter.Defect.DROP_GET_AND_PUT),
        () -> false));
    assertEquals(1, opened.size());
    assertTrue(opened.get(0).closed);
    assertThrows(AdapterUnavailableException.class, () -> new FixedRateIMapLoad().run(
        slowSettings(), getWorkload(), HydraCacheSemanticAdapter::new,
        () -> InMemorySemanticAdapter.strict("reference"), () -> false));
  }

  @Test void rejectsAdapterReuseAndNonemptyOrUnquietStarts() {
    var shared = new TestAdapter("shared");
    assertThrows(SemanticMismatchException.class, () -> run(() -> shared, () -> shared));
    assertTrue(shared.closed);
    assertThrows(SemanticMismatchException.class, () -> run(() -> {
      var adapter = new TestAdapter("prepopulated");
      adapter.execute(PUT);
      return adapter;
    }, () -> new TestAdapter("right")));
    assertThrows(SemanticMismatchException.class, () -> run(() -> new TestAdapter("pending") {
      @Override public List<MapEvent> drainEvents() {
        return List.of(new MapEvent(MapEvent.Kind.ADDED, KEY, Optional.of(VALUE), 0));
      }
    }, () -> new TestAdapter("right")));
  }

  @Test void rejectsChangedTimedIdentityAndCapabilityBeforeRunning() {
    var different = afterPreflight("same", () -> new TestAdapter("changed"));
    var missing = afterPreflight("same", () -> new TestAdapter("same") {
      @Override public Set<Operation> capabilities() { return Set.of(); }
    });
    for (var supplier : List.of(different, missing)) {
      assertThrows(SemanticMismatchException.class, () -> run(supplier, () -> new TestAdapter("right")));
    }
  }

  @Test void rejectsPreloadErrorsAndDifferentPreloadedState() {
    var workload = new IMapLoadWorkload(trace(PUT, GET), List.of(PUT), List.of(GET));
    var refused = afterPreflight("same", () -> new TestAdapter("same") {
      @Override public OperationOutcome execute(ScenarioStep step) {
        return OperationOutcome.error(ErrorClass.SIZE, "preload refused");
      }
    });
    var dropped = afterPreflight("same", () -> new TestAdapter("same") {
      @Override public OperationOutcome execute(ScenarioStep step) { return PRESENT; }
    });
    for (var supplier : List.of(refused, dropped)) {
      assertThrows(SemanticMismatchException.class, () -> new FixedRateIMapLoad().run(
          slowSettings(), workload, supplier, () -> new TestAdapter("right"), () -> false));
    }
  }

  @Test void cancellationAndInterruptPreventAdmission() {
    var factory = (Supplier<MapSemanticAdapter>) () -> fail("no adapter should open");
    assertThrows(CancellationException.class, () -> new FixedRateIMapLoad().run(
        slowSettings(), getWorkload(), factory, factory, () -> true));
    Thread.currentThread().interrupt();
    assertThrows(InterruptedException.class, () -> new FixedRateIMapLoad().run(
        slowSettings(), getWorkload(), factory, factory, () -> false));
  }

  @Test void cancellationInterruptsRunningWorkerAndClosesAdapters() {
    var cancel = new AtomicBoolean();
    var interrupted = new AtomicBoolean();
    var gate = new CountDownLatch(1);
    var worker = new TestAdapter("left") {
      @Override public OperationOutcome execute(ScenarioStep step) {
        cancel.set(true);
        try {
          gate.await();
        } catch (InterruptedException failure) {
          interrupted.set(true);
        }
        return PRESENT;
      }
    };
    try {
      assertThrows(CancellationException.class, () -> new FixedRateIMapLoad().run(
          slowSettings(), getWorkload(), afterPreflight("left", () -> worker),
          () -> new TestAdapter("right"), cancel::get));
      assertTrue(interrupted.get());
      assertTrue(worker.closed);
    } finally {
      gate.countDown();
    }
  }

  @Test void cancellationRefusesToPublishIfWorkerIgnoresInterruption() throws Exception {
    var cancel = new AtomicBoolean();
    var gate = new Semaphore(0);
    var finished = new CountDownLatch(1);
    var worker = new TestAdapter("left") {
      @Override public OperationOutcome execute(ScenarioStep step) {
        cancel.set(true);
        try {
          gate.acquireUninterruptibly();
          return PRESENT;
        } finally {
          finished.countDown();
        }
      }
    };
    try {
      var error = assertThrows(TimeoutException.class, () -> new FixedRateIMapLoad().run(
          new FixedRateIMapLoad.Settings(117, 1, 0, 1, 1, 1, 30), getWorkload(),
          afterPreflight("left", () -> worker), () -> new TestAdapter("right"), cancel::get));
      assertTrue(error.getMessage().contains("workers did not terminate"));
    } finally {
      gate.release();
      assertTrue(finished.await(2, TimeUnit.SECONDS));
    }
  }

  @Test void quantilesUseNearestRankAndReportSafeBucketUpperBounds() {
    var histogram = new LoadLatencyHistogram();
    assertTrue(histogram.snapshot().quantiles().isEmpty());
    for (int i = 0; i < 95; i++) histogram.record(0);
    for (int i = 0; i < 4; i++) histogram.record(256);
    histogram.record(Long.MAX_VALUE);
    assertEquals(new FixedRateIMapLoad.Latencies(0, 0, 257, Long.MAX_VALUE),
        histogram.snapshot().quantiles().orElseThrow());
    assertEquals(List.of(new FixedRateIMapLoad.Bucket(0, 95), new FixedRateIMapLoad.Bucket(257, 4),
        new FixedRateIMapLoad.Bucket(Long.MAX_VALUE, 1)), histogram.snapshot().buckets());
    assertThrows(IllegalArgumentException.class, () -> new FixedRateIMapLoad.Histogram(1, List.of()));
    assertThrows(IllegalArgumentException.class,
        () -> new FixedRateIMapLoad.Histogram(0, List.of(new FixedRateIMapLoad.Bucket(0, 0))));
    assertThrows(IllegalArgumentException.class, () -> new FixedRateIMapLoad.Histogram(2,
        List.of(new FixedRateIMapLoad.Bucket(1, 1), new FixedRateIMapLoad.Bucket(1, 1))));
    var tooMany = java.util.stream.IntStream.rangeClosed(0, 57 * 128)
        .mapToObj(index -> new FixedRateIMapLoad.Bucket(index, 1)).toList();
    assertThrows(IllegalArgumentException.class, () -> new FixedRateIMapLoad.Histogram(tooMany.size(), tooMany));
  }

  private static FixedRateIMapLoad.Phase runPhase(int slots, ManualClock clock,
      MapSemanticAdapter adapter, Executor executor, List<ScenarioStep> requests)
      throws InterruptedException, TimeoutException {
    return FixedRateIMapLoad.runPhase(new FixedRateIMapLoad.Settings(117, 1000, 0, 6, 1, 1, 2),
        slots, requests, adapter, () -> false, executor, clock, new SplittableRandom(117));
  }

  private static FixedRateIMapLoad.Pair run(Supplier<MapSemanticAdapter> left,
      Supplier<MapSemanticAdapter> right) throws InterruptedException, TimeoutException {
    return new FixedRateIMapLoad().run(slowSettings(), getWorkload(), left, right, () -> false);
  }

  private static IMapLoadWorkload getWorkload() {
    return new IMapLoadWorkload(trace(GET), List.of(), List.of(GET));
  }

  private static FixedRateIMapLoad.Settings slowSettings() {
    return new FixedRateIMapLoad.Settings(117, 1, 0, 1, 1, 1, 1000);
  }

  private static Supplier<MapSemanticAdapter> afterPreflight(String name,
      Supplier<MapSemanticAdapter> timed) {
    var count = new AtomicInteger();
    return () -> count.getAndIncrement() == 0 ? new TestAdapter(name) : timed.get();
  }

  private static Supplier<MapSemanticAdapter> trackingFactory(String name, List<TestAdapter> opened) {
    return () -> {
      var adapter = new TestAdapter(name);
      opened.add(adapter);
      return adapter;
    };
  }

  private static TestAdapter adapter(Function<ScenarioStep, OperationOutcome> work) {
    return new TestAdapter("test") {
      @Override public OperationOutcome execute(ScenarioStep step) { return work.apply(step); }
    };
  }

  private static final class ManualClock implements FixedRateIMapLoad.Clock {
    long now;
    Runnable afterPause = () -> {};

    @Override public long now() { return now; }

    @Override public void pause(long nanos) {
      now += nanos;
      afterPause.run();
    }
  }

  private static class TestAdapter implements MapSemanticAdapter {
    final InMemorySemanticAdapter delegate;
    final AtomicInteger workerCalls = new AtomicInteger();
    boolean closed;

    TestAdapter(String name) { delegate = InMemorySemanticAdapter.strict(name); }

    @Override public AdapterIdentity identity() { return delegate.identity(); }
    @Override public Set<Operation> capabilities() { return delegate.capabilities(); }
    @Override public OperationOutcome execute(ScenarioStep step) {
      if (Thread.currentThread().getName().equals("imap-load-worker")) workerCalls.incrementAndGet();
      return delegate.execute(step);
    }
    @Override public List<MapEvent> drainEvents() { return delegate.drainEvents(); }
    @Override public StateSnapshot snapshot() { return delegate.snapshot(); }
    @Override public void close() { closed = true; }
  }
}
