package io.hydracache.imap.hazelcast;

import static org.junit.jupiter.api.Assertions.*;
import static io.hydracache.imap.hazelcast.NativeIMapObservation075.*;
import static io.hydracache.imap.hazelcast.NativeIMapProfile075.Operation.*;
import io.hydracache.imap.semantic.BytesValue;
import com.hazelcast.map.IMap;
import java.lang.reflect.Proxy;
import java.util.*;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.*;

final class NativeIMapExtended075Test {
  static HazelcastLoopback075 fixture;
  @BeforeAll static void start() throws Exception { fixture = new HazelcastLoopback075(); }
  @AfterAll static void stop() { fixture.close(); }
  static String name() { return "imap075-native-" + UUID.randomUUID(); }
  static BytesValue b(String hex) { return BytesValue.fromHex(hex); }
  static Request batch(List<Entry> entries) { return new Request(PUT_DETAILED, List.of(), entries, Optional.empty(), 64); }
  static Request keys(NativeIMapProfile075.Operation operation, BytesValue... keys) { return new Request(operation, List.of(keys), List.of(), Optional.empty(), 64); }
  static List<NativeEvent> await(NativeIMapAdapter075 adapter, int count) throws Exception {
    var events = new ArrayList<NativeEvent>(); long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5);
    do { events.addAll(adapter.drainEvents()); if (events.size() >= count) return List.copyOf(events); Thread.sleep(2); } while (System.nanoTime() < deadline);
    fail("native callbacks incomplete: " + events.size() + "/" + count); return List.of();
  }
  @SuppressWarnings("unchecked") @Test void aggregateLostResponseAndExpirationNeverMasqueradeAsHealthy() {
    var registration=new java.util.concurrent.atomic.AtomicReference<com.hazelcast.core.EntryListener<byte[],byte[]>>();
    IMap<byte[],byte[]> map=(IMap<byte[],byte[]>)Proxy.newProxyInstance(getClass().getClassLoader(),new Class<?>[]{IMap.class},(proxy,method,args)->switch(method.getName()){
      case "isEmpty"->true;case "iterator"->Collections.emptyIterator();case "addEntryListener"->{registration.set((com.hazelcast.core.EntryListener<byte[],byte[]>)args[0]);yield UUID.randomUUID();}case "removeEntryListener"->true;
      case "putAll","clear"->throw new IllegalStateException("injected lost aggregate response");
      default->throw new AssertionError(method.getName());
    });
    try(var adapter=new NativeIMapAdapter075(map,NativeIMapProfile075.exclusive(),4)) {
      assertEquals(1,adapter.registrationGeneration());
      var failure=assertThrows(NativeIMapAdapter075.UncertainWrite.class,()->adapter.execute(Request.empty(CLEAR)));assertEquals(0,failure.attemptedIndex());assertEquals(Optional.of(NativeIMapAdapter075.Gap.UNCERTAIN_WRITE),adapter.gap());
    }
    try(var adapter=new NativeIMapAdapter075(map,NativeIMapProfile075.exclusive(),4)) {
      registration.get().entryExpired(new com.hazelcast.core.EntryEvent<byte[],byte[]>("fixture",null,com.hazelcast.core.EntryEventType.EXPIRED.getType(),new byte[]{1},new byte[]{2}));
      assertEquals(Optional.of(NativeIMapAdapter075.Gap.PROFILE_VIOLATION),adapter.gap());
    }
    assertThrows(IllegalArgumentException.class,()->new Request(REMOVE_DETAILED,Collections.nCopies(65,b("61")),List.of(),Optional.empty(),64));
  }
  @Test void detailedBinaryHelpersMatchIndependentMapAndActualPerKeyEvents() throws Exception {
    try (var adapter = new NativeIMapAdapter075(fixture.client(), name(), NativeIMapProfile075.exclusive())) {
      var expected = new TreeMap<BytesValue, BytesValue>();
      for (var entries : List.of(List.of(new Entry(b(""), b("")), new Entry(b("00ff"), b("0080ff"))),
          List.of(new Entry(b("00ff"), b("01")), new Entry(b("ff"), b("02"))))) {
        var result = adapter.execute(batch(entries)); var projected = NativeIMapProjection075.detailed(adapter.profile(), result);
        for (int index = 0; index < entries.size(); index++) {
          var entry = entries.get(index); var old = expected.put(entry.key(), entry.value());
          assertEquals(old == null ? "inserted" : "replaced", projected.get(index).kind());
          assertEquals(Optional.ofNullable(old), projected.get(index).previous());
        }
        var events = await(adapter, entries.size()); assertEquals(entries.size(), events.size());
        for (var event : events) assertEquals(expected.get(event.key().orElseThrow()), event.value().orElseThrow());
        assertEquals(digestEntries(expected.entrySet().stream().map(e -> new Entry(e.getKey(), e.getValue())).toList()), adapter.snapshot().digest());
      }
      assertEquals(new NumberReturn(3), adapter.execute(Request.empty(SIZE)).rawReturn());
      assertEquals(new BooleanReturn(false), adapter.execute(Request.empty(IS_EMPTY)).rawReturn());
      assertEquals(new ValueReturn(Optional.of(b("03"))), adapter.execute(new Request(GET_OR_DEFAULT, List.of(b("03")), List.of(), Optional.of(b("03")), 64)).rawReturn());
      assertEquals(new BooleanReturn(true), adapter.execute(new Request(CONTAINS_ENTRY, List.of(b("00ff")), List.of(), Optional.of(b("01")), 64)).rawReturn());
      var views = (ViewsReturn) adapter.execute(keys(ENTRY_VIEWS, b("00ff"), b("03"))).rawReturn(); assertEquals(1, views.entries().size()); assertTrue(views.entries().get(0).nativeVersion() >= 0);
      var scan = adapter.execute(Request.empty(SCAN)); assertTrue(scan.nativeCalls().isEmpty()); assertEquals(3, ((ViewsReturn) scan.rawReturn()).entries().size());
      var removed = NativeIMapProjection075.detailed(adapter.profile(), adapter.execute(keys(REMOVE_DETAILED, b("00ff"), b("03"))));
      assertEquals(List.of("removed", "absent"), removed.stream().map(NativeIMapProjection075.Item::kind).toList()); assertEquals(EventKind.REMOVED, await(adapter, 1).get(0).kind());
    }
  }
  @Test void aggregatesRemainAggregatesAndDoNotPretendDetailedOutcomes() throws Exception {
    try (var adapter = new NativeIMapAdapter075(fixture.client(), name(), NativeIMapProfile075.exclusive())) {
      var aggregate = adapter.execute(new Request(PUT_ALL_DIAGNOSTIC, List.of(), List.of(new Entry(b("61"), b("31")), new Entry(b("62"), b("32"))), Optional.empty(), 64));
      assertEquals(Origin.NATIVE_AGGREGATE, aggregate.origin()); assertEquals(new VoidReturn(), aggregate.rawReturn());
      assertThrows(IllegalArgumentException.class, () -> NativeIMapProjection075.detailed(adapter.profile(), aggregate)); await(adapter, 2);
      assertEquals(new BooleanReturn(true), adapter.execute(keys(EVICT, b("61"))).rawReturn()); assertEquals(EventKind.EVICTED, await(adapter, 1).get(0).kind());
      adapter.execute(Request.empty(CLEAR)); var cleared = await(adapter, 1); assertEquals(EventKind.MAP_CLEARED, cleared.get(0).kind()); assertTrue(cleared.get(0).key().isEmpty()); assertEquals(1, cleared.get(0).affectedEntries());
      adapter.execute(batch(List.of(new Entry(b("61"), b("31"))))); await(adapter, 1);
      adapter.execute(Request.empty(EVICT_ALL)); assertEquals(EventKind.MAP_EVICTED, await(adapter, 1).get(0).kind()); assertEquals(new BooleanReturn(true), adapter.execute(Request.empty(IS_EMPTY)).rawReturn());
    }
  }
  @Test void wholeRequestAndProspectiveCardinalityBoundsPrecedeFirstWrite() throws Exception {
    try (var adapter = new NativeIMapAdapter075(fixture.client(), name(), NativeIMapProfile075.exclusive())) {
      var initial = adapter.snapshot();
      assertThrows(IllegalArgumentException.class, () -> batch(List.of()));
      assertThrows(IllegalArgumentException.class, () -> batch(List.of(new Entry(b("61"), b("31")), new Entry(b("61"), b("32")))));
      assertThrows(IllegalArgumentException.class, () -> batch(List.of(new Entry(b("61"), BytesValue.copyOf(new byte[65537])))));
      assertThrows(IllegalArgumentException.class, () -> keys(REMOVE_DETAILED, BytesValue.copyOf(new byte[1025])));
      assertThrows(IllegalArgumentException.class, () -> new Request(SIZE, List.of(b("61")), List.of(), Optional.empty(), 64));
      assertThrows(IllegalArgumentException.class, () -> new Request(SCAN, List.of(), List.of(), Optional.empty(), 65));
      assertEquals(initial, adapter.snapshot());
      var entries = new ArrayList<Entry>(); for (int i = 0; i < 64; i++) entries.add(new Entry(BytesValue.copyOf(new byte[]{(byte)i}), b("31")));
      adapter.execute(batch(entries)); await(adapter, 64); var full = adapter.snapshot();
      assertThrows(IllegalArgumentException.class, () -> adapter.execute(batch(List.of(new Entry(b("00"), b("32")), new Entry(b("ff"), b("32"))))));
      assertEquals(full, adapter.snapshot()); assertThrows(IllegalArgumentException.class, () -> adapter.execute(new Request(SCAN, List.of(), List.of(), Optional.empty(), 63)));
    }
  }
  @Test void closedProfilesRejectWeakenedFieldsAndProjectionDetectsMissingDuplicateEvents() {
    assertThrows(IllegalArgumentException.class, () -> new NativeIMapProfile075(2, Set.of(NativeIMapProfile075.Field.values())));
    assertThrows(IllegalArgumentException.class, () -> new NativeIMapProfile075(1, Set.of(NativeIMapProfile075.Field.RAW_RETURN)));
    assertThrows(IllegalArgumentException.class, () -> NativeIMapProfile075.Operation.valueOf("REPLACE_IF_VERSION"));
    var a = new NativeEvent(1, EventKind.ADDED, Optional.of(b("61")), Optional.empty(), Optional.of(b("31")), 1);
    var b = new NativeEvent(1, EventKind.ADDED, Optional.of(b("62")), Optional.empty(), Optional.of(b("32")), 1);
    assertTrue(NativeIMapProjection075.samePerKeyEvents(List.of(a, b), List.of(b, a)));
    assertFalse(NativeIMapProjection075.samePerKeyEvents(List.of(a, b), List.of(a, a)));
    assertFalse(NativeIMapProjection075.samePerKeyEvents(List.of(a), List.of()));
    assertThrows(IllegalArgumentException.class, () -> new Snapshot(List.of(new Entry(b("61"), b("31"))), "00"));
    var names = Arrays.stream(NativeEvent.class.getRecordComponents()).map(java.lang.reflect.RecordComponent::getName).toList();
    assertFalse(names.contains("watermark")); assertFalse(names.contains("owner")); assertFalse(names.contains("token"));
  }
  @SuppressWarnings("unchecked") @Test void partialTransportFailureNeverLabelsUnattemptedItemsNotApplied() {
    var writes = new AtomicInteger(); var stored = new HashMap<BytesValue, BytesValue>();
    IMap<byte[], byte[]> raw = (IMap<byte[], byte[]>) Proxy.newProxyInstance(getClass().getClassLoader(), new Class<?>[]{IMap.class}, (proxy, method, args) -> switch (method.getName()) {
      case "isEmpty" -> true; case "iterator" -> Collections.emptyIterator(); case "addEntryListener" -> UUID.randomUUID(); case "removeEntryListener" -> true;
      case "put" -> { if (writes.incrementAndGet() == 2) throw new IllegalStateException("injected lost response"); yield stored.put(BytesValue.copyOf((byte[])args[0]), BytesValue.copyOf((byte[])args[1])) == null ? null : new byte[]{1}; }
      default -> throw new AssertionError("unexpected map call: " + method.getName());
    });
    try (var adapter = new NativeIMapAdapter075(raw, NativeIMapProfile075.exclusive(), 64)) {
      var failure = assertThrows(NativeIMapAdapter075.UncertainWrite.class, () -> adapter.execute(batch(List.of(new Entry(b("61"), b("31")), new Entry(b("62"), b("32")), new Entry(b("63"), b("33"))))));
      assertEquals(1, failure.completed().size()); assertEquals(1, failure.attemptedIndex()); assertEquals(2, writes.get()); assertEquals(1, stored.size());
      assertEquals(Optional.of(NativeIMapAdapter075.Gap.UNCERTAIN_WRITE), adapter.gap()); assertThrows(IllegalStateException.class, adapter::drainEvents);
    }
  }
  @Test void nativeProfileViolationAndCloseAreExplicitAndDoNotOwnClientLifetime() {
    String name = name(); var adapter = new NativeIMapAdapter075(fixture.client(), name, NativeIMapProfile075.exclusive());
    fixture.client().<byte[], byte[]>getMap(name).put(new byte[]{1}, new byte[]{2}, 3600, TimeUnit.SECONDS);
    assertThrows(IllegalStateException.class, adapter::snapshot); assertEquals(Optional.of(NativeIMapAdapter075.Gap.PROFILE_VIOLATION), adapter.gap());
    adapter.close(); adapter.close(); assertThrows(IllegalStateException.class, adapter::snapshot); assertThrows(IllegalStateException.class, adapter::drainEvents);
    assertTrue(fixture.client().getLifecycleService().isRunning());
  }
}
