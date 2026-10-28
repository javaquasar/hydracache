package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.*;

import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import java.util.OptionalLong;
import java.util.concurrent.atomic.AtomicLong;
import org.junit.jupiter.api.Test;

final class ExtendedIMapApiTest {
  private static final BoundedIMapApi.Bounds BOUNDS =
      new BoundedIMapApi.Bounds(8, 8, 8, 2, 2, 256, 8);
  private static BytesValue bytes(String hex) { return BytesValue.fromHex(hex); }
  private static BoundedIMapApi.MutationIdentity id(long sequence) {
    return new BoundedIMapApi.MutationIdentity("client", sequence);
  }

  @Test
  void pointMethodsPreserveEmptyBytesVersionAndExpiry() {
    var map = new BoundedIMapApi(BOUNDS);
    var key = bytes("");
    assertEquals(bytes("ff"), map.getOrDefault(key, bytes("ff"), 0));
    assertFalse(map.containsEntry(key, bytes(""), 0));
    map.set(key, bytes(""), 5L, 0);
    var view = map.getEntryView(key, 0).orElseThrow();
    assertEquals(bytes(""), map.getOrDefault(key, bytes("ff"), 0));
    assertTrue(map.containsEntry(key, bytes(""), 0));
    assertFalse(map.containsEntry(key, bytes("ff"), 0));
    assertEquals(OptionalLong.of(5), view.remainingTtl());
    map.set(key, bytes("31"), null, 1);
    var replaced = map.getEntryView(key, 1).orElseThrow();
    assertTrue(replaced.version() > view.version());
    assertTrue(replaced.remainingTtl().isEmpty());
    map.set(key, bytes("32"), 5L, 1);
    assertTrue(map.getEntryView(key, 5).isEmpty());
  }

  @Test
  void timedMutationsRetainIdentityAndNeverDuplicateLostResponses() {
    var map = new BoundedIMapApi(BOUNDS);
    assertEquals(BoundedIMapApi.TryStatus.NOT_APPLIED,
        map.tryPut(id(1), bytes("61"), bytes("31"), null, 0, 0, false).status());
    assertEquals(0, map.size(0));
    var unknown = map.tryPut(id(1), bytes("61"), bytes("31"), null, 10, 0, true);
    assertEquals(BoundedIMapApi.TryStatus.OUTCOME_UNKNOWN, unknown.status());
    long version = map.getEntryView(bytes("61"), 0).orElseThrow().version();
    var replay = map.tryPut(id(1), bytes("61"), bytes("31"), null, 0, 0, false);
    assertEquals(BoundedIMapApi.TryStatus.APPLIED, replay.status());
    assertEquals(OptionalLong.of(version), replay.version());
    assertEquals(version, map.getEntryView(bytes("61"), 0).orElseThrow().version());
    assertThrows(IllegalArgumentException.class,
        () -> map.tryPut(id(1), bytes("61"), bytes("32"), null, 10, 0, false));

    assertEquals(BoundedIMapApi.TryStatus.NOT_APPLIED,
        map.tryRemove(id(2), bytes("61"), 0, 0, false).status());
    assertEquals(BoundedIMapApi.TryStatus.OUTCOME_UNKNOWN,
        map.tryRemove(id(2), bytes("61"), 10, 0, true).status());
    assertEquals(BoundedIMapApi.TryStatus.APPLIED,
        map.tryRemove(id(2), bytes("61"), 10, 0, false).status());
    assertEquals(BoundedIMapApi.TryStatus.APPLIED,
        map.tryRemove(id(3), bytes("61"), 10, 0, false).status());
    assertThrows(IllegalArgumentException.class,
        () -> map.tryPut(id(3), bytes("61"), bytes("31"), null, 10, 0, false));
  }

  @Test
  void dedupAndReplacementBoundsPreflightBeforeMutation() {
    var map = new BoundedIMapApi(new BoundedIMapApi.Bounds(1, 1, 1, 1, 1, 256, 2));
    map.tryPut(id(1), bytes("61"), bytes("31"), null, 10, 0, false);
    assertThrows(IllegalArgumentException.class,
        () -> map.tryRemove(id(2), bytes("61"), 10, 0, false));
    assertTrue(map.containsEntry(bytes("61"), bytes("31"), 0));
    assertThrows(IllegalArgumentException.class,
        () -> map.replaceAllDetailed(List.of(
            new BoundedIMapApi.ReplaceInput(bytes("61"), bytes("32")),
            new BoundedIMapApi.ReplaceInput(bytes("62"), bytes("3333"))), 0));
    assertTrue(map.containsEntry(bytes("61"), bytes("31"), 0));
    assertThrows(IllegalArgumentException.class,
        () -> map.replaceAllDetailed(List.of(), 0));
    assertThrows(IllegalArgumentException.class,
        () -> map.replaceAllDetailed(List.of(
            new BoundedIMapApi.ReplaceInput(bytes("61"), bytes("32")),
            new BoundedIMapApi.ReplaceInput(bytes("61"), bytes("33"))), 0));
    assertTrue(map.containsEntry(bytes("61"), bytes("31"), 0));
  }

  @Test
  void explicitReplacementReceiptsStayOrderedAndDoNotInsertMissingKeys() {
    var map = new BoundedIMapApi(BOUNDS);
    map.set(bytes("61"), bytes("31"), 5L, 0);
    map.set(bytes("62"), bytes("32"), 1L, 0);
    var results = map.replaceAllDetailed(List.of(
        new BoundedIMapApi.ReplaceInput(bytes("62"), bytes("ff")),
        new BoundedIMapApi.ReplaceInput(bytes("61"), bytes("33")),
        new BoundedIMapApi.ReplaceInput(bytes("63"), bytes("44"))), 1);
    assertEquals(List.of(0, 1, 2), results.stream().map(BoundedIMapApi.ReplaceReceipt::index).toList());
    assertEquals(List.of(false, true, false),
        results.stream().map(BoundedIMapApi.ReplaceReceipt::replaced).toList());
    assertTrue(results.get(0).version().isEmpty());
    assertTrue(map.containsEntry(bytes("61"), bytes("33"), 1));
    assertEquals(OptionalLong.of(4), map.getEntryView(bytes("61"), 1).orElseThrow().remainingTtl());
    assertEquals(1, map.size(1));
  }

  @Test
  void identifiedBulkReplayPreservesReceiptsTtlAndCrossKindConflicts() {
    var map = new BoundedIMapApi(BOUNDS);
    map.set(bytes("61"), bytes("31"), 5L, 0);
    var request = List.of(new BoundedIMapApi.ReplaceInput(bytes("61"), bytes("32")));
    var timedOut = map.tryReplaceAllDetailed(id(1), request, 0, 0, false);
    assertEquals(BoundedIMapApi.TryStatus.NOT_APPLIED, timedOut.outcome().status());
    assertTrue(timedOut.items().isEmpty());
    var unknown = map.tryReplaceAllDetailed(id(1), request, 10, 0, true);
    assertEquals(BoundedIMapApi.TryStatus.OUTCOME_UNKNOWN, unknown.outcome().status());
    assertTrue(unknown.items().isEmpty());
    long revision = map.revision();
    var replay = map.tryReplaceAllDetailed(id(1), request, 0, 1, false);
    assertEquals(BoundedIMapApi.TryStatus.APPLIED, replay.outcome().status());
    assertEquals(OptionalLong.of(revision), replay.items().get(0).version());
    assertEquals(revision, map.revision());
    assertEquals(OptionalLong.of(4), map.getEntryView(bytes("61"), 1).orElseThrow().remainingTtl());
    assertThrows(IllegalArgumentException.class,
        () -> map.tryRemove(id(1), bytes("61"), 10, 1, false));
    assertThrows(IllegalArgumentException.class, () -> map.tryReplaceAllDetailed(id(1),
        List.of(new BoundedIMapApi.ReplaceInput(bytes("61"), bytes("33"))), 10, 1, false));
    assertEquals(BoundedIMapApi.TryStatus.APPLIED,
        map.tryReplaceAllDetailed(id(2), List.of(
            new BoundedIMapApi.ReplaceInput(bytes("62"), bytes("32"))), 10, 1, false)
            .outcome().status());
    assertThrows(IllegalArgumentException.class,
        () -> map.tryPut(id(2), bytes("62"), bytes("32"), null, 10, 1, false));
    assertEquals(BoundedIMapApi.TryStatus.OUTCOME_UNKNOWN,
        map.tryRemove(id(3), bytes("62"), 10, 1, true).status());
    var noOpReplay = map.tryRemove(id(3), bytes("62"), 10, 1, false);
    assertEquals(BoundedIMapApi.TryStatus.APPLIED, noOpReplay.status());
    assertTrue(noOpReplay.version().isEmpty());
  }

  @Test
  void aggregateReplacementRequestAndReceiptBoundsRejectBeforeWrites() {
    var tiny = new BoundedIMapApi(new BoundedIMapApi.Bounds(8, 8, 2, 1, 1, 2, 2));
    tiny.set(bytes("61"), bytes("31"), null, 0);
    assertThrows(IllegalArgumentException.class, () -> tiny.replaceAllDetailed(List.of(
        new BoundedIMapApi.ReplaceInput(bytes("61"), bytes("32"))), 0));
    assertTrue(tiny.containsEntry(bytes("61"), bytes("31"), 0));
    assertThrows(IllegalArgumentException.class,
        () -> tiny.getOrDefault(bytes("62"), bytes("313131"), 0));
    tiny.set(bytes("62"), bytes("313131"), null, 0);
    assertThrows(IllegalArgumentException.class, () -> tiny.getEntryView(bytes("62"), 0));

    var map = new BoundedIMapApi(new BoundedIMapApi.Bounds(8, 4 * 1024 * 1024,
        300, 1, 1, 1024, 300));
    map.set(bytes("61"), bytes("31"), null, 0);
    BytesValue large = BytesValue.copyOf(new byte[4 * 1024 * 1024]);
    assertThrows(IllegalArgumentException.class, () -> map.replaceAllDetailed(List.of(
        new BoundedIMapApi.ReplaceInput(bytes("61"), large),
        new BoundedIMapApi.ReplaceInput(bytes("62"), large)), 0));
    assertTrue(map.containsEntry(bytes("61"), bytes("31"), 0));
    var tooMany = java.util.Collections.nCopies(257,
        new BoundedIMapApi.ReplaceInput(bytes("61"), bytes("32")));
    assertThrows(IllegalArgumentException.class, () -> map.replaceAllDetailed(tooMany, 0));
    assertTrue(map.containsEntry(bytes("61"), bytes("31"), 0));
  }

  @Test
  void removalReleasesListenerRegistrationAndAllQueuedEvents() {
    var map = new BoundedIMapApi(BOUNDS);
    map.addEntryListener(1);
    map.set(bytes("61"), bytes("31"), null, 0);
    assertEquals(1, map.retainedListenerEvents());
    assertEquals(new BoundedIMapApi.ListenerRemoval(true, 1), map.removeEntryListener(1));
    assertEquals(0, map.retainedListenerEvents());
    assertEquals(new BoundedIMapApi.ListenerRemoval(false, 0), map.removeEntryListener(1));
    assertThrows(IllegalArgumentException.class, () -> map.drainEntryListener(1));
    map.addEntryListener(1);
    assertTrue(map.drainEntryListener(1).isEmpty());
    map.set(bytes("61"), bytes("31"), null, 0);
    assertFalse(map.drainEntryListener(1).get(0).gap());
    for (int index = 0; index < 4; index++) map.set(bytes("61"), bytes("31"), null, 0);
    assertEquals(1, map.retainedListenerEvents());
    var signals = map.drainEntryListener(1);
    assertTrue(signals.get(0).gap());
    assertTrue(signals.get(0).key().isEmpty());
    assertThrows(IllegalArgumentException.class, () -> map.addEntryListener(1));
    assertThrows(IllegalArgumentException.class, () -> map.addEntryListener(0));
    map.destroy(0);
    assertEquals(0, map.retainedListenerEvents());
  }

  @Test
  void cursorIteratorIsLazyBoundedAndRejectsMutationAndExpiry() {
    var map = new BoundedIMapApi(BOUNDS);
    for (String key : List.of("61", "62", "63")) map.set(bytes(key), bytes("31"), null, 0);
    var tick = new AtomicLong();
    var iterator = map.iterator(BoundedIMapApi.Projection.VALUES, 1, tick::get);
    var values = new ArrayList<BytesValue>();
    while (iterator.hasNext()) values.add(iterator.next().value().orElseThrow());
    assertEquals(List.of(bytes("31"), bytes("31"), bytes("31")), values);
    assertThrows(java.util.NoSuchElementException.class, iterator::next);
    var buffered = map.iterator(BoundedIMapApi.Projection.KEYS, 2, tick::get);
    assertTrue(buffered.hasNext());
    map.evict(bytes("61"), 0);
    assertThrows(IllegalStateException.class, buffered::next);
    assertFalse(buffered.hasNext());
    assertThrows(java.util.NoSuchElementException.class, buffered::next);
    map.set(bytes("64"), bytes("32"), 2L, 0);
    var expiring = map.iterator(BoundedIMapApi.Projection.ENTRIES, 1, tick::get);
    assertTrue(expiring.hasNext());
    tick.set(2);
    assertThrows(IllegalStateException.class, expiring::hasNext);
    assertThrows(IllegalArgumentException.class,
        () -> map.iterator(BoundedIMapApi.Projection.KEYS, 0, tick::get));
  }

  @Test
  void closingIteratorMidPageReleasesItsBufferAndStopsFetching() {
    var map = new BoundedIMapApi(BOUNDS);
    map.set(bytes("61"), bytes("31"), null, 0);
    map.set(bytes("62"), bytes("32"), null, 0);
    var iterator = map.iterator(BoundedIMapApi.Projection.ENTRIES, 2, () -> 0);
    assertTrue(iterator.hasNext());
    iterator.close();
    iterator.close();
    map.destroy(0);
    assertFalse(iterator.hasNext());
    assertThrows(java.util.NoSuchElementException.class, iterator::next);
  }

  @Test
  void cursorsCannotCrossMapOrSearchTarget() {
    var map = new BoundedIMapApi(BOUNDS);
    var other = new BoundedIMapApi(BOUNDS);
    for (String key : List.of("61", "62", "63")) {
      map.set(bytes(key), bytes("31"), null, 0);
      other.set(bytes(key), bytes("31"), null, 0);
    }
    var page = map.scan(BoundedIMapApi.Projection.KEYS, Optional.empty(), 1, 0);
    assertEquals(map.revision(), page.nextCursor().orElseThrow().revision());
    assertEquals(1, page.nextCursor().orElseThrow().offset());
    assertThrows(IllegalArgumentException.class,
        () -> other.scan(BoundedIMapApi.Projection.KEYS, page.nextCursor(), 1, 0));
    assertThrows(IllegalArgumentException.class,
        () -> map.scan(BoundedIMapApi.Projection.VALUES, page.nextCursor(), 1, 0));
    var search = map.containsValue(bytes("ff"), Optional.empty(), 1, 0);
    assertThrows(IllegalArgumentException.class,
        () -> map.containsValue(bytes("31"), search.nextCursor(), 1, 0));
    assertThrows(IllegalArgumentException.class, () -> map.scan(BoundedIMapApi.Projection.KEYS,
        Optional.of(new BoundedIMapApi.Cursor(map.revision(), 0)), 1, 0));
  }

  @Test
  void capabilityDescriptorIsExplicitAndCannotEnableProduction() {
    var descriptor = IMapCapabilityDescriptor.local(BOUNDS);
    assertFalse(descriptor.productionEnabled());
    assertEquals(40, descriptor.plannedOperations().size());
    assertTrue(descriptor.plannedOperations().contains("get_and_put"));
    assertFalse(descriptor.supports("get_and_put"));
    assertTrue(descriptor.supports("get_entry_view"));
    descriptor.requireSupported("try_put");
    assertThrows(UnsupportedOperationException.class,
        () -> descriptor.requireSupported("entry_processor"));
    assertEquals(8, descriptor.bounds().get("dedup_entries_per_tenant"));
    assertTrue(descriptor.divergences().contains("local_reference_only"));
    assertTrue(descriptor.unsupportedFamilies().contains("distributed_transaction"));
    assertThrows(UnsupportedOperationException.class, () -> descriptor.operations().add("sql"));
    assertThrows(IllegalArgumentException.class,
        () -> new IMapCapabilityDescriptor(java.util.Set.of(), java.util.Set.of(), java.util.Map.of(),
            java.util.Set.of(), java.util.Set.of()));
  }

  @Test
  void plannedCapabilitiesAndFixedBoundsMatchTheCheckedInContract() throws Exception {
    java.nio.file.Path root = java.nio.file.Path.of("").toAbsolutePath();
    while (root != null && !java.nio.file.Files.isRegularFile(
        root.resolve("docs/testing/imap/0.75/operation-contract.json"))) root = root.getParent();
    assertNotNull(root, "repository operation contract must be present");
    String operations = java.nio.file.Files.readString(
        root.resolve("docs/testing/imap/0.75/operation-contract.json"));
    operations = operations.substring(operations.indexOf("\"operations\""));
    var ids = new java.util.HashSet<String>();
    var matcher = java.util.regex.Pattern.compile("\"id\"\\s*:\\s*\"([^\"]+)\"")
        .matcher(operations);
    while (matcher.find()) ids.add(matcher.group(1));
    var descriptor = IMapCapabilityDescriptor.local(BOUNDS);
    assertEquals(ids, descriptor.plannedOperations());
    String wholeContract = java.nio.file.Files.readString(
        root.resolve("docs/testing/imap/0.75/operation-contract.json"));
    var forbidden = java.util.regex.Pattern.compile(
        "\"unsupported_families\"\\s*:\\s*\\[([^]]*)]").matcher(wholeContract);
    assertTrue(forbidden.find());
    var forbiddenIds = new java.util.HashSet<String>();
    var family = java.util.regex.Pattern.compile("\"([^\"]+)\"").matcher(forbidden.group(1));
    while (family.find()) forbiddenIds.add(family.group(1));
    assertEquals(forbiddenIds, descriptor.unsupportedFamilies());
    String bounds = java.nio.file.Files.readString(
        root.resolve("docs/testing/imap/0.75/resource-bounds.json"));
    for (var bound : descriptor.bounds().entrySet()) {
      var maximum = java.util.regex.Pattern.compile("\"id\"\\s*:\\s*\"" + bound.getKey()
          + "\"[^}]*\"maximum\"\\s*:\\s*(\\d+)").matcher(bounds);
      assertTrue(maximum.find(), "missing bound " + bound.getKey());
      assertTrue(bound.getValue() <= Integer.parseInt(maximum.group(1)), bound.getKey());
      if (bound.getKey().equals("request_bytes")) {
        assertEquals(BoundedIMapApi.MAX_REQUEST_BYTES, Integer.parseInt(maximum.group(1)));
      }
    }
  }

  @Test
  void newValueObjectsRejectInvalidFields() {
    assertThrows(IllegalArgumentException.class,
        () -> new BoundedIMapApi.MutationIdentity("", 1));
    assertThrows(IllegalArgumentException.class,
        () -> new BoundedIMapApi.EntryView(bytes("31"), 0, OptionalLong.empty()));
    assertThrows(IllegalArgumentException.class, () -> new BoundedIMapApi.TryOutcome(
        BoundedIMapApi.TryStatus.APPLIED, id(1), OptionalLong.of(0)));
  }
}
