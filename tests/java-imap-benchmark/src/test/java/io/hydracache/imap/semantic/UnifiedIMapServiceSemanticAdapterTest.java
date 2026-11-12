package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.*;
import java.io.StringReader;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.function.Consumer;
import org.junit.jupiter.api.Test;

final class UnifiedIMapServiceSemanticAdapterTest {
  private static BytesValue b(String value) { return BytesValue.fromHex(value); }
  private static UnifiedIMapService owner() { return new UnifiedIMapService(new BoundedIMapApi.Bounds(128, 128, 256, 256, 256, 4096, 256), 1); }
  private static ScenarioStep write(String id, Operation operation, String key, String value, TtlDirective ttl) { return new ScenarioStep(id, operation, List.of(), List.of(new ScenarioEntry(b(key), b(value))), Optional.empty(), ttl, 0); }
  private static ScenarioStep read(String id, Operation operation, String key) { return new ScenarioStep(id, operation, List.of(b(key)), List.of(), Optional.empty(), TtlDirective.preserve(), 0); }
  private static ScenarioManifest manifest(String text) throws Exception { return ScenarioManifestLoader.load(new StringReader("format=imap-semantic-v1\nseed=117\n" + text), ManifestLimits.defaults()); }

  @Test void sharedSeededManifestsUseActualOwner() throws Exception {
    var scenario = manifest("""
        step=a contains_key key=61
        step=b put key=61 value=31 ttl=eternal
        step=c put_if_absent key=61 value=32 ttl=preserve
        step=d replace key=61 expected=30 value=33 ttl=preserve
        step=e replace key=61 expected=31 value=33 ttl=expire_after:4
        step=f replace_if_present key=61 value=34 ttl=preserve
        step=g get_and_put key=62 value=35 ttl=eternal
        step=h remove_if_value key=62 expected=30
        step=i set_ttl key=61 ttl=expire_after:2
        step=j remaining_ttl key=61
        step=k put_all entries=63:36,64:37 ttl=expire_after:2
        step=l get_all keys=61,62,65
        step=m listener_gap key=61
        step=n advance ticks=2
        step=o get key=61
        step=p get_and_remove key=62
        step=q remove_all keys=61,62,63,64
        """);
    try (var actual = UnifiedIMapServiceSemanticAdapter.fresh("all-operations")) {
      assertEquals(Set.of(Operation.values()), actual.capabilities()); assertEquals("local-reference", actual.identity().product());
      new SemanticOracle().compare(scenario, InMemorySemanticAdapter.strict("oracle"), actual).requireEquivalent();
    }
    for (long seed : List.of(0L, 1L, 117L, 118L, Long.MAX_VALUE)) {
      try (var actual = UnifiedIMapServiceSemanticAdapter.fresh("seed-" + seed)) {
        new SemanticOracle().compare(SeededSemanticScenario.generate(seed, 180), InMemorySemanticAdapter.strict("oracle"), actual).requireEquivalent();
      }
    }
    var service = owner();
    try (var actual = new UnifiedIMapServiceSemanticAdapter("alias", service)) {
      actual.execute(write("put", Operation.PUT, "61", "31", TtlDirective.eternal()));
      assertEquals(b("31"), service.diagnosticSnapshot(0).get(0).value());
      service.execute(new UnifiedIMapService.DomainCommand("set").key(b("61")).value(b("32")).identity(new BoundedIMapApi.MutationIdentity("external", 1)));
      assertEquals(Optional.of(b("32")), actual.execute(ScenarioStep.get("get", b("61"))).value());
      assertEquals(2, actual.drainEvents().size());
      new ConcurrentSemanticOracle().putIfAbsentRace(actual, b("62"), b("33"), b("34"));
    }
    assertEquals(0, service.diagnosticStats(0).listeners()); assertEquals(2, service.diagnosticSnapshot(0).size());
  }

  @Test void retainedBorrowedMapExpectationsUseUnifiedOwner() throws Exception {
    Map<String, Consumer<MapSemanticAdapter>> cases = Map.of(
        "hz-map-put-get", adapter -> { adapter.execute(write("p", Operation.PUT, "61", "31", TtlDirective.eternal())); assertEquals(Optional.of(b("31")), adapter.execute(ScenarioStep.get("g", b("61"))).value()); },
        "hz-map-remove", adapter -> { adapter.execute(write("p", Operation.PUT, "61", "31", TtlDirective.eternal())); assertEquals(OutcomeKind.REMOVED, adapter.execute(read("r", Operation.GET_AND_REMOVE, "61")).kind()); assertEquals(OutcomeKind.ABSENT, adapter.execute(ScenarioStep.get("g", b("61"))).kind()); },
        "hz-map-remove-if-same", adapter -> { adapter.execute(write("p", Operation.PUT, "61", "31", TtlDirective.eternal())); assertEquals(OutcomeKind.MISMATCH, adapter.execute(new ScenarioStep("m", Operation.REMOVE_IF_VALUE, List.of(b("61")), List.of(), Optional.of(b("32")), TtlDirective.preserve(), 0)).kind()); assertEquals(OutcomeKind.REMOVED, adapter.execute(new ScenarioStep("r", Operation.REMOVE_IF_VALUE, List.of(b("61")), List.of(), Optional.of(b("31")), TtlDirective.preserve(), 0)).kind()); },
        "hz-map-replace", adapter -> { adapter.execute(write("p", Operation.PUT, "61", "31", TtlDirective.eternal())); assertEquals(OutcomeKind.REPLACED, adapter.execute(write("r", Operation.REPLACE_IF_PRESENT, "61", "32", TtlDirective.preserve())).kind()); assertEquals(Optional.of(b("32")), adapter.execute(ScenarioStep.get("g", b("61"))).value()); },
        "hz-map-replace-if-same", adapter -> { adapter.execute(write("p", Operation.PUT, "61", "31", TtlDirective.eternal())); assertEquals(OutcomeKind.MISMATCH, adapter.execute(new ScenarioStep("r", Operation.REPLACE, List.of(), List.of(new ScenarioEntry(b("61"), b("32"))), Optional.of(b("30")), TtlDirective.preserve(), 0)).kind()); },
        "hz-map-listener", adapter -> { adapter.execute(write("p", Operation.PUT, "61", "31", TtlDirective.eternal())); assertEquals(b("61"), adapter.drainEvents().get(0).key()); },
        "hz-map-listener-value", adapter -> { adapter.execute(write("p", Operation.PUT, "61", "31", TtlDirective.eternal())); assertEquals(Optional.of(b("31")), adapter.drainEvents().get(0).value()); },
        "hz-map-ttl", adapter -> { adapter.execute(write("p", Operation.PUT, "61", "31", TtlDirective.expireAfter(2))); adapter.execute(new ScenarioStep("a", Operation.ADVANCE, List.of(), List.of(), Optional.empty(), TtlDirective.preserve(), 2)); assertEquals(OutcomeKind.ABSENT, adapter.execute(ScenarioStep.get("g", b("61"))).kind()); });
    Path root = Path.of("").toAbsolutePath(); while (root != null && !Files.isRegularFile(root.resolve("docs/integrations/hazelcast_borrowed_suite.json"))) root = root.getParent();
    assertNotNull(root); String corpus = Files.readString(root.resolve("docs/integrations/hazelcast_borrowed_suite.json"));
    assertTrue(corpus.contains("a9ce2a02ac17f88fcd38869ac698e56e613dc40c"));
    var ids = new HashSet<String>(); var matcher = java.util.regex.Pattern.compile("\\\"id\\\":\\\"(hz-map-[^\\\"]+)\\\"").matcher(corpus);
    while (matcher.find()) ids.add(matcher.group(1)); assertEquals(ids, cases.keySet());
    for (var entry : cases.entrySet()) try (var actual = UnifiedIMapServiceSemanticAdapter.fresh(entry.getKey())) { entry.getValue().accept(actual); }
  }

  @Test void invalidPreserveAndDuplicateBulkNeverChangeStateOrEmit() throws Exception {
    var scenario = manifest("""
        step=a put key=61 value=31 ttl=preserve
        step=b put_if_absent key=61 value=31 ttl=preserve
        step=c get_and_put key=61 value=31 ttl=preserve
        step=d put_all entries=61:31,62:32 ttl=preserve
        step=e put key=61 value=31 ttl=eternal
        step=f put_all entries=61:32,62:33 ttl=preserve
        step=g put_all entries=61:32,61:33 ttl=eternal
        step=h get_all keys=61,61
        step=i remove_all keys=61,61
        step=j set_ttl key=61 ttl=preserve
        """);
    try (var actual = UnifiedIMapServiceSemanticAdapter.fresh("invalid")) {
      new SemanticOracle().compare(scenario, InMemorySemanticAdapter.strict("oracle"), actual).requireEquivalent();
      assertEquals(1, actual.snapshot().liveCardinality()); assertTrue(actual.drainEvents().isEmpty());
      assertEquals(Optional.of(b("31")), actual.execute(ScenarioStep.get("unchanged", b("61"))).value());
    }
  }

  @Test void actualFaultCanariesFailTheOracleAndAllAdapterOwnershipIsReleased() throws Exception {
    var scenario = manifest("step=a put key=61 value=31 ttl=eternal\nstep=b get_and_put key=61 value=32 ttl=eternal\n");
    var service = owner();
    try (var broken = new UnifiedIMapServiceSemanticAdapter("canary", service, true)) {
      assertFalse(new SemanticOracle().compare(scenario, InMemorySemanticAdapter.strict("oracle"), broken).equivalent());
      assertEquals(b("31"), service.diagnosticSnapshot(0).get(0).value());
    }
    var actual = new UnifiedIMapServiceSemanticAdapter("lifecycle", service);
    service.execute(new UnifiedIMapService.DomainCommand("evict").key(b("61")).identity(new BoundedIMapApi.MutationIdentity("external", 1)));
    assertEquals(MapEvent.Kind.REMOVED, actual.drainEvents().get(0).kind());
    service.execute(new UnifiedIMapService.DomainCommand("destroy").identity(new BoundedIMapApi.MutationIdentity("external", 2)));
    assertEquals(MapEvent.Kind.INVALIDATED, actual.drainEvents().get(0).kind());
    assertEquals(ErrorClass.INTERNAL, actual.execute(ScenarioStep.get("dead", b("61"))).errorClass());
    actual.close(); actual.close(); assertThrows(IllegalStateException.class, actual::snapshot);
    assertThrows(IllegalStateException.class, actual::drainEvents); assertThrows(NullPointerException.class, () -> actual.execute(null));
    assertThrows(IllegalArgumentException.class, () -> new UnifiedIMapServiceSemanticAdapter("x".repeat(97), owner()));
    assertThrows(IllegalStateException.class, () -> UnifiedIMapServiceSemanticAdapter.sha256("missing"));
    var tiny = new UnifiedIMapService(new BoundedIMapApi.Bounds(1, 1, 8, 8, 8, 128, 8), 1);
    try (var bounded = new UnifiedIMapServiceSemanticAdapter("bounds", tiny)) { assertEquals(ErrorClass.SIZE, bounded.execute(write("large", Operation.PUT, "0102", "31", TtlDirective.eternal())).errorClass()); }
  }
}
