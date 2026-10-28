package io.hydracache.imap.semantic;

import java.util.ArrayList;
import java.util.ArrayDeque;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Iterator;
import java.util.List;
import java.util.Map;
import java.util.NoSuchElementException;
import java.util.Objects;
import java.util.Optional;
import java.util.OptionalLong;
import java.util.TreeMap;
import java.util.function.LongSupplier;
import java.nio.charset.StandardCharsets;

/** Test-only bounded reference for collection views and IMap lifecycle operations. */
public final class BoundedIMapApi {
  public static final int MAX_REQUEST_ENTRIES = 256;
  public static final int MAX_REQUEST_BYTES = 8 * 1024 * 1024;
  public enum Projection { KEYS, VALUES, ENTRIES }
  public enum Match { PRESENT, ABSENT, INCOMPLETE }
  public enum RemovalCause { CLEAR, EVICT, DESTROY }
  public enum TryStatus { APPLIED, NOT_APPLIED, OUTCOME_UNKNOWN }

  public interface CloseableIterator<T> extends Iterator<T>, AutoCloseable {
    @Override void close();
  }

  public record EntryView(BytesValue value, long version, OptionalLong remainingTtl) {
    public EntryView {
      Objects.requireNonNull(value, "value");
      Objects.requireNonNull(remainingTtl, "remainingTtl");
      if (version <= 0 || (remainingTtl.isPresent() && remainingTtl.getAsLong() <= 0)) {
        throw new IllegalArgumentException("invalid entry metadata");
      }
    }
  }

  public record MutationIdentity(String client, long sequence) {
    public MutationIdentity {
      if (Objects.requireNonNull(client, "client").isBlank()
          || client.getBytes(StandardCharsets.UTF_8).length > 128
          || sequence <= 0) throw new IllegalArgumentException("invalid mutation identity");
    }
  }

  public record TryOutcome(TryStatus status, MutationIdentity identity, OptionalLong version) {
    public TryOutcome {
      Objects.requireNonNull(status, "status");
      Objects.requireNonNull(identity, "identity");
      Objects.requireNonNull(version, "version");
      if (version.isPresent() && version.getAsLong() <= 0) {
        throw new IllegalArgumentException("invalid version");
      }
    }
  }

  public record ReplaceInput(BytesValue key, BytesValue value) {
    public ReplaceInput {
      Objects.requireNonNull(key, "key");
      Objects.requireNonNull(value, "value");
    }
  }

  public record ReplaceReceipt(int index, BytesValue key, boolean replaced, OptionalLong version) {}
  public record BulkTryOutcome(TryOutcome outcome, List<ReplaceReceipt> items) {
    public BulkTryOutcome {
      Objects.requireNonNull(outcome, "outcome");
      items = List.copyOf(items);
    }
  }

  public record ListenerRemoval(boolean removed, int releasedEvents) {}
  public record ListenerSignal(Optional<BytesValue> key, boolean gap) {}

  private record MutationRequest(BytesValue key, BytesValue value, Long expiresAt, boolean remove) {}
  private record BulkRequest(List<ReplaceInput> replacements) {}
  private record RetainedMutation(Object request, TryOutcome outcome, List<ReplaceReceipt> items,
      Object serviceOutcome) {
    RetainedMutation(Object request, TryOutcome outcome, List<ReplaceReceipt> items) {
      this(request, outcome, items, null);
    }
  }
  private final Map<MutationIdentity, Object> pendingService = new HashMap<>();

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

  public static final class Cursor {
    private final long revision;
    private final int offset;
    private final Object owner;
    private final BytesValue searchValue;
    private final Projection projection;

    public Cursor(long revision, int offset) { this(revision, offset, null, null, null); }

    private Cursor(long revision, int offset, Object owner, BytesValue searchValue,
        Projection projection) {
      if (revision <= 0 || offset < 0) throw new IllegalArgumentException("invalid cursor");
      this.revision = revision;
      this.offset = offset;
      this.owner = owner;
      this.searchValue = searchValue;
      this.projection = projection;
    }

    public long revision() { return revision; }
    public int offset() { return offset; }
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

  private record StoredValue(BytesValue value, Long expiresAt, long version) {}

  private final Bounds bounds;
  private final TreeMap<BytesValue, StoredValue> entries = new TreeMap<>();
  private final Map<MutationIdentity, RetainedMutation> retained = new HashMap<>();
  private final Map<Long, ArrayDeque<ListenerSignal>> listeners = new HashMap<>();
  private final Object cursorOwner = new Object();
  private long revision = 1;
  private boolean destroyed;

  public BoundedIMapApi(Bounds bounds) {
    this.bounds = Objects.requireNonNull(bounds, "bounds");
  }

  public synchronized long revision() { return revision; }
  public synchronized boolean isDestroyed() { return destroyed; }

  public synchronized void put(BytesValue key, BytesValue value, Long expiresAt, long now) {
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
    revision++;
    entries.put(key, new StoredValue(value, expiresAt, revision));
    publish(key);
  }

  public synchronized BytesValue getOrDefault(BytesValue key, BytesValue fallback, long now) {
    Objects.requireNonNull(fallback, "fallback");
    checkBound("valueBytes", bounds.maxValueBytes(), fallback.size());
    BytesValue result = getEntryView(key, now).map(EntryView::value).orElse(fallback);
    checkBound("responseBytes", bounds.maxResponseBytes(), result.size());
    return result;
  }

  public synchronized boolean containsEntry(BytesValue key, BytesValue expected, long now) {
    Objects.requireNonNull(expected, "expected");
    checkBound("valueBytes", bounds.maxValueBytes(), expected.size());
    return getEntryView(key, now).map(view -> view.value().equals(expected)).orElse(false);
  }

  public synchronized void set(BytesValue key, BytesValue value, Long expiresAt, long now) {
    put(key, value, expiresAt, now);
  }

  public synchronized Optional<EntryView> getEntryView(BytesValue key, long now) {
    requireActive();
    Objects.requireNonNull(key, "key");
    checkBound("keyBytes", bounds.maxKeyBytes(), key.size());
    purgeExpired(now);
    StoredValue stored = entries.get(key);
    if (stored == null) return Optional.empty();
    checkBound("responseBytes", bounds.maxResponseBytes(), stored.value().size());
    return Optional.of(new EntryView(stored.value(), stored.version(), stored.expiresAt() == null
        ? OptionalLong.empty() : OptionalLong.of(stored.expiresAt() - now)));
  }

  public synchronized TryOutcome tryPut(MutationIdentity identity, BytesValue key,
      BytesValue value, Long expiresAt, long deadline, long now, boolean responseLostAfterApply) {
    Objects.requireNonNull(value, "value");
    return tryMutate(identity, new MutationRequest(key, value, expiresAt, false), deadline, now,
        responseLostAfterApply);
  }

  public synchronized TryOutcome tryRemove(MutationIdentity identity, BytesValue key,
      long deadline, long now, boolean responseLostAfterApply) {
    return tryMutate(identity, new MutationRequest(key, null, null, true), deadline, now,
        responseLostAfterApply);
  }

  private TryOutcome tryMutate(MutationIdentity identity, MutationRequest request,
      long deadline, long now, boolean responseLostAfterApply) {
    requireActive();
    Objects.requireNonNull(identity, "identity");
    Objects.requireNonNull(request.key(), "key");
    RetainedMutation prior = retained.get(identity);
    if (prior != null) {
      if (!prior.request().equals(request)) throw new IllegalArgumentException("mutation conflict");
      return prior.outcome();
    }
    if (pendingService.containsKey(identity)) throw new IllegalArgumentException("mutation conflict");
    if (deadline <= now) return new TryOutcome(TryStatus.NOT_APPLIED, identity, OptionalLong.empty());
    checkBound("dedupEntries", bounds.maxEntries(), retained.size() + pendingService.size() + 1);
    boolean applied;
    if (request.remove()) {
      applied = evict(request.key(), now);
    } else {
      put(request.key(), request.value(), request.expiresAt(), now);
      applied = true;
    }
    TryOutcome outcome = new TryOutcome(TryStatus.APPLIED,
        identity, applied ? OptionalLong.of(revision) : OptionalLong.empty());
    retained.put(identity, new RetainedMutation(request, outcome, List.of()));
    return responseLostAfterApply
        ? new TryOutcome(TryStatus.OUTCOME_UNKNOWN, identity, OptionalLong.empty()) : outcome;
  }

  public synchronized List<ReplaceReceipt> replaceAllDetailed(List<ReplaceInput> replacements,
      long now) {
    requireActive();
    Objects.requireNonNull(replacements, "replacements");
    requirePositive("replacementCount", replacements.size());
    checkBound("requestEntries", Math.min(bounds.maxBulkMutationItems(), MAX_REQUEST_ENTRIES),
        replacements.size());
    List<ReplaceInput> input = List.copyOf(replacements);
    var seen = new HashSet<BytesValue>();
    long requestBytes = 0;
    long responseBytes = 0;
    for (ReplaceInput replacement : input) {
      checkBound("keyBytes", bounds.maxKeyBytes(), replacement.key().size());
      checkBound("valueBytes", bounds.maxValueBytes(), replacement.value().size());
      if (!seen.add(replacement.key())) throw new IllegalArgumentException("duplicate key");
      requestBytes += 8L + replacement.key().size() + replacement.value().size();
      responseBytes += 17L + replacement.key().size();
      if (requestBytes > MAX_REQUEST_BYTES) throw new IllegalArgumentException("request byte bound");
      if (responseBytes > bounds.maxResponseBytes()) {
        throw new IllegalArgumentException("response byte bound");
      }
    }
    purgeExpired(now);
    var result = new ArrayList<ReplaceReceipt>(input.size());
    for (int index = 0; index < input.size(); index++) {
      ReplaceInput replacement = input.get(index);
      boolean present = entries.containsKey(replacement.key());
      if (present) put(replacement.key(), replacement.value(),
          entries.get(replacement.key()).expiresAt(), now);
      result.add(new ReplaceReceipt(index, replacement.key(), present,
          present ? OptionalLong.of(revision) : OptionalLong.empty()));
    }
    return List.copyOf(result);
  }

  public synchronized BulkTryOutcome tryReplaceAllDetailed(MutationIdentity identity,
      List<ReplaceInput> replacements, long deadline, long now, boolean responseLostAfterApply) {
    requireActive();
    Objects.requireNonNull(identity, "identity");
    Objects.requireNonNull(replacements, "replacements");
    requirePositive("replacementCount", replacements.size());
    checkBound("requestEntries", Math.min(bounds.maxBulkMutationItems(), MAX_REQUEST_ENTRIES),
        replacements.size());
    BulkRequest request = new BulkRequest(List.copyOf(replacements));
    RetainedMutation prior = retained.get(identity);
    if (prior != null) {
      if (!prior.request().equals(request)) throw new IllegalArgumentException("mutation conflict");
      return new BulkTryOutcome(prior.outcome(), prior.items());
    }
    if (pendingService.containsKey(identity)) throw new IllegalArgumentException("mutation conflict");
    if (deadline <= now) return new BulkTryOutcome(
        new TryOutcome(TryStatus.NOT_APPLIED, identity, OptionalLong.empty()), List.of());
    checkBound("dedupEntries", bounds.maxEntries(), retained.size() + pendingService.size() + 1);
    List<ReplaceReceipt> result = replaceAllDetailed(request.replacements(), now);
    boolean applied = result.stream().anyMatch(ReplaceReceipt::replaced);
    TryOutcome outcome = new TryOutcome(TryStatus.APPLIED,
        identity, applied ? OptionalLong.of(revision) : OptionalLong.empty());
    retained.put(identity, new RetainedMutation(request, outcome, result));
    if (responseLostAfterApply) return new BulkTryOutcome(
        new TryOutcome(TryStatus.OUTCOME_UNKNOWN, identity, OptionalLong.empty()), List.of());
    return new BulkTryOutcome(outcome, result);
  }

  public synchronized void addEntryListener(long registration) {
    requireActive();
    if (registration <= 0 || listeners.containsKey(registration)) {
      throw new IllegalArgumentException("invalid listener registration");
    }
    checkBound("listeners", bounds.maxEntries(), listeners.size() + 1);
    listeners.put(registration, new ArrayDeque<>());
  }

  public synchronized List<ListenerSignal> drainEntryListener(long registration) {
    requireActive();
    ArrayDeque<ListenerSignal> events = listeners.get(registration);
    if (events == null) throw new IllegalArgumentException("unknown listener registration");
    var result = List.copyOf(events);
    events.clear();
    return result;
  }

  public synchronized ListenerRemoval removeEntryListener(long registration) {
    requireActive();
    ArrayDeque<ListenerSignal> removed = listeners.remove(registration);
    return new ListenerRemoval(removed != null, removed == null ? 0 : removed.size());
  }

  public synchronized int retainedListenerEvents() {
    return listeners.values().stream().mapToInt(ArrayDeque::size).sum();
  }

  private void publish(BytesValue key) {
    for (var queue : listeners.values()) {
      if (!queue.isEmpty() && queue.peekFirst().gap()) continue;
      if (queue.size() >= bounds.maxPageItems()) {
        queue.clear();
        queue.add(new ListenerSignal(Optional.empty(), true));
      } else {
        queue.add(new ListenerSignal(Optional.of(key), false));
      }
    }
  }

  public synchronized int size(long now) {
    requireActive();
    purgeExpired(now);
    return entries.size();
  }

  public synchronized boolean isEmpty(long now) { return size(now) == 0; }

  public synchronized MatchResult containsValue(
      BytesValue value, Optional<Cursor> cursor, int scanBudget, long now) {
    requireActive();
    Objects.requireNonNull(value, "value");
    Objects.requireNonNull(cursor, "cursor");
    checkBound("valueBytes", bounds.maxValueBytes(), value.size());
    requirePositive("scanBudget", scanBudget);
    checkBound("scanItems", bounds.maxScanItems(), scanBudget);
    purgeExpired(now);
    int offset = cursorOffset(cursor, value, null);
    int visited = 0;
    for (StoredValue stored : entries.values().stream().skip(offset).limit(scanBudget).toList()) {
      visited++;
      if (stored.value().equals(value)) return new MatchResult(Match.PRESENT, Optional.empty());
    }
    int nextOffset = Math.min(entries.size(), offset + visited);
    if (nextOffset == entries.size()) return new MatchResult(Match.ABSENT, Optional.empty());
    return new MatchResult(
        Match.INCOMPLETE, Optional.of(new Cursor(revision, nextOffset, cursorOwner, value, null)));
  }

  public synchronized ScanPage scan(
      Projection projection, Optional<Cursor> cursor, int pageItems, long now) {
    requireActive();
    Objects.requireNonNull(projection, "projection");
    Objects.requireNonNull(cursor, "cursor");
    requirePositive("pageItems", pageItems);
    checkBound("pageItems", bounds.maxPageItems(), pageItems);
    purgeExpired(now);
    int offset = cursorOffset(cursor, null, projection);
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
        complete ? Optional.empty()
            : Optional.of(new Cursor(revision, nextOffset, cursorOwner, null, projection)),
        complete, revision);
  }

  public synchronized boolean evict(BytesValue key, long now) {
    requireActive();
    Objects.requireNonNull(key, "key");
    checkBound("keyBytes", bounds.maxKeyBytes(), key.size());
    purgeExpired(now);
    boolean removed = entries.remove(key) != null;
    if (removed) {
      revision++;
      publish(key);
    }
    return removed;
  }

  public synchronized RemovalReceipt clear(long now) { return removeAll(RemovalCause.CLEAR, now); }
  public synchronized RemovalReceipt evictAll(long now) { return removeAll(RemovalCause.EVICT, now); }

  public synchronized RemovalReceipt destroy(long now) {
    requireActive();
    int removed = (int) entries.values().stream()
        .filter(value -> value.expiresAt() == null || value.expiresAt() > now).count();
    checkBound("bulkMutationItems", bounds.maxBulkMutationItems(), removed);
    revision = Math.incrementExact(revision);
    entries.clear();
    destroyed = true;
    retained.clear();
    pendingService.clear();
    listeners.clear();
    return new RemovalReceipt(RemovalCause.DESTROY, removed, revision);
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

  private int cursorOffset(Optional<Cursor> cursor, BytesValue searchValue, Projection projection) {
    if (cursor.isEmpty()) return 0;
    Cursor value = cursor.orElseThrow();
    if (value.owner != cursorOwner || !Objects.equals(value.searchValue, searchValue)
        || value.projection != projection) {
      throw new IllegalArgumentException("foreign or invalid cursor");
    }
    if (value.revision() != revision) throw new IllegalStateException("stale scan cursor");
    if (value.offset() > entries.size()) throw new IllegalArgumentException("foreign or invalid cursor");
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

  /** Lazy iterator retaining at most one bounded page; mutation fails even with a buffered page. */
  public synchronized CloseableIterator<ScanItem> iterator(Projection projection, int pageItems,
      LongSupplier clock) {
    requireActive();
    Objects.requireNonNull(projection, "projection");
    Objects.requireNonNull(clock, "clock");
    requirePositive("pageItems", pageItems);
    checkBound("pageItems", bounds.maxPageItems(), pageItems);
    purgeExpired(clock.getAsLong());
    long initialRevision = revision;
    return new CloseableIterator<>() {
      private final ArrayDeque<ScanItem> buffered = new ArrayDeque<>();
      private Optional<Cursor> cursor = Optional.empty();
      private boolean complete;
      private boolean closed;

      @Override public boolean hasNext() {
        synchronized (BoundedIMapApi.this) {
          if (closed) return false;
          try {
            requireActive();
            purgeExpired(clock.getAsLong());
            if (revision != initialRevision) throw new IllegalStateException("stale iterator");
            if (buffered.isEmpty() && !complete) {
              ScanPage page = scan(projection, cursor, pageItems, clock.getAsLong());
              buffered.addAll(page.items());
              cursor = page.nextCursor();
              complete = page.complete();
            }
            if (buffered.isEmpty()) close();
            return !buffered.isEmpty();
          } catch (RuntimeException failure) {
            close();
            throw failure;
          }
        }
      }

      @Override public ScanItem next() {
        synchronized (BoundedIMapApi.this) {
          if (!hasNext()) throw new NoSuchElementException();
          return buffered.removeFirst();
        }
      }

      @Override public void close() {
        synchronized (BoundedIMapApi.this) {
          buffered.clear();
          cursor = Optional.empty();
          complete = true;
          closed = true;
        }
      }
    };
  }

  record RetainedEntry(BytesValue key, BytesValue value, Long expiresAt, long version) {}
  record LegacyOutcome(TryOutcome outcome, List<ReplaceReceipt> items) {}

  synchronized List<RetainedEntry> retainedEntries() {
    return entries.entrySet().stream().map(entry -> new RetainedEntry(entry.getKey(),
        entry.getValue().value(), entry.getValue().expiresAt(), entry.getValue().version())).toList();
  }

  synchronized void expireForService(long now) { if (!destroyed) purgeExpired(now); }
  Bounds serviceBounds() { return bounds; }

  synchronized Object recoverService(MutationIdentity identity, Object request) {
    RetainedMutation value = retained.get(identity);
    if (value == null) return null;
    if (!value.request().equals(request)) throw new IllegalArgumentException("mutation conflict");
    return value.serviceOutcome();
  }

  synchronized void beginService(MutationIdentity identity, Object request) {
    requireActive();
    Object pending = pendingService.get(identity);
    if (pending != null && !pending.equals(request)) {
      throw new IllegalArgumentException("mutation conflict");
    }
    if (pending == null) {
      checkBound("dedupEntries", bounds.maxEntries(), retained.size() + pendingService.size() + 1);
      pendingService.put(identity, request);
    }
  }

  synchronized void completeService(MutationIdentity identity, Object request, Object outcome) {
    pendingService.remove(identity);
    retained.put(identity, new RetainedMutation(request, null, List.of(), outcome));
  }

  synchronized void abandonService(MutationIdentity identity) { pendingService.remove(identity); }
  synchronized boolean servicePending(MutationIdentity identity) {
    return pendingService.containsKey(identity);
  }
  synchronized Object serviceOutcome(MutationIdentity identity) {
    RetainedMutation value = retained.get(identity);
    return value == null ? null : value.serviceOutcome() == null
        ? new LegacyOutcome(value.outcome(), value.items()) : value.serviceOutcome();
  }
  synchronized int serviceRetainedCount() { return retained.size(); }
  synchronized int servicePendingCount() { return pendingService.size(); }
  synchronized int serviceLiveCount() { return entries.size(); }
  synchronized long serviceValueBytes() {
    return entries.values().stream().mapToLong(entry -> entry.value().size()).sum();
  }
}
