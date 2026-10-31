package io.hydracache.imap.semantic;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.OptionalInt;
import java.util.OptionalLong;

/** Development fixture qualification over an actual staging service, without a live import API. */
public final class IMapMigrationDryRun implements AutoCloseable {
  public static final String MODE = "api_only_cold_repopulation";
  public record Limits(int maxEntries, int maxTotalBytes, int maxChunkEntries, int maxChunkBytes, int maxKeyBytes, int maxValueBytes) {
    public Limits {
      if (maxEntries <= 0 || maxEntries > 256 || maxTotalBytes <= 0 || maxTotalBytes > 1024 * 1024
          || maxChunkEntries <= 0 || maxChunkEntries > 64 || maxChunkEntries > maxEntries
          || maxChunkBytes <= 0 || maxChunkBytes > maxTotalBytes || maxKeyBytes <= 0 || maxKeyBytes > 4096
          || maxValueBytes <= 0 || maxValueBytes > 65536) throw new IllegalArgumentException("migration fixture limits");
    }
  }
  public record Entry(BytesValue key, BytesValue value, OptionalLong ttlRemaining) {
    public Entry { Objects.requireNonNull(key); Objects.requireNonNull(value); Objects.requireNonNull(ttlRemaining); }
  }
  public record Fixture(String id, String codecId, String tenant, String namespace, long generation,
      String sourceSnapshot, List<Entry> entries, int expectedCount, String expectedDigest) {
    public Fixture { if (entries.size() > 256) throw new IllegalArgumentException("migration fixture hard count bound"); entries = List.copyOf(entries); Objects.requireNonNull(expectedDigest); }
  }
  public record Chunk(int index, String codecId, String tenant, String namespace, long generation,
      String sourceSnapshot, List<Entry> entries, String digest) { public Chunk { if (entries.size() > 64) throw new IllegalArgumentException("migration chunk hard count bound"); entries = List.copyOf(entries); } }
  public record ChunkReceipt(int index, boolean duplicate, int entries, int nextIndex) {}
  public record Receipt(String mode, boolean dryRun, boolean productionImport, int entries, int chunks,
      long ownedBytes, String sourceDigest, String stagingDigest, boolean liveUnchanged) {}
  private final Fixture fixture;
  private final Limits limits;
  private final UnifiedIMapService live, staging;
  private final List<UnifiedIMapService.SnapshotEntry> before;
  private final Map<Integer, String> completed = new HashMap<>();
  private int accepted;
  private boolean closed;

  public IMapMigrationDryRun(Fixture fixture, Limits limits, UnifiedIMapService live) {
    this.fixture = Objects.requireNonNull(fixture); this.limits = Objects.requireNonNull(limits); this.live = Objects.requireNonNull(live);
    if (fixture.id().isEmpty() || utf8(fixture.id()).length > 64 || fixture.sourceSnapshot().isEmpty()
        || utf8(fixture.sourceSnapshot()).length > 64 || !fixture.codecId().equals("binary_identity_v1")
        || fixture.generation() <= 0 || fixture.entries().size() != fixture.expectedCount()
        || fixture.expectedCount() > limits.maxEntries()) throw new IllegalArgumentException("migration fixture identity/codec/count");
    physical(new Entry(BytesValue.fromHex(""), BytesValue.fromHex(""), OptionalLong.empty()));
    long bytes = 0; var keys = new HashSet<BytesValue>();
    for (var entry : fixture.entries()) { validateEntry(entry); if (!keys.add(entry.key())) throw new IllegalArgumentException("duplicate migration key"); bytes += ownerBytes(entry); }
    if (bytes > limits.maxTotalBytes() || !digest(fixture, fixture.entries(), OptionalInt.empty()).equals(fixture.expectedDigest())) throw new IllegalArgumentException("migration source digest or budget mismatch");
    var defaults = UnifiedIMapService.BudgetLimits.defaults();
    var budget = new UnifiedIMapService.BudgetLimits(limits.maxTotalBytes(), defaults.outcomes(), defaults.pending(), defaults.listeners(), defaults.streams(), defaults.resync(), defaults.maxClients(), defaults.maxBatch(), defaults.expiryWork(), defaults.queryBytes());
    staging = new UnifiedIMapService(new BoundedIMapApi.Bounds(8192, limits.maxValueBytes(), limits.maxEntries() + 1, 256, 256, 1024 * 1024, limits.maxEntries() + 1), fixture.generation(), budget);
    before = live.diagnosticSnapshot(0);
  }
  public static Chunk chunk(Fixture fixture, int index, List<Entry> entries) {
    if (index < 0) throw new IllegalArgumentException("negative migration chunk index");
    return new Chunk(index, fixture.codecId(), fixture.tenant(), fixture.namespace(), fixture.generation(), fixture.sourceSnapshot(), entries, digest(fixture, entries, OptionalInt.of(index)));
  }
  public ChunkReceipt accept(Chunk chunk) {
    requireOpen();
    if (!chunk.codecId().equals(fixture.codecId()) || !chunk.tenant().equals(fixture.tenant())
        || !chunk.namespace().equals(fixture.namespace()) || chunk.generation() != fixture.generation()
        || !chunk.sourceSnapshot().equals(fixture.sourceSnapshot())) throw new IllegalArgumentException("migration chunk scope/codec/source mismatch");
    if (chunk.index() < 0 || chunk.entries().size() > limits.maxChunkEntries()
        || (chunk.entries().isEmpty() && fixture.expectedCount() != 0)
        || !digest(fixture, chunk.entries(), OptionalInt.of(chunk.index())).equals(chunk.digest())) throw new IllegalArgumentException("migration corrupt or oversized chunk");
    String prior = completed.get(chunk.index());
    if (prior != null) {
      if (!prior.equals(chunk.digest())) throw new IllegalArgumentException("migration chunk identity conflict");
      return new ChunkReceipt(chunk.index(), true, chunk.entries().size(), completed.size());
    }
    if (chunk.index() != completed.size() || completed.size() > limits.maxEntries()) throw new IllegalArgumentException("migration chunk out of order");
    var keys = new HashSet<BytesValue>(); staging.diagnosticSnapshot(0).forEach(entry -> keys.add(entry.key()));
    long bytes = 0;
    for (var entry : chunk.entries()) { validateEntry(entry); if (!keys.add(physical(entry))) throw new IllegalArgumentException("duplicate migration key"); bytes += ownerBytes(entry); }
    if (bytes > limits.maxChunkBytes() || staging.resourceUsage().values() + bytes > limits.maxTotalBytes()
        || accepted + chunk.entries().size() > fixture.expectedCount()) throw new IllegalArgumentException("migration chunk budget/count refusal");
    for (var entry : chunk.entries()) {
      var command = new UnifiedIMapService.DomainCommand("set").key(physical(entry)).value(entry.value())
          .identity(new BoundedIMapApi.MutationIdentity("migration-dry-run", accepted + 1L))
          .ttl(entry.ttlRemaining().isPresent() ? TtlDirective.expireAfter(entry.ttlRemaining().getAsLong()) : TtlDirective.eternal());
      staging.execute(command); accepted++;
    }
    completed.put(chunk.index(), chunk.digest());
    return new ChunkReceipt(chunk.index(), false, chunk.entries().size(), completed.size());
  }
  public Receipt finish() {
    requireOpen(); if (accepted != fixture.expectedCount()) throw new IllegalArgumentException("migration dry-run incomplete");
    var entries = new ArrayList<Entry>();
    for (var entry : staging.diagnosticSnapshot(0)) {
      var key = ReferenceCanonicalKeyCodec.decode(entry.key().copy(), ReferenceCanonicalKeyCodec.Bounds.defaults());
      if (!key.tenant().equals(fixture.tenant()) || !key.namespace().equals(fixture.namespace()) || key.generation() != fixture.generation()) throw new IllegalArgumentException("staging namespace mismatch");
      entries.add(new Entry(BytesValue.copyOf(key.key()), entry.value(), entry.remainingTtl()));
    }
    String actual = digest(fixture, entries, OptionalInt.empty());
    if (entries.size() != fixture.expectedCount() || !actual.equals(fixture.expectedDigest())) throw new IllegalArgumentException("actual staging cardinality/digest mismatch");
    return new Receipt(MODE, true, false, entries.size(), completed.size(), staging.resourceUsage().values(), fixture.expectedDigest(), actual, before.equals(live.diagnosticSnapshot(0)));
  }
  UnifiedIMapService staging() { return staging; }
  @Override public void close() {
    if (!closed) { staging.execute(new UnifiedIMapService.DomainCommand("destroy").identity(new BoundedIMapApi.MutationIdentity("migration-close", 1))); completed.clear(); closed = true; }
  }
  private void requireOpen() { if (closed) throw new IllegalArgumentException("migration dry-run is closed"); }
  private void validateEntry(Entry entry) {
    if (entry.key().size() > limits.maxKeyBytes() || entry.value().size() > limits.maxValueBytes()
        || (entry.ttlRemaining().isPresent() && entry.ttlRemaining().getAsLong() <= 0)) throw new IllegalArgumentException("migration entry bounds/TTL");
  }
  private BytesValue physical(Entry entry) { return BytesValue.copyOf(ReferenceCanonicalKeyCodec.encode(new ReferenceCanonicalKeyCodec.Key(fixture.tenant(), fixture.namespace(), fixture.generation(), entry.key().copy()), ReferenceCanonicalKeyCodec.Bounds.defaults())); }
  private long ownerBytes(Entry entry) { return 24L + physical(entry).size() + entry.value().size(); }
  private static byte[] utf8(String value) { return value.getBytes(StandardCharsets.UTF_8); }
  public static String digest(Fixture fixture, List<Entry> input, OptionalInt index) {
    if (input.size() > 256 || List.of(fixture.codecId(), fixture.tenant(), fixture.namespace(), fixture.sourceSnapshot()).stream().anyMatch(text -> utf8(text).length > 256)) throw new IllegalArgumentException("migration digest scope/count bound");
    var entries = new ArrayList<>(input); if (index.isEmpty()) entries.sort(java.util.Comparator.comparing(Entry::key));
    byte[] domain = utf8("HC075-MIGRATION-DRY-RUN\0");
    var scope = List.of(fixture.codecId(), fixture.tenant(), fixture.namespace(), fixture.sourceSnapshot());
    long size = domain.length + 8 + 1 + (index.isPresent() ? 8 : 0) + 4;
    for (var text : scope) size += 4L + utf8(text).length;
    for (var entry : entries) size += 16L + entry.key().size() + entry.value().size();
    if (size > 1024 * 1024) throw new IllegalArgumentException("migration digest frame bound");
    var frame = ByteBuffer.allocate(Math.toIntExact(size)); frame.put(domain);
    for (var text : scope) { byte[] encoded = utf8(text); frame.putInt(encoded.length).put(encoded); }
    frame.putLong(fixture.generation()).put((byte) (index.isPresent() ? 1 : 0));
    if (index.isPresent()) frame.putLong(index.getAsInt()); frame.putInt(entries.size());
    for (var entry : entries) frame.putInt(entry.key().size()).put(entry.key().copy()).putInt(entry.value().size()).put(entry.value().copy()).putLong(entry.ttlRemaining().orElse(0));
    return hash(frame.array(), "SHA-256");
  }
  static String hash(byte[] frame, String algorithm) {
    try { return java.util.HexFormat.of().formatHex(MessageDigest.getInstance(algorithm).digest(frame)); }
    catch (NoSuchAlgorithmException failure) { throw new IllegalStateException("required migration digest unavailable", failure); }
  }
}
