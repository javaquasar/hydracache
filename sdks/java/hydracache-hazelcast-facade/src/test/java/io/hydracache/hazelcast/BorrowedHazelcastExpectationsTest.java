package io.hydracache.hazelcast;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.Optional;
import java.util.function.Supplier;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import java.util.stream.Stream;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.TestFactory;

/** Executes source-pinned Hazelcast expectations adapted to the narrow HydraCache facade. */
final class BorrowedHazelcastExpectationsTest {
  private static final Pattern ROW = Pattern.compile(
      ".*\\\"id\\\":\\\"([^\\\"]+)\\\".*\\\"expected\\\":\\\"([^\\\"]+)\\\".*"
          + "\\\"hydracache_test\\\":\\\"([^\\\"]+)\\\".*");
  private static final Duration TIMEOUT = Duration.ofSeconds(1);
  private static final Map<String, Case> CASES = cases();

  @TestFactory
  Stream<DynamicTest> borrowedHazelcastSuiteOutcomesMatchTheManifestExactly() throws IOException {
    Map<String, ManifestRow> rows = loadManifest();
    Map<String, Case> cases = new LinkedHashMap<>(CASES);
    if ("W1_SWALLOW".equals(System.getenv("HYDRACACHE_CANARY_DEFECT"))) {
      cases.remove("hz-map-put-get");
    }
    assertEquals(rows.keySet(), cases.keySet(),
        "HC-CANARY-RED:W1 manifest and executable expectation IDs differ");
    return rows.values().stream().map(row -> DynamicTest.dynamicTest(row.id(), () -> {
      Case testCase = cases.get(row.id());
      assertEquals(row.testName(), testCase.testName(), "manifest runner mapping drifted");
      assertEquals(row.expected(), testCase.observed().get(),
          "unexpected pass/fail/divergence versus manifest");
    }));
  }

  @TestFactory
  Stream<DynamicTest> canaryBorrowedSuiteRunnerTreatsAnUnlistedFailureAsSkip() throws IOException {
    if (!"W1_SWALLOW".equals(System.getenv("HYDRACACHE_CANARY_DEFECT"))) {
      return Stream.empty();
    }
    return borrowedHazelcastSuiteOutcomesMatchTheManifestExactly();
  }

  private static Map<String, Case> cases() {
    Map<String, Case> cases = new LinkedHashMap<>();
    cases.put("hz-map-put-get", pass("map_put_get", () -> {
      HydraMap<String, String> map = map(new HydraCacheFacadeTest.FakeClient(), "put-get");
      map.put("key", "value");
      assertEquals("value", map.get("key").orElseThrow());
    }));
    cases.put("hz-map-remove", pass("map_remove_current", () -> {
      HydraMap<String, String> map = map(new HydraCacheFacadeTest.FakeClient(), "remove");
      map.put("key", "value");
      assertTrue(map.remove("key", "value"));
      assertTrue(map.get("key").isEmpty());
    }));
    cases.put("hz-map-remove-if-same", pass("map_remove_if_same", () -> {
      HydraMap<String, String> map = map(new HydraCacheFacadeTest.FakeClient(), "remove-same");
      map.put("key", "value");
      assertFalse(map.remove("key", "stale"));
      assertEquals("value", map.get("key").orElseThrow());
    }));
    cases.put("hz-map-replace", pass("map_replace", () -> {
      HydraMap<String, String> map = map(new HydraCacheFacadeTest.FakeClient(), "replace");
      map.put("key", "old");
      assertTrue(map.replace("key", "old", "new"));
      assertEquals("new", map.get("key").orElseThrow());
    }));
    cases.put("hz-map-replace-if-same", pass("map_replace_if_same", () -> {
      HydraMap<String, String> map = map(new HydraCacheFacadeTest.FakeClient(), "replace-same");
      map.put("key", "old");
      assertFalse(map.replace("key", "stale", "new"));
      assertEquals("old", map.get("key").orElseThrow());
    }));
    cases.put("hz-map-listener", pass("map_entry_listener", () -> {
      HydraMap<String, String> map = map(new HydraCacheFacadeTest.FakeClient(), "listener");
      var event = new java.util.concurrent.atomic.AtomicReference<EntryEvent<String, String>>();
      try (var ignored = map.addEntryListener(new CapturingListener(event))) {
        map.put("key", "value");
        assertEquals("key", event.get().key());
      }
    }));
    cases.put("hz-map-listener-value", pass("map_listener_value", () -> {
      HydraMap<String, String> map = map(new HydraCacheFacadeTest.FakeClient(), "listener-value");
      var event = new java.util.concurrent.atomic.AtomicReference<EntryEvent<String, String>>();
      try (var ignored = map.addEntryListener(new CapturingListener(event))) {
        map.put("key", "value");
        assertEquals(Optional.of("value"), event.get().value());
      }
    }));
    cases.put("hz-map-ttl", pass("map_put_with_ttl", () -> {
      HydraCacheFacadeTest.FakeClient client = new HydraCacheFacadeTest.FakeClient();
      HydraMap<String, String> map = map(client, "ttl");
      map.put("key", "value", Duration.ofSeconds(5));
      assertEquals("value", map.get("key").orElseThrow());
      client.advanceTime(Duration.ofSeconds(6));
      assertTrue(map.get("key").isEmpty());
      assertThrows(IllegalArgumentException.class,
          () -> map.put("bad", "value", Duration.ofSeconds(-1)));
    }));
    cases.put("hz-lock-free", pass("lock_free_acquire", () -> {
      HydraFencedLock<String> lock = lock(new HydraCacheFacadeTest.FakeClient(), "free");
      assertTrue(lock.tryLock(Duration.ofSeconds(5)).isPresent());
      lock.unlock();
    }));
    cases.put("hz-lock-held", pass("lock_held_by_other", () -> {
      HydraCacheFacadeTest.FakeClient client = new HydraCacheFacadeTest.FakeClient();
      HydraFencedLock<String> first = lock(client, "held");
      HydraFencedLock<String> second = lock(client, "held");
      first.tryLock(Duration.ofSeconds(5)).orElseThrow();
      assertTrue(second.tryLock(Duration.ofSeconds(5)).isEmpty());
      first.unlock();
    }));
    cases.put("hz-lock-ownership", pass("lock_ownership", () -> {
      HydraFencedLock<String> lock = lock(new HydraCacheFacadeTest.FakeClient(), "owner");
      long fence = lock.tryLock(Duration.ofSeconds(5)).orElseThrow();
      assertTrue(lock.isLocked());
      assertEquals(fence, lock.getFence().orElseThrow());
      lock.unlock();
      assertFalse(lock.isLocked());
    }));
    cases.put("hz-lock-renew", pass("lock_renew", () -> {
      HydraFencedLock<String> lock = lock(new HydraCacheFacadeTest.FakeClient(), "renew");
      lock.tryLock(Duration.ofSeconds(5)).orElseThrow();
      lock.renew(Duration.ofSeconds(10));
      lock.unlock();
    }));
    cases.put("hz-lock-lease-expiry", pass("lock_lease_expiry", () -> {
      HydraCacheFacadeTest.FakeClient client = new HydraCacheFacadeTest.FakeClient();
      HydraFencedLock<String> first = lock(client, "lease-expiry");
      HydraFencedLock<String> successor = lock(client, "lease-expiry");
      long firstFence = first.tryLock(Duration.ofSeconds(1)).orElseThrow();
      client.advanceTime(Duration.ofSeconds(2));
      long successorFence = successor.tryLock(Duration.ofSeconds(1)).orElseThrow();
      assertTrue(successorFence > firstFence);
      successor.unlock();
    }));
    cases.put("hz-lock-session-loss", new Case("lock_session_loss_divergence", () -> {
      assertTrue(java.util.Arrays.stream(HydraFencedLock.class.getMethods())
          .noneMatch(method -> method.getName().toLowerCase().contains("session")));
      return "divergence-documented";
    }));
    cases.put("hz-lock-reentrant", new Case("lock_non_reentrant_divergence", () -> {
      HydraFencedLock<String> lock = lock(new HydraCacheFacadeTest.FakeClient(), "reentrant");
      lock.tryLock(Duration.ofSeconds(5)).orElseThrow();
      assertThrows(HydraCacheFacadeException.class,
          () -> lock.tryLock(Duration.ofSeconds(5)));
      lock.unlock();
      return "divergence-documented";
    }));
    cases.put("hz-entry-processor", new Case("execute_on_key_unsupported", () -> {
      HydraMap<String, String> map = map(new HydraCacheFacadeTest.FakeClient(), "execute");
      assertThrows(UnsupportedOperationException.class, () -> map.executeOnKey("key"));
      return "unsupported-documented";
    }));
    return Map.copyOf(cases);
  }

  private static Case pass(String name, ThrowingRunnable body) {
    return new Case(name, () -> {
      body.run();
      return "pass";
    });
  }

  private static HydraMap<String, String> map(HydraCacheFacadeTest.FakeClient client, String name) {
    return new HydraCacheInstance(client, TIMEOUT)
        .getMap(name, HydraCodec.utf8(), HydraCodec.utf8());
  }

  private static HydraFencedLock<String> lock(
      HydraCacheFacadeTest.FakeClient client, String key) {
    return new HydraCacheInstance(client, TIMEOUT)
        .getFencedLock("borrowed", key, HydraCodec.utf8());
  }

  private static Map<String, ManifestRow> loadManifest() throws IOException {
    Path cursor = Path.of(System.getProperty("user.dir")).toAbsolutePath();
    Path manifest = null;
    while (cursor != null) {
      Path candidate = cursor.resolve(Path.of("docs", "integrations",
          "hazelcast_borrowed_suite.json"));
      if (Files.isRegularFile(candidate)) {
        manifest = candidate;
        break;
      }
      cursor = cursor.getParent();
    }
    if (manifest == null) throw new IOException("cannot locate Hazelcast borrowed-suite manifest");
    Map<String, ManifestRow> rows = new LinkedHashMap<>();
    for (String line : Files.readAllLines(manifest)) {
      Matcher matcher = ROW.matcher(line);
      if (matcher.matches()) {
        ManifestRow prior = rows.put(matcher.group(1),
            new ManifestRow(matcher.group(1), matcher.group(2), matcher.group(3)));
        assertEquals(null, prior, "duplicate manifest row ID");
      }
    }
    assertFalse(rows.isEmpty(), "borrowed-suite manifest did not yield executable rows");
    return rows;
  }

  private record ManifestRow(String id, String expected, String testName) { }
  private record Case(String testName, Supplier<String> observed) { }

  @FunctionalInterface
  private interface ThrowingRunnable { void run(); }

  private record CapturingListener(
      java.util.concurrent.atomic.AtomicReference<EntryEvent<String, String>> target)
      implements EntryListener<String, String> {
    @Override public void onEvent(EntryEvent<String, String> event) { target.set(event); }
    @Override public void onGap(long watermark) { throw new AssertionError("unexpected gap"); }
  }
}
