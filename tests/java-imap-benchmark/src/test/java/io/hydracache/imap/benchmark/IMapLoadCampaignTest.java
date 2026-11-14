package io.hydracache.imap.benchmark;

import static org.junit.jupiter.api.Assertions.*;

import io.hydracache.imap.semantic.AdapterIdentity;
import io.hydracache.imap.semantic.BytesValue;
import io.hydracache.imap.semantic.HydraCacheSemanticAdapter;
import io.hydracache.imap.semantic.InMemorySemanticAdapter;
import io.hydracache.imap.semantic.MapEvent;
import io.hydracache.imap.semantic.MapSemanticAdapter;
import io.hydracache.imap.semantic.Operation;
import io.hydracache.imap.semantic.OperationOutcome;
import io.hydracache.imap.semantic.ScenarioManifest;
import io.hydracache.imap.semantic.ScenarioStep;
import io.hydracache.imap.semantic.StateSnapshot;
import java.util.ArrayList;
import java.util.List;
import java.util.Set;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.stream.IntStream;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.Timeout;

@Timeout(15)
final class IMapLoadCampaignTest {
  static final IMapPairedAnalysis.Policy POLICY = new IMapPairedAnalysis.Policy(117, 1000, 950, 1000, 1500);
  static final IMapLoadCampaign.Bindings BINDINGS = bindings("a");
  static final IMapLoadCampaign.Parallelism SERIAL = new IMapLoadCampaign.Parallelism(1, 1);
  private static final ScenarioStep GET = ScenarioStep.get("get", BytesValue.fromHex("00ff"));
  static final IMapLoadCampaign.Variant READS = new IMapLoadCampaign.Variant("reads", workload(GET));

  @Test void expandsAnImmutableDeterministicMatrixWithBalancedPairs() {
    var variants = new ArrayList<>(List.of(READS, new IMapLoadCampaign.Variant("other", READS.workload())));
    var rates = new ArrayList<>(List.of(20, 40));
    var parallelism = new ArrayList<>(List.of(SERIAL, new IMapLoadCampaign.Parallelism(2, 4)));
    var first = IMapLoadCampaign.plan(variants, rates, parallelism, settings(0, 100), 5, BINDINGS, POLICY);
    var second = IMapLoadCampaign.plan(variants, rates, parallelism, settings(0, 100), 5, BINDINGS, POLICY);
    variants.clear();
    rates.clear();
    parallelism.clear();
    assertEquals(8, first.cells().size());
    assertEquals(40, first.runs().size());
    assertEquals(first.runs(), second.runs());
    assertEquals(40, first.runs().stream().map(run -> run.settings().seed()).distinct().count());
    assertEquals("reads-r20-w1-i1", first.cells().get(0).name());
    assertEquals("other-r40-w2-i4", first.cells().get(7).name());
    for (var cell : first.cells()) {
      assertEquals(List.of(1, 2, 3, 4, 5), cell.runs().stream().map(IMapLoadCampaign.Run::pairNumber).toList());
      for (int i = 1; i < 5; i++) assertNotEquals(cell.runs().get(i - 1).rightFirst(), cell.runs().get(i).rightFirst());
      assertEquals(POLICY, cell.runs().get(0).analysisPolicy());
      assertEquals(BINDINGS, cell.runs().get(0).bindings());
    }
    assertNotEquals(first.cells().get(0).runs().get(0).rightFirst(),
        first.cells().get(1).runs().get(0).rightFirst());
    assertThrows(UnsupportedOperationException.class, () -> first.runs().clear());
  }

  @Test void fingerprintsOrderedWeightsPreloadAndRequestPayload() {
    var other = ScenarioStep.get("other", BytesValue.fromHex("ff"));
    var trace = new ScenarioManifest(117, List.of(GET, other));
    var reference = new IMapLoadWorkload(trace, List.of(), List.of(GET, other));
    assertEquals(reference.fingerprint(), new IMapLoadWorkload(trace, List.of(), List.of(GET, other)).fingerprint());
    for (var changed : List.of(
        new IMapLoadWorkload(trace, List.of(), List.of(other, GET)),
        new IMapLoadWorkload(trace, List.of(), List.of(GET, GET, other)),
        new IMapLoadWorkload(trace, List.of(GET), List.of(GET, other)),
        workload(ScenarioStep.get("get", BytesValue.fromHex("01"))))) {
      assertNotEquals(reference.fingerprint(), changed.fingerprint());
    }
  }

  @Test void rejectsUnboundedAndAmbiguousMatrices() {
    assertThrows(IllegalArgumentException.class, () -> new IMapLoadCampaign.Variant("X/path", READS.workload()));
    assertThrows(IllegalArgumentException.class, () -> bindings("Z"));
    assertThrows(IllegalArgumentException.class, () -> plan(List.of(), List.of(20), List.of(SERIAL), 5));
    assertThrows(IllegalArgumentException.class,
        () -> plan(List.of(READS), IntStream.rangeClosed(1, 65).boxed().toList(), List.of(SERIAL), 5));
    assertThrows(IllegalArgumentException.class, () -> plan(List.of(READS), List.of(20, 20), List.of(SERIAL), 5));
    assertThrows(IllegalArgumentException.class, () -> plan(
        List.of(READS, new IMapLoadCampaign.Variant("reads", workload(ScenarioStep.get("other", BytesValue.fromHex("01"))))),
        List.of(20), List.of(SERIAL), 5));
    assertThrows(IllegalArgumentException.class, () -> plan(List.of(READS), List.of(20), List.of(SERIAL), 4));
    assertThrows(IllegalArgumentException.class, () -> plan(List.of(READS), List.of(20), List.of(SERIAL), 33));
    assertThrows(IllegalArgumentException.class, () -> plan(List.of(READS),
        IntStream.rangeClosed(1, 33).boxed().toList(), List.of(SERIAL, new IMapLoadCampaign.Parallelism(2, 2)), 5));
    assertThrows(IllegalArgumentException.class,
        () -> plan(List.of(READS), List.of(0), List.of(SERIAL), 5));
    assertThrows(IllegalArgumentException.class,
        () -> plan(List.of(READS), List.of(20), List.of(new IMapLoadCampaign.Parallelism(2, 1)), 5));
  }

  @Test void executesActualTrialsInPlannedOrderAndKeepsSideIdentity() {
    var campaign = campaign(0, 1, 5, POLICY);
    var actual = new CopyOnWriteArrayList<String>();
    var attempts = campaign.execute(run -> new TrackingAdapter("left", run.pairNumber(), actual),
        run -> new TrackingAdapter("right", run.pairNumber(), actual), () -> false);
    var expected = new ArrayList<String>();
    for (var run : campaign.runs()) {
      expected.add(run.pairNumber() + (run.rightFirst() ? "right" : "left"));
      expected.add(run.pairNumber() + (run.rightFirst() ? "left" : "right"));
    }
    assertEquals(expected, actual);
    assertEquals(campaign.runs(), attempts.stream().map(IMapLoadCampaign.Attempt::run).toList());
    for (var attempt : attempts) {
      var measured = assertInstanceOf(IMapLoadCampaign.Measured.class, attempt);
      assertEquals("left", measured.result().left().adapter().implementation());
      assertEquals("right", measured.result().right().adapter().implementation());
      assertEquals(measured.run().rightFirst(), measured.result().rightFirst());
    }
    assertTrue(IMapPairedAnalysis.analyze(campaign, attempts).get(0).statistics().isPresent());
  }

  @Test void retainsFailedPairsWithoutRerunningTheLosingSide() {
    var campaign = campaign(0, 1, 5, POLICY);
    var opened = new ArrayList<Integer>();
    var attempts = campaign.execute(run -> {
      opened.add(run.pairNumber());
      if (run.pairNumber() == 1) return new HydraCacheSemanticAdapter();
      if (run.pairNumber() == 2) throw new IllegalStateException("offline".repeat(100));
      return InMemorySemanticAdapter.strict("left");
    }, run -> InMemorySemanticAdapter.strict("right"), () -> false);
    var first = assertInstanceOf(IMapLoadCampaign.Failed.class, attempts.get(0));
    var second = assertInstanceOf(IMapLoadCampaign.Failed.class, attempts.get(1));
    assertEquals(IMapLoadCampaign.Failure.NOT_COMPARABLE, first.failure());
    assertEquals(IMapLoadCampaign.Failure.INFRASTRUCTURE, second.failure());
    assertEquals(512, second.detail().length());
    assertEquals(List.of(1, 2, 3, 3, 4, 4, 5, 5), opened);
    assertThrows(IllegalArgumentException.class,
        () -> new IMapLoadCampaign.Failed(first.run(), first.failure(), "x".repeat(513)));
  }

  @Test void cancelledAndInterruptedCampaignsRetainTheirFinalAttempt() {
    var campaign = campaign(0, 1, 5, POLICY);
    assertCancelled(campaign.execute(run -> fail("left opened"), run -> fail("right opened"), () -> true));
    Thread.currentThread().interrupt();
    try {
      assertCancelled(campaign.execute(run -> fail("left opened"), run -> fail("right opened"), () -> false));
      assertTrue(Thread.currentThread().isInterrupted());
    } finally {
      Thread.interrupted();
    }
  }

  private static void assertCancelled(List<IMapLoadCampaign.Attempt> attempts) {
    assertEquals(1, attempts.size());
    assertEquals(IMapLoadCampaign.Failure.CANCELLED,
        assertInstanceOf(IMapLoadCampaign.Failed.class, attempts.get(0)).failure());
  }

  static IMapLoadCampaign campaign(int warmup, int requests, int pairs, IMapPairedAnalysis.Policy policy) {
    return IMapLoadCampaign.plan(List.of(READS), List.of(2), List.of(SERIAL),
        settings(warmup, requests), pairs, BINDINGS, policy);
  }

  private static FixedRateIMapLoad.Settings settings(int warmup, int requests) {
    return new FixedRateIMapLoad.Settings(117, 2, warmup, requests, 1, 1, 1000);
  }

  static IMapLoadCampaign.Bindings bindings(String digit) {
    String digest = digit.repeat(64);
    return new IMapLoadCampaign.Bindings(digest, digest, digest, digest, digest, digest);
  }

  private static IMapLoadCampaign plan(List<IMapLoadCampaign.Variant> variants, List<Integer> rates,
      List<IMapLoadCampaign.Parallelism> parallelism, int pairs) {
    return IMapLoadCampaign.plan(variants, rates, parallelism, settings(0, 100), pairs, BINDINGS, POLICY);
  }

  private static IMapLoadWorkload workload(ScenarioStep step) {
    return new IMapLoadWorkload(new ScenarioManifest(117, List.of(step)), List.of(), List.of(step));
  }

  private static final class TrackingAdapter implements MapSemanticAdapter {
    private final String side;
    private final int pair;
    private final List<String> actual;
    private final InMemorySemanticAdapter delegate;

    TrackingAdapter(String side, int pair, List<String> actual) {
      this.side = side;
      this.pair = pair;
      this.actual = actual;
      delegate = InMemorySemanticAdapter.strict(side);
    }

    @Override public AdapterIdentity identity() { return delegate.identity(); }
    @Override public Set<Operation> capabilities() { return delegate.capabilities(); }
    @Override public OperationOutcome execute(ScenarioStep step) {
      if (Thread.currentThread().getName().equals("imap-load-worker")) actual.add(pair + side);
      return delegate.execute(step);
    }
    @Override public List<MapEvent> drainEvents() { return delegate.drainEvents(); }
    @Override public StateSnapshot snapshot() { return delegate.snapshot(); }
  }
}
