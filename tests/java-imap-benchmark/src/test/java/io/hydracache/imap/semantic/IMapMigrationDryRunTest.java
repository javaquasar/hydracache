package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.OptionalInt;
import java.util.OptionalLong;
import org.junit.jupiter.api.Test;

final class IMapMigrationDryRunTest {
  private static final String ROOT = "549c5f499215bda23f4f47cd1da100ac1be57c0ebfade39da61b6f8ca7d81d63";
  private static final String EMPTY_ROOT = "2c54704a5a5fffe97161de43afdbeb2d6f4c173af8ba4da372dee63a2cb69caa";
  private static final IMapMigrationDryRun.Limits LIMITS = new IMapMigrationDryRun.Limits(8, 4096, 2, 1024, 128, 256);
  private static BytesValue b(String hex) { return BytesValue.fromHex(hex); }
  private static IMapMigrationDryRun.Entry entry(String key, String value, Long ttl) { return new IMapMigrationDryRun.Entry(b(key), b(value), ttl == null ? OptionalLong.empty() : OptionalLong.of(ttl)); }
  private static IMapMigrationDryRun.Fixture fixture() {
    return new IMapMigrationDryRun.Fixture("binary-unicode-empty", "binary_identity_v1", "tenant-ü", "cache-猫", 7,
        "offline-fixture-a", List.of(entry("", "", null), entry("00ff", "000102ff", 11L), entry("c3a9", "e78cab", 7L), entry("65cc81", "f09f9880", null)), 4, ROOT);
  }
  private static UnifiedIMapService live() { return new UnifiedIMapService(new BoundedIMapApi.Bounds(8192, 256, 64, 64, 64, 4096, 64), 7); }
  private static IMapMigrationDryRun.Chunk changed(IMapMigrationDryRun.Chunk source, int index, String codec, String tenant, String namespace, long generation, String snapshot, String digest) {
    return new IMapMigrationDryRun.Chunk(index, codec, tenant, namespace, generation, snapshot, source.entries(), digest);
  }
  private static void acceptAll(IMapMigrationDryRun dry, IMapMigrationDryRun.Fixture fixture) {
    dry.accept(IMapMigrationDryRun.chunk(fixture, 0, fixture.entries().subList(0, 2)));
    dry.accept(IMapMigrationDryRun.chunk(fixture, 1, fixture.entries().subList(2, 4)));
  }

  @Test void actualStagingMatchesSharedGoldenCodecTtlDigestAndNeverMutatesLiveOwner() throws Exception {
    var fixture = fixture(); var live = live();
    live.execute(new UnifiedIMapService.DomainCommand("set").key(b("ff")).value(b("31")).identity(new BoundedIMapApi.MutationIdentity("live", 1)));
    var before = live.diagnosticSnapshot(0);
    try (var dry = new IMapMigrationDryRun(fixture, LIMITS, live)) {
      assertThrows(IllegalArgumentException.class, dry::finish);
      var first = IMapMigrationDryRun.chunk(fixture, 0, fixture.entries().subList(0, 2));
      assertFalse(dry.accept(first).duplicate()); var usage = dry.staging().resourceUsage();
      assertTrue(dry.accept(first).duplicate()); assertEquals(usage, dry.staging().resourceUsage());
      dry.accept(IMapMigrationDryRun.chunk(fixture, 1, fixture.entries().subList(2, 4)));
      var receipt = dry.finish(); assertEquals(4, receipt.entries()); assertEquals(2, receipt.chunks());
      assertTrue(receipt.dryRun()); assertFalse(receipt.productionImport()); assertTrue(receipt.liveUnchanged());
      assertEquals(ROOT, receipt.sourceDigest()); assertEquals(ROOT, receipt.stagingDigest());
      assertEquals(List.of(11L, 7L), dry.staging().diagnosticSnapshot(0).stream().filter(entry -> entry.remainingTtl().isPresent()).map(entry -> entry.remainingTtl().getAsLong()).toList());
      assertEquals(before, live.diagnosticSnapshot(0));
      dry.close(); dry.close(); assertEquals(0, dry.staging().resourceUsage().values()); assertEquals(0, dry.staging().resourceUsage().outcomes());
      assertThrows(IllegalArgumentException.class, dry::finish); assertThrows(IllegalArgumentException.class, () -> dry.accept(first));
    }
    Path root = Path.of("").toAbsolutePath(); while (root != null && !Files.isRegularFile(root.resolve("docs/testing/imap/0.75/migration-fixtures.json"))) root = root.getParent();
    assertNotNull(root); String corpus = Files.readString(root.resolve("docs/testing/imap/0.75/migration-fixtures.json"));
    assertTrue(corpus.contains(ROOT)); assertTrue(corpus.contains(EMPTY_ROOT));
    assertEquals("c3a9", BytesValue.copyOf("é".getBytes(StandardCharsets.UTF_8)).hex());
    assertEquals("65cc81", BytesValue.copyOf("é".getBytes(StandardCharsets.UTF_8)).hex());
    assertNotEquals(b("c3a9"), b("65cc81"));
    var empty = new IMapMigrationDryRun.Fixture("empty-source", "binary_identity_v1", "tenant-ü", "empty", 8, "offline-empty", List.of(), 0, EMPTY_ROOT);
    try (var dry = new IMapMigrationDryRun(empty, LIMITS, live())) {
      dry.accept(IMapMigrationDryRun.chunk(empty, 0, List.of())); assertEquals(0, dry.finish().entries());
    }
  }

  @Test void chunkScopeOrderingChecksumIdentityAndBudgetsFailBeforeWrites() {
    var fixture = fixture(); var first = IMapMigrationDryRun.chunk(fixture, 0, fixture.entries().subList(0, 2));
    try (var dry = new IMapMigrationDryRun(fixture, LIMITS, live())) {
      var refused = List.of(changed(first, 0, "unknown", first.tenant(), first.namespace(), 7, first.sourceSnapshot(), first.digest()),
          changed(first, 0, first.codecId(), "other", first.namespace(), 7, first.sourceSnapshot(), first.digest()),
          changed(first, 0, first.codecId(), first.tenant(), "other", 7, first.sourceSnapshot(), first.digest()),
          changed(first, 0, first.codecId(), first.tenant(), first.namespace(), 8, first.sourceSnapshot(), first.digest()),
          changed(first, 0, first.codecId(), first.tenant(), first.namespace(), 7, "other", first.digest()),
          changed(first, 0, first.codecId(), first.tenant(), first.namespace(), 7, first.sourceSnapshot(), "0".repeat(64)),
          IMapMigrationDryRun.chunk(fixture, 1, first.entries()), IMapMigrationDryRun.chunk(fixture, 0, List.of()),
          IMapMigrationDryRun.chunk(fixture, 0, fixture.entries().subList(0, 3)),
          IMapMigrationDryRun.chunk(fixture, 0, List.of(fixture.entries().get(0), fixture.entries().get(0))));
      for (var chunk : refused) { assertThrows(IllegalArgumentException.class, () -> dry.accept(chunk)); assertTrue(dry.staging().diagnosticSnapshot(0).isEmpty()); }
      dry.accept(first);
      assertThrows(IllegalArgumentException.class, () -> dry.accept(IMapMigrationDryRun.chunk(fixture, 0, List.of(entry("", "01", null), fixture.entries().get(1)))));
    }
    try (var dry = new IMapMigrationDryRun(fixture, new IMapMigrationDryRun.Limits(8, 4096, 2, 1, 128, 256), live())) {
      assertThrows(IllegalArgumentException.class, () -> dry.accept(first)); assertEquals(0, dry.staging().resourceUsage().values());
    }
    assertThrows(IllegalArgumentException.class, () -> IMapMigrationDryRun.chunk(fixture, -1, List.of()));
  }

  @Test void actualServiceDefectsAndFixtureContractMisuseCannotProduceAValidReceipt() {
    var fixture = fixture(); var live = live();
    try (var dry = new IMapMigrationDryRun(fixture, LIMITS, live)) {
      acceptAll(dry, fixture);
      live.execute(new UnifiedIMapService.DomainCommand("set").key(b("01")).value(b("01")).identity(new BoundedIMapApi.MutationIdentity("external", 1)));
      assertFalse(dry.finish().liveUnchanged());
      var key = dry.staging().diagnosticSnapshot(0).get(0).key();
      dry.staging().execute(new UnifiedIMapService.DomainCommand("set").key(key).value(b("99")).identity(new BoundedIMapApi.MutationIdentity("defect", 1)));
      assertThrows(IllegalArgumentException.class, dry::finish);
    }
    try (var dry = new IMapMigrationDryRun(fixture, LIMITS, live())) {
      acceptAll(dry, fixture);
      var key = ReferenceCanonicalKeyCodec.encode(new ReferenceCanonicalKeyCodec.Key("tenant-ü", "wrong", 7, new byte[0]), ReferenceCanonicalKeyCodec.Bounds.defaults());
      dry.staging().execute(new UnifiedIMapService.DomainCommand("set").key(BytesValue.copyOf(key)).value(b("01")).identity(new BoundedIMapApi.MutationIdentity("defect", 1)));
      assertThrows(IllegalArgumentException.class, dry::finish);
    }
    assertThrows(IllegalArgumentException.class, () -> new IMapMigrationDryRun.Limits(0, 1, 1, 1, 1, 1));
    assertThrows(IllegalArgumentException.class, () -> new IMapMigrationDryRun(new IMapMigrationDryRun.Fixture("id", "unknown", "t", "n", 1, "source", List.of(), 0, EMPTY_ROOT), LIMITS, live()));
    assertThrows(IllegalArgumentException.class, () -> new IMapMigrationDryRun(new IMapMigrationDryRun.Fixture("id", "binary_identity_v1", "t", "n", 1, "source", List.of(entry("", "", 0L)), 1, "bad"), LIMITS, live()));
    assertThrows(IllegalArgumentException.class, () -> new IMapMigrationDryRun(new IMapMigrationDryRun.Fixture("id", "binary_identity_v1", "t", "n", 1, "source", List.of(entry("", "", null), entry("", "", null)), 2, "bad"), LIMITS, live()));
    assertThrows(IllegalArgumentException.class, () -> new IMapMigrationDryRun(new IMapMigrationDryRun.Fixture("id", "binary_identity_v1", "t", "n", 1, "source", List.of(), 0, "bad"), LIMITS, live()));
    assertThrows(IllegalArgumentException.class, () -> IMapMigrationDryRun.digest(fixture, List.of(new IMapMigrationDryRun.Entry(b(""), BytesValue.copyOf(new byte[1024 * 1024]), OptionalLong.empty())), OptionalInt.empty()));
    assertThrows(IllegalStateException.class, () -> IMapMigrationDryRun.hash(new byte[0], "missing-digest"));
    var excessive = java.util.Collections.nCopies(257, entry("", "", null));
    assertThrows(IllegalArgumentException.class, () -> new IMapMigrationDryRun.Fixture("id", "binary_identity_v1", "t", "n", 1, "source", excessive, 257, "bad"));
    assertThrows(IllegalArgumentException.class, () -> IMapMigrationDryRun.digest(fixture, excessive, OptionalInt.empty()));
    assertThrows(IllegalArgumentException.class, () -> IMapMigrationDryRun.chunk(fixture, 0, excessive.subList(0, 65)));
    var oversized = new IMapMigrationDryRun.Fixture("id", "binary_identity_v1", "t", "n", 1, "source", List.of(entry("00".repeat(129), "", null)), 1, "bad");
    assertThrows(IllegalArgumentException.class, () -> new IMapMigrationDryRun(oversized, LIMITS, live()));
  }
}
