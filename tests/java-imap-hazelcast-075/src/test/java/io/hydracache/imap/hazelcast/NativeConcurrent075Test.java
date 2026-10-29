package io.hydracache.imap.hazelcast;

import static org.junit.jupiter.api.Assertions.*;
import static io.hydracache.imap.hazelcast.NativeConcurrentScenario075.*;
import static io.hydracache.imap.hazelcast.NativeConcurrentHistory075.*;
import static io.hydracache.imap.hazelcast.NativeConcurrentOracle075.*;
import static io.hydracache.imap.hazelcast.NativeIMapObservation075.*;
import io.hydracache.imap.semantic.BytesValue;
import java.util.*;
import org.junit.jupiter.api.Test;

final class NativeConcurrent075Test {
  static BytesValue b(String hex) { return BytesValue.fromHex(hex); }
  static Program race() { return new Program(117, List.of(b("61")), List.of(
      new Command(0, 0, 0, Operation.PUT_IF_ABSENT, b("61"), Optional.of(b("31")), Optional.empty()),
      new Command(1, 1, 0, Operation.PUT_IF_ABSENT, b("61"), Optional.of(b("32")), Optional.empty()))); }
  static Snapshot state(String value) { var entries = value == null ? List.<Entry>of() : List.of(new Entry(b("61"), b(value))); return new Snapshot(entries, digestEntries(entries)); }
  static History raceHistory(boolean secondWinner) {
    return History.observed(race(), List.of(new Invoke(1, 0, 0), new Invoke(2, 1, 1), new Return(3, 0, 0, RawResult.value(null)),
        new Return(4, 1, 1, RawResult.value(secondWinner ? null : b("31").copy()))), state(secondWinner ? "32" : "31"));
  }
  @Test void oneWinnerAndPreviousValuesHaveALegalSerializationAndActualOverlap() {
    var history = raceHistory(false); var report = check(history, 1000);
    assertEquals(Verdict.LINEARIZABLE, report.verdict()); assertEquals(List.of(0, 1), report.witness()); assertTrue(history.hasOverlap());
    assertEquals(Verdict.NON_LINEARIZABLE, check(raceHistory(true), 1000).verdict());
    assertEquals(Verdict.EXHAUSTED, check(history, 1).verdict());
    assertThrows(IllegalArgumentException.class, () -> check(history, 0)); assertThrows(IllegalArgumentException.class, () -> check(history, 1_000_001));
  }
  @Test void realtimeEdgesImpossibleGetAndFinalDigestCannotBeIgnored() {
    var program = new Program(1, List.of(b("61")), List.of(
        new Command(0, 0, 0, Operation.GET, b("61"), Optional.empty(), Optional.empty()),
        new Command(1, 1, 0, Operation.PUT, b("61"), Optional.of(b("31")), Optional.empty())));
    var impossible = History.observed(program, List.of(new Invoke(1,0,0), new Return(2,0,0,RawResult.value(b("31").copy())), new Invoke(3,1,1), new Return(4,1,1,RawResult.value(null))), state("31"));
    assertFalse(impossible.hasOverlap()); assertEquals(Verdict.NON_LINEARIZABLE, check(impossible,1000).verdict());
    var raw = List.<Event>of(new Invoke(1,0,0),new Return(2,0,0,RawResult.value(null)),new Invoke(3,1,1),new Return(4,1,1,RawResult.value(null)));
    assertEquals(Verdict.NON_LINEARIZABLE,check(History.observed(program,raw,state("32")),1000).verdict());
    assertEquals(Verdict.LINEARIZABLE,check(History.observed(program,raw,state("31")),1000).verdict());
  }
  @Test void rawNormalizationRejectsMissingDuplicateContradictoryOrSubstitutedCalls() {
    var history = raceHistory(false);
    assertThrows(IllegalArgumentException.class, () -> History.observed(race(), history.raw().subList(0,3), state("31")));
    var duplicate = new ArrayList<>(history.raw()); duplicate.set(1,new Invoke(2,0,0));
    assertThrows(IllegalArgumentException.class, () -> History.observed(race(), duplicate, state("31")));
    var backwards = new ArrayList<>(history.raw()); backwards.set(0,new Return(1,0,0,RawResult.value(null)));
    assertThrows(IllegalArgumentException.class, () -> History.observed(race(), backwards, state("31")));
    var stamp = new ArrayList<>(history.raw()); stamp.set(0,new Invoke(2,0,0));
    assertThrows(IllegalArgumentException.class, () -> History.observed(race(), stamp, state("31")));
    var calls = new ArrayList<>(history.calls()); var call = calls.get(0); calls.set(0,new Call(call.command(),call.invoked(),call.returned(),RawResult.value(b("31").copy())));
    assertThrows(IllegalArgumentException.class, () -> new History(race(),history.raw(),calls,state("31"),history.rawDigest()));
    assertThrows(IllegalArgumentException.class, () -> new History(race(),history.raw(),history.calls(),state("31"),"00"));
    var wrong = new ArrayList<>(history.raw()); wrong.set(3,new Return(4,1,1,RawResult.bool(false)));
    assertThrows(IllegalArgumentException.class, () -> History.observed(race(),wrong,state("31")));
  }
  @Test void sequentialSpecificationPreservesRawBooleanConditionalsAndAllSingleCallFamilies() {
    var state = new TreeMap<BytesValue,BytesValue>(); var key = b("61");
    var replaceAbsent = new Command(0,0,0,Operation.REPLACE_IF_PRESENT,key,Optional.of(b("31")),Optional.empty());
    assertEquals(RawResult.value(null),NativeConcurrentOracle075.apply(state,replaceAbsent));
    var put = new Command(0,0,0,Operation.PUT,key,Optional.of(b("31")),Optional.empty());
    assertEquals(RawResult.value(null),NativeConcurrentOracle075.apply(state,put)); assertEquals(RawResult.value(b("31").copy()),NativeConcurrentOracle075.apply(state,put));
    var mismatch = new Command(0,0,0,Operation.REPLACE_IF_VALUE,key,Optional.of(b("32")),Optional.of(b("33")));
    assertEquals(RawResult.bool(false),NativeConcurrentOracle075.apply(state,mismatch));
    var replace = new Command(0,0,0,Operation.REPLACE_IF_VALUE,key,Optional.of(b("32")),Optional.of(b("31")));
    assertEquals(RawResult.bool(true),NativeConcurrentOracle075.apply(state,replace));
    var remove = new Command(0,0,0,Operation.REMOVE_IF_VALUE,key,Optional.empty(),Optional.of(b("32")));
    assertEquals(RawResult.bool(true),NativeConcurrentOracle075.apply(state,remove)); assertEquals(RawResult.bool(false),NativeConcurrentOracle075.apply(state,remove));
    assertEquals(RawResult.bool(false),NativeConcurrentOracle075.apply(state,new Command(0,0,0,Operation.CONTAINS_KEY,key,Optional.empty(),Optional.empty())));
    assertEquals(seeded(117),seeded(117)); assertNotEquals(seeded(117),seeded(118));
    assertEquals(Set.of(Operation.values()),new HashSet<>(seeded(117).commands().stream().map(Command::operation).toList()));
  }
  static History broken(Program program) {
    var raw = new ArrayList<Event>(); var map = new TreeMap<BytesValue,BytesValue>(); long stamp = 0;
    for (var command : program.commands()) {
      raw.add(new Invoke(++stamp,command.id(),command.worker()));
      if (command.value().isPresent()) map.put(command.key(),command.value().orElseThrow());
      raw.add(new Return(++stamp,command.id(),command.worker(),RawResult.value(null)));
    }
    var entries = map.entrySet().stream().map(e->new Entry(e.getKey(),e.getValue())).toList(); return History.observed(program,raw,new Snapshot(entries,digestEntries(entries)));
  }
  @Test void shrinkingReexecutesCandidatesAndNeverHidesBudgetExhaustion() {
    var commands = new ArrayList<>(race().commands()); commands.add(new Command(2,0,1,Operation.GET,b("61"),Optional.empty(),Optional.empty()));
    commands.add(new Command(3,1,1,Operation.GET,b("61"),Optional.empty(),Optional.empty()));
    var original = broken(new Program(1,race().keys(),commands));
    var shrunk = shrink(original,32,1000,NativeConcurrent075Test::broken); assertFalse(shrunk.exhausted()); assertEquals(2,shrunk.history().calls().size());
    assertTrue(shrink(original,1,1000,NativeConcurrent075Test::broken).exhausted());
    assertThrows(IllegalArgumentException.class,()->shrink(raceHistory(false),1,1000,NativeConcurrent075Test::broken));
    assertThrows(IllegalArgumentException.class,()->shrink(original,0,1000,NativeConcurrent075Test::broken));
    assertThrows(IllegalArgumentException.class,()->shrink(original,1,1000,p->raceHistory(false)));
  }
  @Test void actualTwoOfficialClientsProduceBoundedHistoriesAcrossAllNativeFamilies() throws Exception {
    assertThrows(IllegalArgumentException.class,()->NativeConcurrentProbeMain075.main(new String[]{"foreign"}));
    try(var fixture = new HazelcastLoopback075()) {
      assertThrows(IllegalArgumentException.class,()->NativeConcurrentWorker075.qualified(fixture,seeded(117),0));
      for(long seed:new long[]{117,118}) {
        var run = NativeConcurrentWorker075.qualified(fixture,seeded(seed),8);
        assertEquals(12,run.history().calls().size()); assertTrue(run.history().hasOverlap()); assertNotEquals(run.clients().get(0),run.clients().get(1));
        assertEquals(Verdict.LINEARIZABLE,check(run.history(),100000).verdict()); assertFalse(run.nativeEvents().isEmpty());
        String encoded = NativeConcurrentProbeMain075.encode(run); assertTrue(encoded.contains("\"overlap\":true")); assertTrue(encoded.contains("\"raw_digest\":\""+run.history().rawDigest()));
      }
      assertTrue(fixture.client().getLifecycleService().isRunning());
    }
  }
}
