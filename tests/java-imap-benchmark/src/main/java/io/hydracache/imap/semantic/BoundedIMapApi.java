package io.hydracache.imap.semantic;

import java.util.ArrayList;
import java.util.List;
import java.util.Objects;
import java.util.Optional;
import java.util.TreeMap;

/** Test-only bounded reference for collection views and IMap lifecycle operations. */
public final class BoundedIMapApi {
  public enum Projection { KEYS, VALUES, ENTRIES }
  public enum Match { PRESENT, ABSENT, INCOMPLETE }
  public enum RemovalCause { CLEAR, EVICT, DESTROY }

  public record Bounds(
      int maxKeyBytes,
      int maxValueBytes,
      int maxEntries,
      int maxPageItems,
      int maxScanItems,
      int maxResponseBytes,
      int maxBulkMutationItems) {
    public Bounds {
      if (maxKeyBytes <= 0 || maxValueBytes <= 0 || maxEntries <= 0 || maxPageItems <= 0
          || maxScanItems <= 0 || maxResponseBytes <= 0 || maxBulkMutationItems <= 0) {
        throw new IllegalArgumentException("all collection bounds must be positive");
      }
    }
  }

  public record Cursor(long revision, int offset) {
    public Cursor {
      if (revision <= 0 || offset < 0) throw new IllegalArgumentException("invalid cursor");
    }
  }

  public record ScanItem(Optional<BytesValue> key, Optional<BytesValue> value) {
    public ScanItem {
      key = Objects.requireNonNull(key, "key");
      value = Objects.requireNonNull(value, "value");
      if (key.isEmpty() && value.isEmpty()) throw new IllegalArgumentException("empty scan item");
    }
  }

  public record ScanPage(
      List<ScanItem> items,
      Optional<Cursor> nextCursor,
      boolean complete,
      long snapshotRevision) {
    public ScanPage {
      items = List.copyOf(items);
      nextCursor = Objects.requireNonNull(nextCursor, "nextCursor");
      if (complete == nextCursor.isPresent()) {
        throw new IllegalArgumentException("cursor completeness mismatch");
      }
    }
  }

  public record MatchResult(Match match, Optional<Cursor> nextCursor) {
    public MatchResult {
      Objects.requireNonNull(match, "match");
      nextCursor = Objects.requireNonNull(nextCursor, "nextCursor");
      if ((match == Match.INCOMPLETE) != nextCursor.isPresent()) {
        throw new IllegalArgumentException("incomplete match requires a cursor");
      }
    }
  }

  public record RemovalReceipt(RemovalCause cause, int removed, long revision) {
    public RemovalReceipt {
      Objects.requireNonNull(cause, "cause");
      if (removed < 0 || revision <= 0) throw new IllegalArgumentException("invalid receipt");
    }
  }

  private record StoredValue(BytesValue value, Long expiresAt) {}

  private final Bounds bounds;
  private final TreeMap<BytesValue, StoredValue> entries = new TreeMap<>();
  private long revision = 1;
  private boolean destroyed;

  public BoundedIMapApi(Bounds bounds) {
    this.bounds = Objects.requireNonNull(bounds, "bounds");
  }

  public long revision() { return revision; }
  public boolean isDestroyed() { return destroyed; }

  public void put(BytesValue key, BytesValue value, Long expiresAt, long now) {
    requireActive();
    Objects.requireNonNull(key, "key");
    Objects.requireNonNull(value, "value");
    checkBound("keyBytes", bounds.maxKeyBytes(), key.size());
    checkBound("valueBytes", bounds.maxValueBytes(), value.size());
    if (expiresAt != null && expiresAt <= now) {
      throw new IllegalArgumentException("expiry must be in the future");
    }
    purgeExpired(now);
    if (!entries.containsKey(key)) checkBound("entries", bounds.maxEntries(), entries.size() + 1);
    entries.put(key, new StoredValue(value, expiresAt));
    revision++;
  }

  public int size(long now) {
    requireActive();
    purgeExpired(now);
    return entries.size();
  }

  public boolean isEmpty(long now) { return size(now) == 0; }

  public MatchResult containsValue(
      BytesValue value, Optional<Cursor> cursor, int scanBudget, long now) {
    requireActive();
    Objects.requireNonNull(value, "value");
    Objects.requireNonNull(cursor, "cursor");
    checkBound("valueBytes", bounds.maxValueBytes(), value.size());
    requirePositive("scanBudget", scanBudget);
    checkBound("scanItems", bounds.maxScanItems(), scanBudget);
    purgeExpired(now);
    int offset = cursorOffset(cursor);
    int visited = 0;
    for (StoredValue stored : entries.values().stream().skip(offset).limit(scanBudget).toList()) {
      visited++;
      if (stored.value().equals(value)) return new MatchResult(Match.PRESENT, Optional.empty());
    }
    int nextOffset = Math.min(entries.size(), offset + visited);
    if (nextOffset == entries.size()) return new MatchResult(Match.ABSENT, Optional.empty());
    return new MatchResult(
        Match.INCOMPLETE, Optional.of(new Cursor(revision, nextOffset)));
  }

  public ScanPage scan(
      Projection projection, Optional<Cursor> cursor, int pageItems, long now) {
    requireActive();
    Objects.requireNonNull(projection, "projection");
    Objects.requireNonNull(cursor, "cursor");
    requirePositive("pageItems", pageItems);
    checkBound("pageItems", bounds.maxPageItems(), pageItems);
    purgeExpired(now);
    int offset = cursorOffset(cursor);
    int responseBytes = 0;
    var items = new ArrayList<ScanItem>();
    for (var entry : entries.entrySet().stream().skip(offset).limit(pageItems).toList()) {
      int encodedBytes = switch (projection) {
        case KEYS -> entry.getKey().size();
        case VALUES -> entry.getValue().value().size();
        case ENTRIES -> Math.addExact(entry.getKey().size(), entry.getValue().value().size());
      };
      responseBytes = Math.addExact(responseBytes, encodedBytes);
      checkBound("responseBytes", bounds.maxResponseBytes(), responseBytes);
      items.add(switch (projection) {
        case KEYS -> new ScanItem(Optional.of(entry.getKey()), Optional.empty());
        case VALUES -> new ScanItem(Optional.empty(), Optional.of(entry.getValue().value()));
        case ENTRIES ->
            new ScanItem(Optional.of(entry.getKey()), Optional.of(entry.getValue().value()));
      });
    }
    int nextOffset = offset + items.size();
    boolean complete = nextOffset == entries.size();
    return new ScanPage(items,
        complete ? Optional.empty() : Optional.of(new Cursor(revision, nextOffset)),
        complete, revision);
  }

  public boolean evict(BytesValue key, long now) {
    requireActive();
    Objects.requireNonNull(key, "key");
    checkBound("keyBytes", bounds.maxKeyBytes(), key.size());
    purgeExpired(now);
    boolean removed = entries.remove(key) != null;
    if (removed) revision++;
    return removed;
  }

  public RemovalReceipt clear(long now) { return removeAll(RemovalCause.CLEAR, now); }
  public RemovalReceipt evictAll(long now) { return removeAll(RemovalCause.EVICT, now); }

  public RemovalReceipt destroy(long now) {
    RemovalReceipt receipt = removeAll(RemovalCause.DESTROY, now);
    destroyed = true;
    revision++;
    return new RemovalReceipt(receipt.cause(), receipt.removed(), revision);
  }

  private RemovalReceipt removeAll(RemovalCause cause, long now) {
    requireActive();
    purgeExpired(now);
    checkBound("bulkMutationItems", bounds.maxBulkMutationItems(), entries.size());
    int removed = entries.size();
    if (removed > 0) {
      entries.clear();
      revision++;
    }
    return new RemovalReceipt(cause, removed, revision);
  }

  private int cursorOffset(Optional<Cursor> cursor) {
    if (cursor.isEmpty()) return 0;
    Cursor value = cursor.orElseThrow();
    if (value.revision() != revision) throw new IllegalStateException("stale scan cursor");
    return Math.min(value.offset(), entries.size());
  }

  private void purgeExpired(long now) {
    int before = entries.size();
    entries.entrySet().removeIf(
        entry -> entry.getValue().expiresAt() != null && entry.getValue().expiresAt() <= now);
    if (before != entries.size()) revision++;
  }

  private void requireActive() {
    if (destroyed) throw new IllegalStateException("map is destroyed");
  }

  private static void requirePositive(String name, int value) {
    if (value <= 0) throw new IllegalArgumentException(name + " must be positive");
  }

  private static void checkBound(String name, int limit, int actual) {
    if (actual > limit) throw new IllegalArgumentException(name + " exceeds " + limit);
  }
}
