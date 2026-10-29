package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.*;
import java.util.Arrays;
import java.util.List;
import java.util.Optional;
import java.util.OptionalLong;
import java.util.Set;
import java.util.function.LongSupplier;
import org.junit.jupiter.api.Test;

final class IMapSemanticCompatibilityTest {
  private static BytesValue b(String value) { return BytesValue.fromHex(value); }
  private static UnifiedIMapService owner() { return new UnifiedIMapService(new BoundedIMapApi.Bounds(4, 8, 32, 16, 32, 256, 32), 7); }
  private static void signature(String method, String result, Class<?>... parameters) throws Exception {
    var declared = UnifiedIMapService.class.getDeclaredMethod(method, parameters);
    assertTrue(java.lang.reflect.Modifier.isPublic(declared.getModifiers()));
    assertEquals(result, declared.getGenericReturnType().getTypeName().replace("io.hydracache.imap.semantic.", ""));
  }

  @Test void publicSignaturesAndTypedContractsStayProvisional() throws Exception {
    signature("execute", "UnifiedIMapService$Result", UnifiedIMapService.DomainCommand.class);
    signature("beginOperation", "void", UnifiedIMapService.DomainCommand.class);
    signature("executeReserved", "UnifiedIMapService$Result", UnifiedIMapService.DomainCommand.class);
    signature("retireOutcomes", "UnifiedIMapService$RetirementReceipt", String.class, long.class, long.class);
    signature("resourceUsage", "UnifiedIMapService$ResourceUsage");
    signature("snapshotListener", "UnifiedIMapService$SnapshotLease", long.class, int.class, long.class, long.class);
    signature("versionBulk", "UnifiedIMapService$Result", UnifiedIMapService.VersionAction.class, List.class, BoundedIMapApi.MutationIdentity.class, long.class, long.class, boolean.class);
    signature("queryOutcomes", "java.util.List<UnifiedIMapService$OutcomeItem>", List.class, long.class);
    signature("entryViews", "java.util.List<UnifiedIMapService$VersionedEntryView>", List.class, long.class, long.class);
    assertEquals("java.util.List<io.hydracache.imap.semantic.BytesValue>", UnifiedIMapService.class.getDeclaredMethod("entryViews", List.class, long.class, long.class).getGenericParameterTypes()[0].getTypeName());
    assertEquals("java.util.List<io.hydracache.imap.semantic.UnifiedIMapService$VersionInput>", UnifiedIMapService.class.getDeclaredMethod("versionBulk", UnifiedIMapService.VersionAction.class, List.class, BoundedIMapApi.MutationIdentity.class, long.class, long.class, boolean.class).getGenericParameterTypes()[1].getTypeName());
    assertEquals("java.util.List<io.hydracache.imap.semantic.BoundedIMapApi$MutationIdentity>", UnifiedIMapService.class.getDeclaredMethod("queryOutcomes", List.class, long.class).getGenericParameterTypes()[0].getTypeName());
    signature("expireStep", "UnifiedIMapService$ExpiryProgress", long.class, int.class);
    signature("checkpoint", "UnifiedIMapService$RuntimeCheckpoint");
    signature("restore", "UnifiedIMapService", UnifiedIMapService.RuntimeCheckpoint.class);
    signature("pages", "IMapFlowPublisher<BoundedIMapApi$ScanItem>", BoundedIMapApi.Projection.class, int.class, LongSupplier.class);
    signature("events", "IMapFlowPublisher<UnifiedIMapService$Event>", UnifiedIMapService.ListenerHandle.class);
    assertEquals(List.of("namespaceGeneration", "mapId", "version"), Arrays.stream(UnifiedIMapService.VersionToken.class.getRecordComponents()).map(java.lang.reflect.RecordComponent::getName).toList());
    assertEquals(List.of("index", "key", "value", "token", "remainingTtl"), Arrays.stream(UnifiedIMapService.VersionedEntryView.class.getRecordComponents()).map(java.lang.reflect.RecordComponent::getName).toList());
    assertEquals(List.of("keys", "maxBytes", "deadline"), Arrays.stream(IMapLocalHelper.EntryViews.class.getRecordComponents()).map(java.lang.reflect.RecordComponent::getName).toList());
    assertTrue(IMapLocalHelper.Command.class.isAssignableFrom(IMapLocalHelper.EntryViews.class));
    assertEquals(List.of("includeValue", "maxValueBytes", "queueCapacity", "keyPrefix", "exactKey", "kinds"), Arrays.stream(UnifiedIMapService.ListenerOptions.class.getRecordComponents()).map(java.lang.reflect.RecordComponent::getName).toList());
    for (var method : UnifiedIMapService.class.getMethods()) {
      String types = method.getGenericReturnType() + Arrays.toString(method.getGenericParameterTypes());
      assertFalse(types.contains("com.google.protobuf")); assertFalse(types.contains("io.grpc"));
    }
    assertTrue(AutoCloseable.class.isAssignableFrom(UnifiedIMapService.SnapshotLease.class));
    assertTrue(AutoCloseable.class.isAssignableFrom(UnifiedIMapService.RuntimeCheckpoint.class));
    assertTrue(java.util.concurrent.Flow.Publisher.class.isAssignableFrom(IMapFlowPublisher.class));
    assertFalse(IMapCapabilityDescriptor.unified(owner().semanticBounds()).productionEnabled());
  }

  @Test void nullAbsentEmptyAndLogicalTtlStayDistinctBeforeAnyMutation() {
    var service = owner();
    assertThrows(NullPointerException.class, () -> service.execute(null));
    assertThrows(NullPointerException.class, () -> new UnifiedIMapService.DomainCommand(null));
    assertThrows(NullPointerException.class, () -> new UnifiedIMapService.DomainCommand("set").key(null));
    assertThrows(NullPointerException.class, () -> new UnifiedIMapService.DomainCommand("set").value(null));
    assertThrows(IllegalArgumentException.class, () -> TtlDirective.expireAfter(0));
    assertThrows(NullPointerException.class, () -> new UnifiedIMapService.Result("absent", null, OptionalLong.empty(), OptionalLong.empty(), List.of(), 0, false, Optional.empty()));
    assertThrows(NullPointerException.class, () -> new UnifiedIMapService.Item(0, b(""), "absent", null, OptionalLong.empty(), OptionalLong.empty()));
    assertThrows(NullPointerException.class, () -> new UnifiedIMapService.OperationStatus("unknown", null));
    assertEquals(0, service.diagnosticStats(0).requests()); assertTrue(service.diagnosticSnapshot(0).isEmpty());
    service.execute(new UnifiedIMapService.DomainCommand("set").key(b("")).value(b(""))
        .identity(new BoundedIMapApi.MutationIdentity("compat", 1)).ttl(TtlDirective.expireAfter(2)));
    var present = service.execute(new UnifiedIMapService.DomainCommand("get").key(b("")));
    assertEquals(Optional.of(b("")), present.value()); assertEquals(2, present.remainingTtl().orElseThrow());
    var missing = service.execute(new UnifiedIMapService.DomainCommand("get").key(b("01")));
    assertTrue(missing.value().isEmpty()); assertTrue(missing.version().isEmpty()); assertTrue(missing.remainingTtl().isEmpty());
    service.expireStep(2, 1); assertEquals("absent", service.execute(new UnifiedIMapService.DomainCommand("get").key(b("")).now(2)).kind());
  }

  @Test void filtersAndTypedHelpersPreserveBinaryPrefixExactAndNullRules() {
    var service = owner();
    var prefix = service.registerListener(1, new UnifiedIMapService.ListenerOptions(true, 16, 16, b("00ff"), Optional.empty(), Set.of()));
    var exact = service.registerListener(2, new UnifiedIMapService.ListenerOptions(true, 16, 16, Optional.of(b("00ff01")), Set.of()));
    var both = service.registerListener(3, new UnifiedIMapService.ListenerOptions(true, 16, 16, b("00ff"), Optional.of(b("00ff01")), Set.of()));
    var incompatible = service.registerListener(4, new UnifiedIMapService.ListenerOptions(true, 16, 16, b("00fe"), Optional.of(b("00ff01")), Set.of()));
    var empty = service.registerListener(5, new UnifiedIMapService.ListenerOptions(true, 16, 16, Optional.of(b("")), Set.of()));
    assertEquals(655, service.resourceUsage().listeners());
    long sequence = 0;
    for (String key : List.of("", "00", "00fe", "00ff", "00ff01", "00ffff")) service.execute(new UnifiedIMapService.DomainCommand("set").key(b(key)).value(b("31")).identity(new BoundedIMapApi.MutationIdentity("filter", ++sequence)));
    assertEquals(List.of(b("00ff"), b("00ff01"), b("00ffff")), prefix.drain().stream().map(UnifiedIMapService.Event::key).toList());
    assertEquals(List.of(b("00ff01")), exact.drain().stream().map(UnifiedIMapService.Event::key).toList());
    assertEquals(1, both.drain().size()); assertTrue(incompatible.drain().isEmpty()); assertEquals(b(""), empty.drain().get(0).key());
    try (var snapshot = service.snapshotListener(3, 16, 256, 0)) { assertEquals(List.of(b("00ff01")), snapshot.entries().stream().map(UnifiedIMapService.ResyncEntry::key).toList()); }
    try (var snapshot = service.snapshotListener(4, 16, 256, 0)) { assertTrue(snapshot.entries().isEmpty()); }
    service.execute(new UnifiedIMapService.DomainCommand("entry_listener").key(b("00ff")).expected(b("00ff01")).includeValue(true).listenerId(6).limit(16));
    service.execute(new UnifiedIMapService.DomainCommand("set").key(b("00ff01")).value(b("32")).identity(new BoundedIMapApi.MutationIdentity("filter", ++sequence)));
    assertEquals(List.of(b("00ff01")), service.drainListener(6).stream().map(UnifiedIMapService.Event::key).toList());
    prefix.drain(); exact.drain(); both.drain(); incompatible.drain(); empty.drain();
    service.execute(new UnifiedIMapService.DomainCommand("clear_detailed").identity(new BoundedIMapApi.MutationIdentity("filter", ++sequence)));
    for (var handle : List.of(prefix, exact, both, incompatible, empty)) { assertEquals("cleared", handle.drain().get(0).kind()); handle.close(); }
    service.removeListener(6); assertEquals(0, service.resourceUsage().listeners());
    assertThrows(NullPointerException.class, () -> new UnifiedIMapService.ListenerOptions(false, 0, 1, null, Optional.empty(), Set.of()));
    assertThrows(IMapDomainException.class, () -> service.registerListener(8, new UnifiedIMapService.ListenerOptions(false, 0, 1, b("0001020304"), Optional.empty(), Set.of())));
    assertThrows(IMapDomainException.class, () -> service.signalListenerGap(999));
  }

  @Test void ttlUpdatePreserveIsRejectedBeforeExpiryAndRetainedIdentityAdmission() {
    var service = owner(); var listener = service.registerListener(1, UnifiedIMapService.ListenerOptions.metadata(16));
    service.execute(new UnifiedIMapService.DomainCommand("set").key(b("01")).value(b("31")).ttl(TtlDirective.expireAfter(1)).identity(new BoundedIMapApi.MutationIdentity("ttl", 1)));
    var token = service.execute(new UnifiedIMapService.DomainCommand("get_entry_view").key(b("01"))).token().orElseThrow();
    listener.drain(); var before = service.diagnosticSnapshot(0); var bytes = service.resourceUsage();
    long sequence = 2;
    for (String operation : List.of("set_ttl", "set_ttl_if_version")) {
      var identity = new BoundedIMapApi.MutationIdentity("ttl", sequence++);
      var command = new UnifiedIMapService.DomainCommand(operation).key(b("01")).ttl(TtlDirective.preserve()).token(token).now(1).identity(identity);
      assertEquals("invalid_argument", assertThrows(IMapDomainException.class, () -> service.execute(command)).kind());
      assertEquals("unknown", service.queryOperationOutcome(identity).kind());
    }
    assertEquals("invalid_argument", assertThrows(IMapDomainException.class, () -> service.versionBulk(UnifiedIMapService.VersionAction.SET_TTL,
        List.of(new UnifiedIMapService.VersionInput(b("01"), token, Optional.empty(), TtlDirective.preserve())), new BoundedIMapApi.MutationIdentity("ttl", 4), 1, 10, false)).kind());
    assertEquals(before, service.diagnosticSnapshot(0)); assertEquals(bytes, service.resourceUsage()); assertTrue(listener.drain().isEmpty());
    assertEquals(0, service.expireStep(0, 1).expired()); listener.close();
  }
}
