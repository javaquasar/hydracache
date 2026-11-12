package io.hydracache.imap.semantic;

import java.nio.charset.StandardCharsets;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;
import java.util.OptionalLong;
import java.util.Set;
import java.util.TreeMap;
import java.util.concurrent.atomic.AtomicLong;
import java.util.function.LongSupplier;

/** One local command service over the existing bounded map, with no network dispatch. */
public final class UnifiedIMapService {
  private static final AtomicLong NEXT_MAP = new AtomicLong(1);
  private static final BytesValue EMPTY = BytesValue.copyOf(new byte[0]);
  private static final Set<String> MUTATIONS = Set.of("put", "delete", "conditional_replace", "conditional_remove",
      "put_if_absent", "replace_if_present", "get_and_remove", "get_and_put", "put_all_detailed", "remove_all_detailed",
      "set_ttl", "clear_detailed", "destroy", "evict", "evict_all_detailed", "set", "try_put", "try_remove",
      "replace_all_detailed", "replace_if_version", "remove_if_version", "set_ttl_if_version");

  public record VersionToken(long namespaceGeneration, long mapId, long version) {
    public VersionToken {
      if (namespaceGeneration <= 0 || mapId <= 0 || version <= 0) invalid("invalid version token");
    }
  }
  public record Item(int index, BytesValue key, String kind, Optional<BytesValue> value,
      OptionalLong version, OptionalLong remainingTtl) {
    public Item { Objects.requireNonNull(key); Objects.requireNonNull(kind); Objects.requireNonNull(value); Objects.requireNonNull(version); Objects.requireNonNull(remainingTtl); }
  }
  public record Result(String kind, Optional<BytesValue> value, OptionalLong version,
      OptionalLong remainingTtl, List<Item> items, long number, boolean flag,
      Optional<VersionToken> token, Optional<BoundedIMapApi.Cursor> cursor) {
    public Result { Objects.requireNonNull(kind); Objects.requireNonNull(value); Objects.requireNonNull(version); Objects.requireNonNull(remainingTtl); Objects.requireNonNull(token); Objects.requireNonNull(cursor); items = List.copyOf(items); }
    public Result(String kind, Optional<BytesValue> value, OptionalLong version,
        OptionalLong remainingTtl, List<Item> items, long number, boolean flag,
        Optional<VersionToken> token) {
      this(kind, value, version, remainingTtl, items, number, flag, token, Optional.empty());
    }
  }
  public record SnapshotEntry(BytesValue key, BytesValue value, long version,
      OptionalLong remainingTtl) {}
  public record VersionedEntryView(int index, BytesValue key, Optional<BytesValue> value,
      Optional<VersionToken> token, OptionalLong remainingTtl) {
    public VersionedEntryView {
      Objects.requireNonNull(key); Objects.requireNonNull(value); Objects.requireNonNull(token); Objects.requireNonNull(remainingTtl);
      if (index < 0 || value.isPresent() != token.isPresent() || value.isEmpty() && remainingTtl.isPresent()) invalid("entry view shape");
    }
  }
  public record Statistics(long liveEntries, long valueBytes, long retainedOutcomes,
      long listeners, long queuedEvents, long inflight, long rejections, long cancellations,
      long requests, long activeStreams, long activeCursors) {}
  public record Event(String kind, BytesValue key, Optional<BytesValue> oldValue,
      Optional<BytesValue> value, long version, long watermark) {}
  public record ListenerOptions(boolean includeValue, int maxValueBytes, int queueCapacity,
      BytesValue keyPrefix, Optional<BytesValue> exactKey, Set<String> kinds) {
    public ListenerOptions(boolean includeValue, int maxValueBytes, int queueCapacity,
        Optional<BytesValue> exactKey, Set<String> kinds) {
      this(includeValue, maxValueBytes, queueCapacity, EMPTY, exactKey, kinds);
    }
    public ListenerOptions {
      keyPrefix = Objects.requireNonNull(keyPrefix, "keyPrefix");
      exactKey = Objects.requireNonNull(exactKey, "exactKey");
      kinds = Set.copyOf(kinds);
      if (maxValueBytes < 0 || queueCapacity <= 0) invalid("invalid listener options");
      if (!Set.of("added", "updated", "removed", "expired", "evicted", "cleared", "destroyed")
          .containsAll(kinds)) invalid("unsupported listener event kind");
    }
    public static ListenerOptions metadata(int capacity) {
      return new ListenerOptions(false, 0, capacity, Optional.empty(), Set.of());
    }
  }
  public record OperationStatus(String kind, Optional<Result> result) { public OperationStatus { Objects.requireNonNull(kind); Objects.requireNonNull(result); } }
  /** Logical retained payload bounds, not an estimate of JVM RSS. */
  public record BudgetLimits(long values, long outcomes, long pending, long listeners,
      long streams, long resync, int maxClients, int maxBatch, int expiryWork, long queryBytes) {
    public BudgetLimits {
      if (values <= 0 || outcomes <= 0 || pending <= 0 || listeners <= 0 || streams <= 0
          || resync <= 0 || maxClients <= 0 || maxBatch <= 0 || maxBatch > 256 || expiryWork <= 0
          || queryBytes <= 0) throw domain(IMapDomainException.Kind.INVALID_ARGUMENT, "resource limits must be positive and batch <=256");
    }
    public static BudgetLimits defaults() {
      return new BudgetLimits(64L << 20, 32L << 20, 1L << 20, 16L << 20,
          64L << 20, 8L << 20, 4096, 256, 64, 8L << 20);
    }
  }
  public record ResourceUsage(long values, long outcomes, long pending, long listeners,
      long streams, long resync) {}
  public record RetirementReceipt(String client, long generation, long through, int removed) {}
  public record OutcomeItem(int index, BoundedIMapApi.MutationIdentity identity, OperationStatus status) {}
  public enum VersionAction { REPLACE, REMOVE, SET_TTL }
  public record VersionInput(BytesValue key, VersionToken token, Optional<BytesValue> value, TtlDirective ttl) {
    public VersionInput { Objects.requireNonNull(key); Objects.requireNonNull(token); Objects.requireNonNull(value); Objects.requireNonNull(ttl); }
  }
  public record ExpiryProgress(int work, int expired, boolean remaining) {}
  public record ResyncEntry(BytesValue key, Optional<BytesValue> value, long version, OptionalLong remainingTtl) {}

  /** Caller-confined builder. Freeze before publishing it to another thread. */
  public static final class DomainCommand {
    public final String operation;
    public BytesValue key = EMPTY;
    public Optional<BytesValue> value = Optional.empty(), expected = Optional.empty();
    public List<BytesValue> keys = List.of();
    public List<BoundedIMapApi.ReplaceInput> entries = List.of();
    public Optional<BoundedIMapApi.MutationIdentity> identity = Optional.empty();
    public TtlDirective ttl = TtlDirective.eternal();
    public long now, deadline = Long.MAX_VALUE, listenerId;
    public Optional<VersionToken> token = Optional.empty();
    public Optional<BoundedIMapApi.Cursor> cursor = Optional.empty();
    public int limit = 256;
    public boolean includeValue, loseResponse, cancelled;
    public DomainCommand(String operation) { this.operation = Objects.requireNonNull(operation); }
    public DomainCommand key(BytesValue v) { key = Objects.requireNonNull(v); return this; }
    public DomainCommand value(BytesValue v) { value = Optional.of(v); return this; }
    public DomainCommand expected(BytesValue v) { expected = Optional.of(v); return this; }
    public DomainCommand keys(List<BytesValue> v) { keys = List.copyOf(v); return this; }
    public DomainCommand entries(List<BoundedIMapApi.ReplaceInput> v) { entries = List.copyOf(v); return this; }
    public DomainCommand identity(BoundedIMapApi.MutationIdentity v) { identity = Optional.of(v); return this; }
    public DomainCommand ttl(TtlDirective v) { ttl = Objects.requireNonNull(v); return this; }
    public DomainCommand now(long v) { now = v; return this; }
    public DomainCommand deadline(long v) { deadline = v; return this; }
    public DomainCommand token(VersionToken v) { token = Optional.of(v); return this; }
    public DomainCommand cursor(BoundedIMapApi.Cursor v) { cursor = Optional.of(v); return this; }
    public DomainCommand listenerId(long v) { listenerId = v; return this; }
    public DomainCommand limit(int v) { limit = v; return this; }
    public DomainCommand includeValue(boolean v) { includeValue = v; return this; }
    public DomainCommand loseResponse(boolean v) { loseResponse = v; return this; }
    public DomainCommand cancelled(boolean v) { cancelled = v; return this; }
    public RequestSnapshot freeze() {
      return new RequestSnapshot(operation, key, value, expected, keys, entries, identity, ttl,
          now, deadline, listenerId, token, cursor, limit, includeValue, loseResponse, cancelled);
    }
    public DomainCommand copy() {
      return freeze().toBuilder();
    }
  }

  /** Immutable ownership boundary. Admission rechecks current state; preparation guarantees shape and bounds. */
  public record RequestSnapshot(String operation, BytesValue key, Optional<BytesValue> value,
      Optional<BytesValue> expected, List<BytesValue> keys, List<BoundedIMapApi.ReplaceInput> entries,
      Optional<BoundedIMapApi.MutationIdentity> identity, TtlDirective ttl, long now, long deadline,
      long listenerId, Optional<VersionToken> token, Optional<BoundedIMapApi.Cursor> cursor,
      int limit, boolean includeValue, boolean loseResponse, boolean cancelled) {
    public RequestSnapshot {
      Objects.requireNonNull(operation); Objects.requireNonNull(key); Objects.requireNonNull(value);
      Objects.requireNonNull(expected); keys = List.copyOf(keys); entries = List.copyOf(entries);
      Objects.requireNonNull(identity); Objects.requireNonNull(ttl); Objects.requireNonNull(token); Objects.requireNonNull(cursor);
    }
    public DomainCommand toBuilder() {
      var builder = new DomainCommand(operation); builder.key = key; builder.value = value; builder.expected = expected;
      builder.keys = keys; builder.entries = entries; builder.identity = identity; builder.ttl = ttl;
      builder.now = now; builder.deadline = deadline; builder.listenerId = listenerId; builder.token = token;
      builder.cursor = cursor; builder.limit = limit; builder.includeValue = includeValue;
      builder.loseResponse = loseResponse; builder.cancelled = cancelled; return builder;
    }
    public RequestSnapshot atTime(long time) { return toBuilder().now(time).freeze(); }
  }

  private static final class ListenerState {
    final ListenerOptions options;
    final ArrayDeque<Event> queue = new ArrayDeque<>();
    boolean gap, terminal, streamAttached;
    long deliveryEpoch;
    Runnable wake = () -> {};
    ListenerState(ListenerOptions options) { this.options = options; }
  }

  private final BoundedIMapApi map;
  private final BoundedIMapApi.Bounds bounds;
  private final long generation, mapId;
  private final BudgetLimits limits;
  private final TreeMap<Long, ListenerState> listeners = new TreeMap<>();
  private final Set<Runnable> pendingWake = new java.util.LinkedHashSet<>();
  private long now, watermark, requests, rejections, cancellations, activeStreams, activeCursors;
  private long streamBytes, resyncBytes;
  private int resyncLeases;

  public UnifiedIMapService(BoundedIMapApi.Bounds bounds, long namespaceGeneration) {
    this(new BoundedIMapApi(bounds), namespaceGeneration);
  }

  public UnifiedIMapService(BoundedIMapApi existing, long namespaceGeneration) {
    this(existing, namespaceGeneration, BudgetLimits.defaults());
  }
  public UnifiedIMapService(BoundedIMapApi.Bounds bounds, long namespaceGeneration, BudgetLimits limits) {
    this(new BoundedIMapApi(bounds), namespaceGeneration, limits);
  }
  public UnifiedIMapService(BoundedIMapApi existing, long namespaceGeneration, BudgetLimits limits) {
    if (namespaceGeneration <= 0) invalid("namespace generation must be positive");
    map = Objects.requireNonNull(existing);
    this.bounds = map.serviceBounds();
    this.limits = Objects.requireNonNull(limits);
    if (bounds.maxPageItems() > 256) invalid("page bound must not exceed 256");
    generation = namespaceGeneration;
    map.configureService(generation, limits.values(), limits.outcomes(), limits.pending(), limits.maxClients());
    mapId = NEXT_MAP.getAndUpdate(value -> Math.incrementExact(value));
  }

  public Result execute(DomainCommand command) {
    return executeSnapshot(Objects.requireNonNull(command, "command").freeze());
  }
  public long namespaceGeneration() { return generation; }
  public synchronized RequestSnapshot prepareRequest(DomainCommand command) {
    var snapshot = Objects.requireNonNull(command, "command").freeze();
    try { validate(snapshot.toBuilder(), true); }
    catch (RuntimeException failure) { throw IMapDomainException.translate(failure); }
    return snapshot;
  }
  public Result executeSnapshot(RequestSnapshot snapshot) {
    var command = Objects.requireNonNull(snapshot, "snapshot").toBuilder();
    try { synchronized (this) { return executeLocked(command); } }
    finally { wakeListeners(); }
  }

  private Result executeLocked(DomainCommand command) {
    Objects.requireNonNull(command, "command");
    requests++;
    BoundedIMapApi.MutationIdentity identity = null;
    boolean admitted = false;
    try {
      validate(command);
      if (isMutation(command.operation) && command.identity.isPresent()) {
        Object prior = map.recoverService(command.identity.orElseThrow(), request(command));
        if (prior instanceof Result result) return result;
        if (!command.cancelled && command.now < command.deadline) {
          validate(command, true);
          map.preflightAdmission(command.identity.orElseThrow(), request(command), resultReservation(command));
          map.preflightVersions(command.entries.size() + command.keys.size() + 2);
        }
      }
      if (!isMutation(command.operation)) validate(command, true);
      advance(command.now);
      if (!isMutation(command.operation)) return read(command);
      if (Set.of("clear_detailed", "evict_all_detailed", "destroy").contains(command.operation)
          && map.hasExpiryBacklog(now)) throw domain(IMapDomainException.Kind.EXPIRY_BACKLOG, "expiry_backlog");
      identity = command.identity.orElseThrow(() -> domain(IMapDomainException.Kind.INVALID_ARGUMENT,
          "mutation identity required"));
      BytesValue digest = request(command);
      Object prior = map.recoverService(identity, digest);
      if (prior instanceof Result result) return result;
      if (command.cancelled || command.now >= command.deadline) {
        map.abandonService(identity);
        if (command.cancelled) cancellations++;
        return simple("not_applied");
      }
      map.beginService(identity, digest, resultReservation(command));
      admitted = true;
      expireTargets(command);
      List<BoundedIMapApi.RetainedEntry> before = map.retainedEntries();
      Result result = mutate(command);
      if (!map.isDestroyed()) map.completeService(identity, digest, result);
      if (!Set.of("put_all_detailed", "replace_all_detailed", "remove_all_detailed")
          .contains(command.operation)) publishChanges(command.operation, before, map.retainedEntries());
      if (command.loseResponse) return simple("outcome_unknown");
      return result;
    } catch (RuntimeException failure) {
      if (admitted && identity != null) map.abandonService(identity);
      rejections++;
      throw IMapDomainException.translate(failure);
    }
  }

  public void beginOperation(DomainCommand command) {
    beginSnapshot(Objects.requireNonNull(command, "command").freeze());
  }
  public void beginSnapshot(RequestSnapshot snapshot) {
    var command = Objects.requireNonNull(snapshot, "snapshot").toBuilder();
    try { synchronized (this) { beginLocked(command); } }
    finally { wakeListeners(); }
  }
  private void beginLocked(DomainCommand command) {
    try {
      validate(command);
      if (!isMutation(command.operation)) invalid("only mutation identities can be pending");
      var identity = command.identity.orElseThrow(() -> domain(
          IMapDomainException.Kind.INVALID_ARGUMENT, "mutation identity required"));
      if (map.recoverService(identity, request(command)) != null) return;
      validate(command, true);
      map.preflightAdmission(identity, request(command), resultReservation(command));
      advance(command.now);
      map.beginService(identity, request(command), resultReservation(command));
    } catch (RuntimeException failure) { throw IMapDomainException.translate(failure); }
  }

  public synchronized void abandonOperation(BoundedIMapApi.MutationIdentity identity) {
    map.abandonService(identity);
  }
  public Result executeReserved(DomainCommand command) {
    return executeReservedSnapshot(Objects.requireNonNull(command, "command").freeze());
  }
  public Result executeReservedSnapshot(RequestSnapshot snapshot) {
    var command = Objects.requireNonNull(snapshot, "snapshot").toBuilder();
    try { synchronized (this) {
      if (isMutation(command.operation)) {
        var identity = command.identity.orElseThrow(() -> domain(IMapDomainException.Kind.INVALID_ARGUMENT, "identity required"));
        if (!map.servicePending(identity) && map.recoverService(identity, request(command)) == null) {
          throw domain(IMapDomainException.Kind.INVALID_ARGUMENT, "unknown operation reservation");
        }
        if (map.servicePending(identity) && !command.cancelled && command.now < command.deadline) {
          preflightOwnerChanges(command);
          map.preflightAdmission(identity, request(command), resultReservation(command));
        }
      }
    } } catch (RuntimeException failure) { throw IMapDomainException.translate(failure); }
    return executeSnapshot(snapshot);
  }
  static boolean isMutation(String operation) { return MUTATIONS.contains(operation); }
  synchronized Optional<Result> recoverOperation(DomainCommand command) {
    try {
      validate(command);
      Object result = map.recoverService(command.identity.orElseThrow(), request(command));
      return result instanceof Result original ? Optional.of(original) : Optional.empty();
    } catch (RuntimeException failure) { throw IMapDomainException.translate(failure); }
  }
  public synchronized boolean cancelReserved(BoundedIMapApi.MutationIdentity identity) {
    boolean existed = map.servicePending(identity);
    if (existed) { map.abandonService(identity); cancellations++; }
    return existed;
  }

  public synchronized OperationStatus queryOperationOutcome(BoundedIMapApi.MutationIdentity identity) {
    Objects.requireNonNull(identity);
    Object result = map.serviceOutcome(identity);
    if (result instanceof Result retained) return new OperationStatus("completed", Optional.of(retained));
    if (result instanceof BoundedIMapApi.LegacyOutcome legacy) {
      Result converted;
      if (legacy.items().isEmpty()) {
        String kind = switch (legacy.outcome().status()) {
          case APPLIED -> "applied"; case NOT_APPLIED -> "not_applied"; case OUTCOME_UNKNOWN -> "outcome_unknown";
        };
        converted = new Result(kind, Optional.empty(), legacy.outcome().version(), OptionalLong.empty(),
            List.of(), 0, legacy.outcome().version().isPresent(), Optional.empty());
      } else {
        List<Item> items = legacy.items().stream().map(item -> new Item(item.index(), item.key(),
            item.replaced() ? "replaced" : "absent", Optional.empty(), item.version(), OptionalLong.empty())).toList();
        converted = items("bulk", items);
      }
      return new OperationStatus("completed", Optional.of(converted));
    }
    return new OperationStatus(map.servicePending(identity) ? "pending" : "unknown", Optional.empty());
  }

  public synchronized RetirementReceipt retireOutcomes(String client, long namespaceGeneration, long through) {
    try { return new RetirementReceipt(client, namespaceGeneration, through, map.retireOutcomes(client, namespaceGeneration, through)); }
    catch (RuntimeException failure) { throw IMapDomainException.translate(failure); }
  }
  public synchronized List<OutcomeItem> queryOutcomes(List<BoundedIMapApi.MutationIdentity> identities, long maxBytes) {
    if (identities.isEmpty() || identities.size() > limits.maxBatch() || maxBytes <= 0 || maxBytes > limits.queryBytes()) invalid("invalid outcome batch bound");
    long bytes = 0;
    for (var identity : identities) {
      Objects.requireNonNull(identity);
      bytes += 64L + identity.client().getBytes(java.nio.charset.StandardCharsets.UTF_8).length
          + map.serviceQueryResultBytes(identity);
      budget(bytes, maxBytes);
    }
    var result = new ArrayList<OutcomeItem>();
    for (int index = 0; index < identities.size(); index++) result.add(new OutcomeItem(index, identities.get(index), queryOperationOutcome(identities.get(index))));
    return List.copyOf(result);
  }
  private long entryViewsInputBytes(List<BytesValue> keys, long maxBytes) {
    if (map.isDestroyed()) throw domain(IMapDomainException.Kind.MAP_DESTROYED, "map is destroyed");
    if (keys.isEmpty() || keys.size() > limits.maxBatch() || maxBytes <= 0 || maxBytes > Math.min(limits.queryBytes(), bounds.maxResponseBytes())) invalid("entry view batch bound");
    long bytes = 0;
    for (var key : keys) { Objects.requireNonNull(key); if (key.size() > bounds.maxKeyBytes()) throw domain(IMapDomainException.Kind.KEY_TOO_LARGE, "key bound"); bytes = Math.addExact(bytes, 16L + key.size()); }
    if (bytes > BoundedIMapApi.MAX_REQUEST_BYTES) throw domain(IMapDomainException.Kind.REQUEST_TOO_LARGE, "request bytes");
    return bytes;
  }
  public List<VersionedEntryView> entryViews(List<BytesValue> input, long time, long maxBytes) {
    try { synchronized (this) { synchronized (map) {
      entryViewsInputBytes(input, maxBytes); checkTime(time);
      var keys = List.copyOf(input); long bytes = 0;
      for (var key : keys) { var entry = map.serviceEntryView(key, time); bytes = Math.addExact(bytes, 64L + key.size() + entry.map(value -> 24L + value.value().size()).orElse(0L)); budget(bytes, maxBytes); }
      advance(time);
      var result = new ArrayList<VersionedEntryView>(keys.size());
      for (int index = 0; index < keys.size(); index++) { var key = keys.get(index); var entry = map.serviceEntryView(key, time); result.add(new VersionedEntryView(index, key, entry.map(BoundedIMapApi.EntryView::value), entry.map(this::token), entry.isPresent() ? entry.orElseThrow().remainingTtl() : OptionalLong.empty())); }
      return List.copyOf(result);
    } } } catch (RuntimeException failure) { throw IMapDomainException.translate(failure); }
    finally { wakeListeners(); }
  }
  public synchronized ResourceUsage resourceUsage() {
    return new ResourceUsage(map.serviceValueOwnerBytes(), map.serviceOutcomeOwnerBytes(),
        map.servicePendingOwnerBytes(), listenerBytes(), streamBytes, resyncBytes);
  }
  BoundedIMapApi.Bounds semanticBounds() { return bounds; }
  /** Each entry has its own conditional decision; this receipt is not a transaction. */
  private void validateVersionBulk(VersionAction action, List<VersionInput> entries, long time) {
    Objects.requireNonNull(action);
    if (entries.isEmpty() || entries.size() > limits.maxBatch()) invalid("invalid version batch size");
    var keys = new HashSet<BytesValue>(); long requestBytes = 0, responseBytes = 0;
    for (var entry : entries) {
      if (!keys.add(entry.key())) invalid("duplicate key");
      if (entry.key().size() > bounds.maxKeyBytes()) throw domain(IMapDomainException.Kind.KEY_TOO_LARGE, "key bound");
      if ((action == VersionAction.REPLACE) != entry.value().isPresent()) invalid("version batch value shape");
      if (entry.value().isPresent() && entry.value().orElseThrow().size() > bounds.maxValueBytes()) throw domain(IMapDomainException.Kind.VALUE_TOO_LARGE, "value bound");
      if (action == VersionAction.SET_TTL && entry.ttl().kind() == TtlDirective.Kind.PRESERVE) invalid("set TTL requires explicit expiry");
      if (entry.ttl().kind() == TtlDirective.Kind.EXPIRE_AFTER) {
        try { Math.addExact(time, entry.ttl().ticks()); } catch (ArithmeticException overflow) { invalid("expiry overflow"); }
      }
      requestBytes += 32L + entry.key().size() + entry.value().map(BytesValue::size).orElse(0);
      responseBytes += 17L + entry.key().size();
    }
    if (requestBytes > BoundedIMapApi.MAX_REQUEST_BYTES) throw domain(IMapDomainException.Kind.REQUEST_TOO_LARGE, "request bytes");
    if (responseBytes > bounds.maxResponseBytes()) throw domain(IMapDomainException.Kind.RESPONSE_TOO_LARGE, "response bytes");
  }
  public Result versionBulk(VersionAction action, List<VersionInput> input,
      BoundedIMapApi.MutationIdentity identity, long time, long deadline, boolean loseResponse) {
    try { synchronized (this) {
      requests++;
      Objects.requireNonNull(action); Objects.requireNonNull(identity);
      List<VersionInput> entries = List.copyOf(input);
      validateVersionBulk(action, entries, time);
      BytesValue digest = BoundedIMapApi.fingerprint("versionBulk|" + action + "|" + entries);
      Object prior = map.recoverService(identity, digest);
      if (prior instanceof Result result) return result;
      checkTime(time);
      if (time >= deadline) return simple("not_applied");
      var existing = new HashMap<BytesValue, BoundedIMapApi.RetainedEntry>();
      map.liveEntries(time).forEach(entry -> existing.put(entry.key(), entry));
      var changes = new java.util.LinkedHashMap<BytesValue, Optional<BytesValue>>();
      for (var entry : entries) {
        var old = existing.get(entry.key());
        if (old != null && entry.token().equals(new VersionToken(generation, mapId, old.version()))) {
          changes.put(entry.key(), action == VersionAction.REMOVE ? Optional.empty()
              : action == VersionAction.REPLACE ? entry.value() : Optional.of(old.value()));
        }
      }
      map.preflightValues(changes);
      long reservation = 64 + entries.stream().mapToLong(entry -> 64L + entry.key().size()).sum();
      map.preflightAdmission(identity, digest, reservation);
      map.preflightVersions(changes.size() + 1);
      advance(time); map.beginService(identity, digest, reservation);
      for (var entry : entries) map.expireTarget(entry.key(), time).ifPresent(old -> emit("expired", old.key(), Optional.of(old.value()), Optional.empty(), map.revision()));
      var receipts = new ArrayList<Item>();
      for (int index = 0; index < entries.size(); index++) {
        var entry = entries.get(index); var old = map.serviceEntryView(entry.key(), time);
        String kind = old.isEmpty() ? "absent" : !entry.token().equals(token(old.orElseThrow())) ? "mismatch"
            : action == VersionAction.REMOVE ? "removed" : "replaced";
        OptionalLong version = old.isPresent() ? OptionalLong.of(old.orElseThrow().version()) : OptionalLong.empty();
        OptionalLong remaining = old.isPresent() ? old.orElseThrow().remainingTtl() : OptionalLong.empty();
        if (kind.equals("removed")) {
          map.evict(entry.key(), time); version = OptionalLong.of(map.revision()); remaining = OptionalLong.empty();
          emit("removed", entry.key(), old.map(BoundedIMapApi.EntryView::value), Optional.empty(), map.revision());
        } else if (kind.equals("replaced")) {
          BytesValue value = action == VersionAction.REPLACE ? entry.value().orElseThrow() : old.orElseThrow().value();
          map.put(entry.key(), value, expiry(entry.ttl(), old), time);
          var changed = map.getEntryView(entry.key(), time).orElseThrow();
          version = OptionalLong.of(changed.version()); remaining = changed.remainingTtl();
          emit("updated", entry.key(), old.map(BoundedIMapApi.EntryView::value), Optional.of(value), map.revision());
        }
        receipts.add(new Item(index, entry.key(), kind, Optional.empty(), version, remaining));
      }
      Result result = items("bulk", receipts); map.completeService(identity, digest, result);
      return loseResponse ? simple("outcome_unknown") : result;
    } } catch (RuntimeException failure) { synchronized (this) { rejections++; } throw IMapDomainException.translate(failure); }
    finally { wakeListeners(); }
  }
  public ExpiryProgress expireStep(long time, int maxWork) {
    try { synchronized (this) {
      if (maxWork <= 0 || maxWork > bounds.maxScanItems()) invalid("invalid expiry work bound");
      if (map.isDestroyed()) throw domain(IMapDomainException.Kind.MAP_DESTROYED, "map is destroyed");
      return expireLocked(time, maxWork);
    } }
    catch (RuntimeException failure) { throw IMapDomainException.translate(failure); }
    finally { wakeListeners(); }
  }
  private ExpiryProgress expireLocked(long time, int maxWork) {
    checkTime(time);
    var batch = map.expireStep(time, maxWork); now = time;
    for (var entry : batch.expired()) emit("expired", entry.key(), Optional.of(entry.value()), Optional.empty(), map.revision());
    return new ExpiryProgress(batch.work(), batch.expired().size(), batch.remaining());
  }
  public final class SnapshotLease implements AutoCloseable {
    private final List<ResyncEntry> entries;
    private final long watermark, bytes;
    private final long namespaceGeneration, incarnation, revision;
    private final boolean includeValue;
    private boolean closed;
    private SnapshotLease(List<ResyncEntry> entries, long watermark, long bytes,
        long namespaceGeneration, long incarnation, long revision, boolean includeValue) {
      this.entries = List.copyOf(entries); this.watermark = watermark; this.bytes = bytes;
      this.namespaceGeneration = namespaceGeneration; this.incarnation = incarnation; this.revision = revision; this.includeValue = includeValue;
      resyncLeases++;
    }
    public List<ResyncEntry> entries() { return entries; }
    public long watermark() { return watermark; }
    public long namespaceGeneration() { return namespaceGeneration; }
    public long mapId() { return incarnation; }
    public long revision() { return revision; }
    public boolean includeValue() { return includeValue; }
    @Override public void close() { synchronized (UnifiedIMapService.this) { if (!closed) { resyncBytes -= bytes; resyncLeases--; closed = true; } } }
  }
  public SnapshotLease snapshotListener(long id, int maxItems, long maxBytes, long time) {
    try { synchronized (this) { return snapshotListenerLocked(id, maxItems, maxBytes, time); } }
    catch (RuntimeException failure) { throw IMapDomainException.translate(failure); }
    finally { wakeListeners(); }
  }
  private SnapshotLease snapshotListenerLocked(long id, int maxItems, long maxBytes, long time) {
      if (resyncLeases >= bounds.maxEntries()) throw domain(IMapDomainException.Kind.CAPACITY, "snapshot lease capacity");
      if (map.isDestroyed()) throw domain(IMapDomainException.Kind.MAP_DESTROYED, "map is destroyed");
      ListenerState state = listeners.get(id);
      if (state == null) throw domain(IMapDomainException.Kind.UNKNOWN_LISTENER, "unknown listener");
      if (state.terminal || maxItems <= 0 || maxItems > limits.maxBatch() || maxBytes <= 0) invalid("invalid listener snapshot bounds");
      checkTime(time);
      long nextEpoch;
      try { nextEpoch = Math.addExact(state.deliveryEpoch, 1); }
      catch (ArithmeticException overflow) { throw domain(IMapDomainException.Kind.VERSION_EXHAUSTED, "listener delivery epoch exhausted"); }
      var entries = new ArrayList<ResyncEntry>(); long bytes = 0;
      for (var entry : map.liveEntries(time)) {
        if (!matches(state.options, entry.key())) continue;
        if (state.options.includeValue() && entry.value().size() > state.options.maxValueBytes()) {
          throw domain(IMapDomainException.Kind.AGGREGATE_BYTE_BUDGET, "snapshot value bound");
        }
        bytes += 32L + entry.key().size() + (state.options.includeValue() ? entry.value().size() : 0);
        entries.add(new ResyncEntry(entry.key(), state.options.includeValue() ? Optional.of(entry.value()) : Optional.empty(),
            entry.version(), entry.expiresAt() == null ? OptionalLong.empty() : OptionalLong.of(entry.expiresAt() - time)));
      }
      budget(entries.size(), maxItems); budget(bytes, maxBytes);
      budget(resyncBytes + bytes, limits.resync());
      advance(time);
      resyncBytes += bytes;
      state.queue.clear(); state.gap = false; state.deliveryEpoch = nextEpoch;
      return new SnapshotLease(entries, watermark, bytes, generation, mapId, map.revision(), state.options.includeValue());
  }

  synchronized long requestOwnedBytes(RequestSnapshot request) {
    long bytes = 128L + request.operation().getBytes(StandardCharsets.UTF_8).length + request.key().size() + bounds.maxResponseBytes();
    bytes += request.value().map(BytesValue::size).orElse(0);
    bytes += request.expected().map(BytesValue::size).orElse(0);
    for (var key : request.keys()) bytes += 16L + key.size();
    for (var entry : request.entries()) bytes += 32L + entry.key().size() + entry.value().size();
    return bytes + request.identity().map(BoundedIMapApi::identityCost).orElse(0L);
  }
  synchronized void reserveAsyncBytes(long bytes) { budget(Math.addExact(streamBytes, bytes), limits.streams()); streamBytes += bytes; }
  synchronized void releaseAsyncBytes(long bytes) { streamBytes -= bytes; }
  synchronized long helperOwnedBytes(IMapLocalHelper.Command<?> command) {
    long bytes = 128 + helperReservation(command);
    if (command instanceof IMapLocalHelper.EntryViews request) bytes += entryViewsInputBytes(request.keys(), request.maxBytes()) + request.maxBytes();
    if (command instanceof IMapLocalHelper.VersionBulk request) for (var entry : request.entries()) {
      bytes += 64L + entry.key().size() + entry.value().map(BytesValue::size).orElse(0);
    }
    if (command instanceof IMapLocalHelper.QueryOutcomes request) {
      if (request.maxBytes() > limits.queryBytes()) invalid("query helper byte bound");
      bytes += request.maxBytes();
      for (var identity : request.identities()) bytes += BoundedIMapApi.identityCost(identity);
    }
    if (command instanceof IMapLocalHelper.Checkpoint) bytes += 256 + map.serviceValueOwnerBytes() + map.serviceOutcomeOwnerBytes();
    if (command instanceof IMapLocalHelper.RetireOutcomes request) bytes += request.client().getBytes(StandardCharsets.UTF_8).length;
    return bytes + IMapLocalHelper.identity(command).map(BoundedIMapApi::identityCost).orElse(0L);
  }
  /** Pure helper shape/budget validation: no clock advance, reservation or result lease. */
  public synchronized void prepareHelperRequest(IMapLocalHelper.Command<?> command, long time) {
    Objects.requireNonNull(command);
    if (map.isDestroyed()) throw domain(IMapDomainException.Kind.MAP_DESTROYED, "map is destroyed");
    helperOwnedBytes(command);
    if (command instanceof IMapLocalHelper.VersionBulk request) validateVersionBulk(request.action(), request.entries(), time);
    if (command instanceof IMapLocalHelper.QueryOutcomes request) {
      if (request.identities().size() > limits.maxBatch()) invalid("outcome helper batch bound");
      for (var identity : request.identities()) map.serviceQueryResultBytes(identity);
    }
    if (command instanceof IMapLocalHelper.ExpireStep request && request.maxWork() > bounds.maxScanItems()) invalid("expiry helper work bound");
    if (command instanceof IMapLocalHelper.RetireOutcomes request) {
      if (request.generation() != generation || request.client().isBlank()
          || request.client().getBytes(StandardCharsets.UTF_8).length > 128) invalid("retirement helper identity");
    }
  }
  private long helperReservation(IMapLocalHelper.Command<?> command) {
    if (command instanceof IMapLocalHelper.VersionBulk request) return 64 + request.entries().stream().mapToLong(entry -> 64L + entry.key().size()).sum();
    if (command instanceof IMapLocalHelper.ListenerSnapshot request) {
      if (request.maxBytes() > limits.resync() || request.maxItems() > limits.maxBatch()) invalid("snapshot helper bound");
      return Math.addExact(64L + 32L * request.maxItems(), request.maxBytes());
    }
    return 64;
  }
  private BytesValue helperDigest(IMapLocalHelper.Command<?> command) {
    if (command instanceof IMapLocalHelper.VersionBulk request) return BoundedIMapApi.fingerprint("versionBulk|" + request.action() + "|" + request.entries());
    if (command instanceof IMapLocalHelper.ListenerSnapshot request) return BoundedIMapApi.fingerprint("helper_snapshot|" + request.listenerId() + "|" + request.maxItems() + "|" + request.maxBytes());
    if (command instanceof IMapLocalHelper.RetireOutcomes request) return BoundedIMapApi.fingerprint("helper_retire|" + request.client().getBytes(StandardCharsets.UTF_8).length + ":" + request.client() + "|" + request.generation() + "|" + request.through());
    var request = (IMapLocalHelper.ExpireStep) command;
    return BoundedIMapApi.fingerprint("helper_expiry|" + request.maxWork());
  }
  synchronized void beginLocalHelper(IMapLocalHelper.Command<?> command, long time) {
    var identity = IMapLocalHelper.identity(command);
    if (identity.isEmpty()) return;
    if (map.recoverService(identity.orElseThrow(), helperDigest(command)) instanceof Result) return;
    checkTime(time);
    map.beginService(identity.orElseThrow(), helperDigest(command), helperReservation(command));
  }
  synchronized <T> Optional<T> recoverLocalHelper(IMapLocalHelper.Command<T> command) {
    var identity = IMapLocalHelper.identity(command);
    if (identity.isEmpty()) return Optional.empty();
    var digest = helperDigest(command); var result = map.recoverService(identity.orElseThrow(), digest);
    if (map.servicePending(identity.orElseThrow())) map.preflightAdmission(identity.orElseThrow(), digest, helperReservation(command));
    return result instanceof Result receipt ? Optional.of(helperValue(command, receipt)) : Optional.empty();
  }
  <T> T executeLocalHelper(IMapLocalHelper.Command<T> command, long time) {
    if (command instanceof IMapLocalHelper.VersionBulk request) {
      @SuppressWarnings("unchecked") T result = (T) versionBulk(request.action(), request.entries(), request.identity(), time, request.deadline(), request.loseResponse());
      return result;
    }
    try { synchronized (this) {
      var prior = recoverLocalHelper(command); if (prior.isPresent()) return prior.orElseThrow();
      if (command instanceof IMapLocalHelper.EntryViews request) {
        @SuppressWarnings("unchecked") T result = (T) entryViews(request.keys(), time, request.maxBytes()); return result;
      }
      if (command instanceof IMapLocalHelper.QueryOutcomes request) {
        @SuppressWarnings("unchecked") T result = (T) queryOutcomes(request.identities(), request.maxBytes()); return result;
      }
      if (command instanceof IMapLocalHelper.Checkpoint) {
        @SuppressWarnings("unchecked") T result = (T) checkpoint(); return result;
      }
      var identity = IMapLocalHelper.identity(command).orElseThrow(); var digest = helperDigest(command);
      if (!map.servicePending(identity)) invalid("unknown helper reservation");
      map.preflightAdmission(identity, digest, helperReservation(command));
      Result receipt;
      if (command instanceof IMapLocalHelper.RetireOutcomes request) {
        var retired = retireOutcomes(request.client(), request.generation(), request.through()); receipt = numbered("helper_retired", retired.removed(), false);
      } else if (command instanceof IMapLocalHelper.ExpireStep request) {
        if (request.maxWork() > bounds.maxScanItems()) invalid("expiry helper work bound");
        var expired = expireLocked(time, request.maxWork()); receipt = new Result("helper_expiry", Optional.empty(), OptionalLong.of(expired.expired()), OptionalLong.empty(), List.of(), expired.work(), expired.remaining(), Optional.empty());
      } else {
        var request = (IMapLocalHelper.ListenerSnapshot) command;
        try (var snapshot = snapshotListenerLocked(request.listenerId(), request.maxItems(), request.maxBytes(), time)) {
          var items = new ArrayList<Item>();
          for (var entry : snapshot.entries()) items.add(new Item(items.size(), entry.key(), "present", entry.value(), OptionalLong.of(entry.version()), entry.remainingTtl()));
          receipt = new Result("helper_listener_snapshot", Optional.empty(), OptionalLong.empty(), OptionalLong.empty(), items, snapshot.watermark(), snapshot.includeValue(), Optional.of(new VersionToken(snapshot.namespaceGeneration(), snapshot.mapId(), snapshot.revision())));
        }
      }
      map.completeService(identity, digest, receipt); return helperValue(command, receipt);
    } } catch (RuntimeException failure) { throw IMapDomainException.translate(failure); }
    finally { wakeListeners(); }
  }
  @SuppressWarnings("unchecked")
  private <T> T helperValue(IMapLocalHelper.Command<T> command, Result receipt) {
    if (command instanceof IMapLocalHelper.VersionBulk) return (T) receipt;
    if (command instanceof IMapLocalHelper.RetireOutcomes request) return (T) new RetirementReceipt(request.client(), request.generation(), request.through(), Math.toIntExact(receipt.number()));
    if (command instanceof IMapLocalHelper.ExpireStep) return (T) new ExpiryProgress(Math.toIntExact(receipt.number()), Math.toIntExact(receipt.version().orElseThrow()), receipt.flag());
    if (resyncLeases >= bounds.maxEntries()) throw domain(IMapDomainException.Kind.CAPACITY, "snapshot lease capacity");
    var entries = receipt.items().stream().map(item -> new ResyncEntry(item.key(), item.value(), item.version().orElseThrow(), item.remainingTtl())).toList();
    long bytes = entries.stream().mapToLong(entry -> 32L + entry.key().size() + entry.value().map(BytesValue::size).orElse(0)).sum();
    budget(resyncBytes + bytes, limits.resync()); resyncBytes += bytes;
    var token = receipt.token().orElseThrow();
    return (T) new SnapshotLease(entries, receipt.number(), bytes, token.namespaceGeneration(), token.mapId(), token.version(), receipt.flag());
  }
  public static final class RuntimeCheckpoint implements AutoCloseable {
    private final UnifiedIMapService owner;
    private final BoundedIMapApi snapshot;
    private final BudgetLimits limits;
    private final long generation, now, watermark, bytes;
    private boolean closed;
    private RuntimeCheckpoint(UnifiedIMapService owner, BoundedIMapApi snapshot, long bytes) {
      this.owner = owner; this.snapshot = snapshot; this.bytes = bytes;
      limits = owner.limits; generation = owner.generation; now = owner.now; watermark = owner.watermark;
      owner.resyncLeases++;
    }
    @Override public void close() { synchronized (owner) { if (!closed) { owner.resyncBytes -= bytes; owner.resyncLeases--; closed = true; } } }
  }
  public synchronized RuntimeCheckpoint checkpoint() {
    if (map.servicePendingCount() != 0 || !listeners.isEmpty() || activeStreams != 0 || resyncLeases != 0) throw domain(IMapDomainException.Kind.CHECKPOINT_BUSY, "checkpoint_busy");
    long bytes = 256 + map.serviceValueOwnerBytes() + map.serviceOutcomeOwnerBytes();
    budget(resyncBytes + bytes, limits.resync());
    var snapshot = map.checkpointCopy(); resyncBytes += bytes;
    return new RuntimeCheckpoint(this, snapshot, bytes);
  }
  public static UnifiedIMapService restore(RuntimeCheckpoint checkpoint) {
    synchronized (checkpoint.owner) {
      if (checkpoint.closed) invalid("checkpoint is closed");
      var restored = new UnifiedIMapService(checkpoint.snapshot.checkpointCopy(), checkpoint.generation, checkpoint.limits);
      restored.now = checkpoint.now; restored.watermark = checkpoint.watermark;
      checkpoint.close(); return restored;
    }
  }

  public List<SnapshotEntry> snapshot(long time) {
    List<SnapshotEntry> result;
    synchronized (this) {
      advance(time);
      result = map.liveEntries(time).stream().map(entry -> new SnapshotEntry(entry.key(), entry.value(),
          entry.version(), ttl(entry.expiresAt()))).toList();
    }
    wakeListeners();
    return result;
  }
  public synchronized List<SnapshotEntry> diagnosticSnapshot(long time) {
    return map.liveEntries(time).stream().map(entry -> new SnapshotEntry(entry.key(), entry.value(),
        entry.version(), entry.expiresAt() == null ? OptionalLong.empty() : OptionalLong.of(entry.expiresAt() - time))).toList();
  }

  public Statistics stats(long time) {
    Statistics result;
    synchronized (this) {
      advance(time);
      result = new Statistics(map.liveEntries(now).size(), map.serviceValueBytes(),
        map.serviceRetainedCount(), listeners.values().stream().filter(state -> !state.terminal).count(),
        listeners.values().stream().mapToLong(state -> state.queue.size()).sum(),
        map.servicePendingCount(), rejections, cancellations, requests, activeStreams, activeCursors);
    }
    wakeListeners();
    return result;
  }
  public synchronized Statistics diagnosticStats(long time) {
    var live = map.liveEntries(time);
    return new Statistics(live.size(), live.stream().mapToLong(entry -> entry.value().size()).sum(),
        map.serviceRetainedCount(), listeners.values().stream().filter(state -> !state.terminal).count(),
        listeners.values().stream().mapToLong(state -> state.queue.size()).sum(), map.servicePendingCount(),
        rejections, cancellations, requests, activeStreams, activeCursors);
  }

  public synchronized ListenerHandle registerListener(long id, ListenerOptions options) {
    if (map.isDestroyed()) throw domain(IMapDomainException.Kind.MAP_DESTROYED, "map is destroyed");
    if (id <= 0 || listeners.containsKey(id)) invalid("invalid listener registration");
    if (options.keyPrefix().size() > bounds.maxKeyBytes()
        || options.exactKey().map(BytesValue::size).orElse(0) > bounds.maxKeyBytes()) throw domain(IMapDomainException.Kind.INVALID_ARGUMENT, "listener filter bound");
    if (listeners.size() >= bounds.maxEntries() || options.queueCapacity() > bounds.maxPageItems()
        || options.maxValueBytes() > bounds.maxResponseBytes()) {
      throw domain(IMapDomainException.Kind.CAPACITY, "listener capacity");
    }
    budget(listenerBytes() + 128 + options.keyPrefix().size() + options.exactKey().map(BytesValue::size).orElse(0), limits.listeners());
    listeners.put(id, new ListenerState(options));
    return new ListenerHandle(id);
  }

  public synchronized List<Event> drainListener(long id) {
    ListenerState state = listeners.get(id);
    if (state == null) return List.of();
    List<Event> events = List.copyOf(state.queue);
    state.queue.clear();
    if (state.terminal) listeners.remove(id);
    return events;
  }

  public boolean removeListener(long id) {
    ListenerState removed;
    synchronized (this) {
      removed = listeners.remove(id);
      if (removed != null) pendingWake.add(removed.wake);
    }
    wakeListeners();
    return removed != null;
  }
  public synchronized void repairListener(long id) {
    ListenerState state = listeners.get(id);
    if (state == null) throw domain(IMapDomainException.Kind.UNKNOWN_LISTENER, "unknown listener");
    state.queue.clear(); state.gap = false; state.deliveryEpoch++;
  }
  /** Package-local fault seam for the development semantic manifest, using the real queue. */
  void signalListenerGap(long id) {
    synchronized (this) {
      ListenerState state = listeners.get(id);
      if (state == null) throw domain(IMapDomainException.Kind.UNKNOWN_LISTENER, "unknown listener");
      watermark++; state.queue.clear(); state.gap = true;
      state.queue.add(new Event("gap", EMPTY, Optional.empty(), Optional.empty(), map.revision(), watermark));
      pendingWake.add(state.wake);
    }
    wakeListeners();
  }

  public final class ListenerHandle implements AutoCloseable {
    private final long id;
    private boolean closed;
    private ListenerHandle(long id) { this.id = id; }
    public long id() { return id; }
    public List<Event> drain() { synchronized (UnifiedIMapService.this) {
      return closed ? List.of() : drainListener(id);
    } }
    @Override public void close() {
      boolean remove;
      synchronized (UnifiedIMapService.this) { remove = !closed; closed = true; }
      if (remove) removeListener(id);
    }
  }

  public IMapFlowPublisher<BoundedIMapApi.ScanItem> pages(BoundedIMapApi.Projection projection,
      int pageItems, LongSupplier clock) {
    synchronized (this) { advance(clock.getAsLong()); }
    wakeListeners();
    BoundedIMapApi.CloseableIterator<BoundedIMapApi.ScanItem> iterator = map.iterator(projection, pageItems, clock);
    try { synchronized (this) { reserveStream(); activeCursors++; } }
    catch (RuntimeException failure) { iterator.close(); throw failure; }
    return new IMapFlowPublisher<>(new IMapFlowPublisher.Source<>() {
      @Override public IMapFlowPublisher.Pull<BoundedIMapApi.ScanItem> poll() {
        try { synchronized (UnifiedIMapService.this) {
            advance(clock.getAsLong());
            return iterator.hasNext() ? IMapFlowPublisher.Pull.item(iterator.next())
                : IMapFlowPublisher.Pull.end();
        } } catch (RuntimeException failure) { throw IMapDomainException.translate(failure); }
        finally { wakeListeners(); }
      }
      @Override public void close() { iterator.close(); synchronized (UnifiedIMapService.this) {
        releaseStream(); activeCursors--;
      } }
    }, pageItems);
  }

  public IMapFlowPublisher<Event> events(ListenerHandle handle) {
    synchronized (this) {
      ListenerState state = listeners.get(handle.id());
      if (state == null || state.streamAttached) invalid("listener handle is closed or already attached");
      reserveStream(); state.streamAttached = true;
    }
    var publisher = new IMapFlowPublisher<Event>(new IMapFlowPublisher.Source<>() {
      final ArrayDeque<Event> pending = new ArrayDeque<>();
      boolean terminalBatch;
      long deliveryEpoch = -1;
      @Override public IMapFlowPublisher.Pull<Event> poll() {
        synchronized (UnifiedIMapService.this) {
          if (handle.closed || (!terminalBatch && !listeners.containsKey(handle.id()))) {
            pending.clear();
            throw domain(IMapDomainException.Kind.UNKNOWN_LISTENER, "listener removed");
          }
          ListenerState current = listeners.get(handle.id());
          if (current != null && current.deliveryEpoch != deliveryEpoch) {
            pending.clear(); deliveryEpoch = current.deliveryEpoch;
          }
          if (pending.isEmpty()) {
            ListenerState state = listeners.get(handle.id());
            terminalBatch = terminalBatch || (state != null && state.terminal);
            if (state != null) {
              deliveryEpoch = state.deliveryEpoch;
              long bytes = 0;
              while (!state.queue.isEmpty() && bytes + eventBytes(state.queue.peekFirst()) <= streamReservation()) {
                Event event = state.queue.removeFirst(); bytes += eventBytes(event); pending.addLast(event);
              }
              if (terminalBatch && state.queue.isEmpty()) listeners.remove(handle.id());
            }
          }
          if (pending.isEmpty()) {
            if (terminalBatch) return IMapFlowPublisher.Pull.end();
            return IMapFlowPublisher.Pull.pending();
          }
          return IMapFlowPublisher.Pull.item(pending.removeFirst());
        }
      }
      @Override public void close() { pending.clear(); handle.close(); synchronized (UnifiedIMapService.this) { releaseStream(); } }
    }, bounds.maxPageItems());
    synchronized (this) {
      ListenerState state = listeners.get(handle.id());
      if (state != null) state.wake = publisher::signal;
    }
    return publisher;
  }

  private Result read(DomainCommand c) {
    if (!Set.of("query_operation_outcome", "local_statistics", "capability_descriptor", "advance")
        .contains(c.operation) && map.isDestroyed()) {
      throw domain(IMapDomainException.Kind.MAP_DESTROYED, "map is destroyed");
    }
    Optional<BoundedIMapApi.EntryView> current = pointRead(c.operation) ? map.getEntryView(c.key, now)
        : Optional.empty();
    return switch (c.operation) {
      case "get", "get_entry_view" -> current.map(view -> point(c.operation.equals("get")
          ? "present" : "entry_view", Optional.of(view.value()), view)).orElse(simple("absent"));
      case "contains_key" -> current.map(view -> flagged(point("present", Optional.empty(), view), true))
          .orElse(simple("absent"));
      case "contains_entry" -> current.map(view -> flagged(point(view.value().equals(required(c.expected))
          ? "present" : "mismatch", Optional.empty(), view), view.value().equals(required(c.expected)))).orElse(simple("absent"));
      case "get_or_default" -> current.map(view -> point("present", Optional.of(view.value()), view))
          .orElse(new Result("absent", Optional.of(required(c.value)), OptionalLong.empty(),
              OptionalLong.empty(), List.of(), 0, false, Optional.empty()));
      case "remaining_ttl" -> current.map(view -> point("present", Optional.empty(), view))
          .orElse(simple("absent"));
      case "size" -> numbered("counted", map.size(now), map.isEmpty(now));
      case "is_empty" -> numbered(map.isEmpty(now) ? "empty" : "non_empty", map.size(now), map.isEmpty(now));
      case "contains_value_bounded" -> {
        BoundedIMapApi.MatchResult match = map.containsValue(required(c.value), c.cursor, c.limit, now);
        yield new Result(switch (match.match()) { case PRESENT -> "present"; case ABSENT -> "absent"; case INCOMPLETE -> "partial"; }, Optional.empty(), OptionalLong.empty(),
            OptionalLong.empty(), List.of(), 0, match.match() == BoundedIMapApi.Match.PRESENT,
            Optional.empty(), match.nextCursor());
      }
      case "key_set_page", "values_page", "entry_set_page" -> {
        BoundedIMapApi.Projection projection = c.operation.equals("key_set_page")
            ? BoundedIMapApi.Projection.KEYS : c.operation.equals("values_page")
            ? BoundedIMapApi.Projection.VALUES : BoundedIMapApi.Projection.ENTRIES;
        var page = map.scan(projection, c.cursor, c.limit, now);
        var items = new ArrayList<Item>();
        for (int index = 0; index < page.items().size(); index++) {
          var item = page.items().get(index);
          items.add(new Item(index, item.key().orElse(EMPTY), "present", item.value(),
              OptionalLong.empty(), OptionalLong.empty()));
        }
        yield new Result(page.complete() ? "page" : "partial", Optional.empty(), OptionalLong.empty(),
            OptionalLong.empty(), items, items.size(), page.complete(), Optional.empty(), page.nextCursor());
      }
      case "get_all" -> {
        List<Item> items = new ArrayList<>();
        for (int index = 0; index < c.keys.size(); index++) {
          BytesValue key = c.keys.get(index);
          Optional<BoundedIMapApi.EntryView> value = map.getEntryView(key, now);
          items.add(item(index, key, value.isPresent() ? "present" : "absent", value));
        }
        yield items("bulk", items);
      }
      case "entry_listener" -> {
        registerListener(c.listenerId, new ListenerOptions(c.includeValue, bounds.maxValueBytes(),
            c.limit, c.key, c.expected, Set.of()));
        yield numbered("listener_added", c.listenerId, true);
      }
      case "remove_entry_listener" -> {
        boolean removed = removeListener(c.listenerId);
        yield numbered(removed ? "listener_removed" : "absent", 0, removed);
      }
      case "capability_descriptor" -> numbered("capabilities", IMapCapabilityDescriptor.unified(bounds).operations().size(), false);
      case "query_operation_outcome" -> {
        OperationStatus status = queryOperationOutcome(c.identity.orElseThrow(() -> domain(
            IMapDomainException.Kind.INVALID_ARGUMENT, "identity required")));
        yield status.result().map(result -> new Result(status.kind(), result.value(), result.version(),
            result.remainingTtl(), result.items(), result.number(), result.flag(), result.token()))
            .orElse(simple(status.kind()));
      }
      case "local_statistics" -> numbered("statistics", stats(now).liveEntries(), map.isDestroyed());
      case "advance" -> simple("advanced");
      default -> throw domain(IMapDomainException.Kind.UNSUPPORTED_OPERATION, "unsupported operation");
    };
  }

  private Result mutate(DomainCommand c) {
    Optional<BoundedIMapApi.EntryView> previous = map.getEntryView(c.key, now);
    String operation = c.operation;
    if (operation.equals("put_all_detailed") || operation.equals("replace_all_detailed")) {
      List<Item> result = new ArrayList<>();
      for (int i = 0; i < c.entries.size(); i++) {
        var entry = c.entries.get(i);
        Optional<BoundedIMapApi.EntryView> old = map.getEntryView(entry.key(), now);
        if (operation.equals("replace_all_detailed") && old.isEmpty()) {
          result.add(item(i, entry.key(), "absent", Optional.empty()));
        } else {
          Long expiry = operation.equals("replace_all_detailed") ? expiry(old)
              : expiry(c.ttl, old);
          map.put(entry.key(), entry.value(), expiry, now);
          emit(old.isPresent() ? "updated" : "added", entry.key(),
              old.map(BoundedIMapApi.EntryView::value), Optional.of(entry.value()), map.revision());
          var changed = map.getEntryView(entry.key(), now).orElseThrow();
          result.add(new Item(i, entry.key(), old.isPresent() ? "replaced" : "inserted", Optional.empty(),
              OptionalLong.of(changed.version()), changed.remainingTtl()));
        }
      }
      return items("bulk", result);
    }
    if (operation.equals("remove_all_detailed")) {
      List<Item> result = new ArrayList<>();
      for (int i = 0; i < c.keys.size(); i++) {
        var old = map.getEntryView(c.keys.get(i), now);
        map.evict(c.keys.get(i), now);
        if (old.isPresent()) emit("removed", c.keys.get(i), old.map(BoundedIMapApi.EntryView::value),
            Optional.empty(), map.revision());
        result.add(new Item(i, c.keys.get(i), old.isPresent() ? "removed" : "absent",
            old.map(BoundedIMapApi.EntryView::value), OptionalLong.empty(), OptionalLong.empty()));
      }
      return items("bulk", result);
    }
    if (Set.of("clear_detailed", "evict_all_detailed", "destroy").contains(operation)) {
      BoundedIMapApi.RemovalReceipt receipt = operation.equals("destroy") ? map.destroy(now)
          : operation.equals("clear_detailed") ? map.clear(now) : map.evictAll(now);
      return new Result(operation.equals("destroy") ? "destroyed" : operation.equals("clear_detailed")
          ? "cleared" : "evicted", Optional.empty(), OptionalLong.empty(),
          OptionalLong.empty(), List.of(), receipt.removed(), false, Optional.empty());
    }
    if (Set.of("replace_if_version", "remove_if_version", "set_ttl_if_version").contains(operation)) {
      if (previous.isEmpty()) return simple("absent");
      if (!token(previous.orElseThrow()).equals(c.token.orElseThrow(() -> domain(
          IMapDomainException.Kind.INVALID_ARGUMENT, "version token required")))) {
        return point("mismatch", Optional.empty(), previous.orElseThrow());
      }
    }
    if (operation.equals("put_if_absent") && previous.isPresent()) {
      return point("present", Optional.of(previous.orElseThrow().value()), previous.orElseThrow());
    }
    if (Set.of("replace_if_present", "conditional_replace", "conditional_remove", "set_ttl")
        .contains(operation) && previous.isEmpty()) return simple("absent");
    if (Set.of("conditional_replace", "conditional_remove").contains(operation)
        && !previous.orElseThrow().value().equals(required(c.expected))) {
      return point("mismatch", Optional.of(previous.orElseThrow().value()), previous.orElseThrow());
    }
    if (Set.of("delete", "get_and_remove", "conditional_remove", "evict", "try_remove",
        "remove_if_version").contains(operation)) {
      boolean changed = map.evict(c.key, now);
      if (operation.equals("try_remove")) return flagged(versioned("applied",
          changed ? OptionalLong.of(map.revision()) : OptionalLong.empty()), changed);
      if (!changed) return simple("absent");
      return new Result(operation.equals("evict") ? "evicted" : "removed",
          operation.equals("get_and_remove") || operation.equals("conditional_remove")
              ? previous.map(BoundedIMapApi.EntryView::value) : Optional.empty(),
          OptionalLong.of(map.revision()), OptionalLong.empty(), List.of(), 0, true, Optional.empty());
    }
    if (Set.of("set_ttl", "set_ttl_if_version").contains(operation)) {
      map.put(c.key, previous.orElseThrow().value(), expiry(c.ttl, previous), now);
      return appliedPoint("replaced", Optional.empty(), map.getEntryView(c.key, now).orElseThrow());
    }
    BytesValue value = required(c.value);
    map.put(c.key, value, expiry(c.ttl, previous), now);
    String kind = Set.of("set", "try_put").contains(operation) ? "applied"
        : previous.isPresent() ? "replaced" : "inserted";
    Optional<BytesValue> returned = Set.of("get_and_put", "conditional_replace", "replace_if_present",
        "replace_if_version").contains(operation) ? previous.map(BoundedIMapApi.EntryView::value) : Optional.empty();
    return appliedPoint(kind, returned, map.getEntryView(c.key, now).orElseThrow());
  }

  private void validate(DomainCommand c) { validate(c, false); }
  private void validate(DomainCommand c, boolean stateful) {
    Objects.requireNonNull(c.key); Objects.requireNonNull(c.ttl);
    if (Set.of("set_ttl", "set_ttl_if_version").contains(c.operation) && c.ttl.kind() == TtlDirective.Kind.PRESERVE) throw domain(IMapDomainException.Kind.INVALID_ARGUMENT, "set TTL requires explicit expiry");
    if (c.key.size() > bounds.maxKeyBytes()) throw domain(IMapDomainException.Kind.KEY_TOO_LARGE, "key bound");
    for (var value : List.of(c.value, c.expected)) {
      if (value.isPresent() && value.orElseThrow().size() > bounds.maxValueBytes()) {
        throw domain(IMapDomainException.Kind.VALUE_TOO_LARGE, "value bound");
      }
    }
    if (c.value.isPresent() && c.value.orElseThrow().size() > bounds.maxResponseBytes()) {
      throw domain(IMapDomainException.Kind.RESPONSE_TOO_LARGE, "response value bound");
    }
    if (c.limit <= 0) invalid("limit must be positive");
    if (c.keys.size() > limits.maxBatch() || c.entries.size() > limits.maxBatch()) throw domain(IMapDomainException.Kind.REQUEST_TOO_LARGE, "request count");
    long requestBytes = 0, responseBytes = 0;
    var seen = new HashSet<BytesValue>();
    var existing = new HashMap<BytesValue, BoundedIMapApi.RetainedEntry>();
    map.retainedEntries().stream().filter(entry -> entry.expiresAt() == null || entry.expiresAt() > c.now)
        .forEach(entry -> existing.put(entry.key(), entry));
    int inserted = 0;
    for (var entry : c.entries) {
      if (!seen.add(entry.key())) invalid("duplicate key");
      if (entry.key().size() > bounds.maxKeyBytes()) throw domain(IMapDomainException.Kind.KEY_TOO_LARGE, "key bound");
      if (entry.value().size() > bounds.maxValueBytes()) throw domain(IMapDomainException.Kind.VALUE_TOO_LARGE, "value bound");
      requestBytes += 8L + entry.key().size() + entry.value().size();
      responseBytes += 17L + entry.key().size();
      if (entry.value().size() > bounds.maxResponseBytes()) throw domain(
          IMapDomainException.Kind.RESPONSE_TOO_LARGE, "response value bound");
      if (c.operation.equals("put_all_detailed")) {
        if (!existing.containsKey(entry.key())) inserted++;
        if (stateful && c.ttl.kind() == TtlDirective.Kind.PRESERVE && !existing.containsKey(entry.key())) {
          throw domain(IMapDomainException.Kind.INVALID_ARGUMENT, "preserve requires existing entry");
        }
      }
    }
    seen.clear();
    for (var key : c.keys) {
      if (!seen.add(key)) throw domain(IMapDomainException.Kind.INVALID_ARGUMENT, "duplicate key");
      if (key.size() > bounds.maxKeyBytes()) throw domain(IMapDomainException.Kind.KEY_TOO_LARGE, "key bound");
      requestBytes += 4L + key.size(); responseBytes += 17L + key.size();
      if (stateful && (c.operation.equals("get_all") || c.operation.equals("remove_all_detailed"))) {
        var old = existing.get(key);
        if (old != null) responseBytes += old.value().size();
      }
    }
    if (requestBytes > BoundedIMapApi.MAX_REQUEST_BYTES) throw domain(IMapDomainException.Kind.REQUEST_TOO_LARGE, "request bytes");
    if (responseBytes > bounds.maxResponseBytes()) throw domain(IMapDomainException.Kind.RESPONSE_TOO_LARGE, "response bytes");
    if (stateful && existing.size() + inserted > bounds.maxEntries()) throw domain(IMapDomainException.Kind.CAPACITY, "entry capacity");
    if (c.ttl.kind() == TtlDirective.Kind.EXPIRE_AFTER) {
      try { Math.addExact(c.now, c.ttl.ticks()); }
      catch (ArithmeticException failure) { invalid("expiry overflow"); }
    }
    if (Set.of("get_all", "remove_all_detailed").contains(c.operation) && c.keys.isEmpty()) invalid("empty keys");
    if (Set.of("put_all_detailed", "replace_all_detailed").contains(c.operation) && c.entries.isEmpty()) invalid("empty entries");
    if (Set.of("put", "set", "get_and_put", "try_put", "put_if_absent", "replace_if_present",
        "conditional_replace", "replace_if_version", "get_or_default", "contains_value_bounded")
        .contains(c.operation)) required(c.value);
    if (Set.of("conditional_replace", "conditional_remove", "contains_entry").contains(c.operation)) required(c.expected);
    if (!stateful) return;
    if (c.ttl.kind() == TtlDirective.Kind.PRESERVE && !existing.containsKey(c.key)
        && Set.of("put", "set", "try_put", "get_and_put", "put_if_absent").contains(c.operation)) invalid("preserve requires existing entry");
    preflightOwnerChanges(c);
  }
  private void preflightOwnerChanges(DomainCommand c) {
    var existing = new HashMap<BytesValue, BoundedIMapApi.RetainedEntry>();
    map.liveEntries(c.now).forEach(entry -> existing.put(entry.key(), entry));
    var changes = new java.util.LinkedHashMap<BytesValue, Optional<BytesValue>>();
    for (var entry : c.entries) {
      if (c.operation.equals("put_all_detailed") || (c.operation.equals("replace_all_detailed") && existing.containsKey(entry.key()))) changes.put(entry.key(), Optional.of(entry.value()));
    }
    var current = existing.get(c.key);
    boolean match = current != null && (!Set.of("conditional_replace", "conditional_remove").contains(c.operation)
        || c.expected.equals(Optional.of(current.value())));
    if (c.operation.endsWith("_if_version")) match = current != null && c.token.isPresent()
        && c.token.orElseThrow().equals(new VersionToken(generation, mapId, current.version()));
    if (Set.of("put", "set", "try_put", "get_and_put").contains(c.operation)
        || (c.operation.equals("put_if_absent") && current == null)
        || (Set.of("replace_if_present", "conditional_replace", "replace_if_version").contains(c.operation) && match)) {
      changes.put(c.key, c.value);
    }
    map.preflightValues(changes);
  }

  private long resultReservation(DomainCommand c) {
    var existing = new HashMap<BytesValue, BoundedIMapApi.RetainedEntry>();
    map.liveEntries(c.now).forEach(entry -> existing.put(entry.key(), entry));
    long bytes = 64;
    for (var entry : c.entries) bytes += 64L + entry.key().size();
    if (c.operation.equals("remove_all_detailed")) for (var key : c.keys) {
      bytes += 64L + key.size() + (existing.containsKey(key) ? existing.get(key).value().size() : 0);
    }
    if (Set.of("put_if_absent", "get_and_put", "replace_if_present", "conditional_replace", "conditional_remove", "get_and_remove", "replace_if_version").contains(c.operation)) {
      var old = existing.get(c.key); if (old != null) bytes += old.value().size();
    }
    return bytes;
  }
  private void expireTargets(DomainCommand command) {
    var keys = new java.util.TreeSet<BytesValue>(); keys.add(command.key);
    keys.addAll(command.keys); command.entries.forEach(entry -> keys.add(entry.key()));
    for (var key : keys) map.expireTarget(key, now).ifPresent(entry -> emit("expired", key,
        Optional.of(entry.value()), Optional.empty(), map.revision()));
  }

  private void advance(long time) {
    checkTime(time);
    expireLocked(time, limits.expiryWork());
  }

  private void publishChanges(String operation, List<BoundedIMapApi.RetainedEntry> before,
      List<BoundedIMapApi.RetainedEntry> after) {
    if (operation.equals("clear_detailed") || operation.equals("destroy")) {
      if (operation.equals("destroy") || !before.isEmpty()) {
        emit(operation.equals("destroy") ? "destroyed" : "cleared", EMPTY, Optional.empty(), Optional.empty(), map.revision());
      }
      if (operation.equals("destroy")) listeners.values().forEach(state -> state.terminal = true);
      return;
    }
    var old = new TreeMap<BytesValue, BoundedIMapApi.RetainedEntry>();
    var current = new TreeMap<BytesValue, BoundedIMapApi.RetainedEntry>();
    before.forEach(value -> old.put(value.key(), value));
    after.forEach(value -> current.put(value.key(), value));
    var keys = new java.util.TreeSet<BytesValue>(); keys.addAll(old.keySet()); keys.addAll(current.keySet());
    for (var key : keys) {
      var previous = old.get(key); var value = current.get(key);
      if (value == null) emit(operation.equals("evict") || operation.equals("evict_all_detailed")
          ? "evicted" : "removed", key, Optional.of(previous.value()), Optional.empty(), map.revision());
      else if (previous == null || previous.version() != value.version()) {
        emit(previous == null ? "added" : "updated", key,
            previous == null ? Optional.empty() : Optional.of(previous.value()), Optional.of(value.value()), value.version());
      }
    }
  }

  private void emit(String kind, BytesValue key, Optional<BytesValue> oldValue,
      Optional<BytesValue> value, long version) {
    watermark++;
    for (var state : listeners.values()) {
      if (state.gap || state.terminal || (!Set.of("cleared", "destroyed").contains(kind) && !matches(state.options, key))
          || (!state.options.kinds().isEmpty() && !state.options.kinds().contains(kind))) continue;
      int bytes = oldValue.map(BytesValue::size).orElse(0) + value.map(BytesValue::size).orElse(0);
      Event event = new Event(kind, key, state.options.includeValue() ? oldValue : Optional.empty(),
          state.options.includeValue() ? value : Optional.empty(), version, watermark);
      if (state.queue.size() >= state.options.queueCapacity()
          || (state.options.includeValue() && bytes > state.options.maxValueBytes())
          || eventBytes(event) > streamReservation()
          || listenerBytes() + eventBytes(event) > limits.listeners()) {
        state.queue.clear(); state.gap = true;
        state.queue.add(new Event("gap", EMPTY, Optional.empty(), Optional.empty(), version, watermark));
      } else state.queue.add(event);
    }
    listeners.values().stream().map(state -> state.wake).forEach(pendingWake::add);
  }

  private void wakeListeners() {
    if (Thread.holdsLock(this)) return;
    List<Runnable> wake;
    synchronized (this) { wake = List.copyOf(pendingWake); pendingWake.clear(); }
    wake.forEach(Runnable::run);
  }

  private BytesValue request(DomainCommand c) {
    return BoundedIMapApi.fingerprint(c.operation + "|" + c.key + "|" + c.value + "|" + c.expected
        + "|" + c.keys + "|" + c.entries + "|" + c.ttl + "|" + c.token);
  }
  private static boolean pointRead(String op) {
    return Set.of("get", "get_entry_view", "contains_key", "contains_entry", "get_or_default",
        "remaining_ttl").contains(op);
  }
  private Long expiry(TtlDirective directive, Optional<BoundedIMapApi.EntryView> previous) {
    return switch (directive.kind()) {
      case ETERNAL -> null;
      case EXPIRE_AFTER -> Math.addExact(now, directive.ticks());
      case PRESERVE -> { if (previous.isEmpty()) invalid("preserve requires existing entry"); yield expiry(previous); }
    };
  }
  private Long expiry(Optional<BoundedIMapApi.EntryView> previous) {
    return previous.orElseThrow().remainingTtl().isPresent()
        ? Math.addExact(now, previous.orElseThrow().remainingTtl().getAsLong()) : null;
  }
  private OptionalLong ttl(Long expiry) { return expiry == null ? OptionalLong.empty() : OptionalLong.of(expiry - now); }
  private VersionToken token(BoundedIMapApi.EntryView view) { return new VersionToken(generation, mapId, view.version()); }
  private Result point(String kind, Optional<BytesValue> value, BoundedIMapApi.EntryView view) {
    return new Result(kind, value, OptionalLong.of(view.version()), view.remainingTtl(), List.of(), 0,
        false, Optional.of(token(view)));
  }
  private static Result simple(String kind) { return new Result(kind, Optional.empty(), OptionalLong.empty(),
      OptionalLong.empty(), List.of(), 0, false, Optional.empty()); }
  private static Result versioned(String kind, OptionalLong version) { return new Result(kind, Optional.empty(),
      version, OptionalLong.empty(), List.of(), 0, false, Optional.empty()); }
  private static Result numbered(String kind, long number, boolean flag) { return new Result(kind,
      Optional.empty(), OptionalLong.empty(), OptionalLong.empty(), List.of(), number, flag, Optional.empty()); }
  private static Result items(String kind, List<Item> items) { return new Result(kind, Optional.empty(),
      OptionalLong.empty(), OptionalLong.empty(), items, items.size(), false, Optional.empty()); }
  private Result appliedPoint(String kind, Optional<BytesValue> value, BoundedIMapApi.EntryView view) {
    Result result = point(kind, value, view);
    return new Result(result.kind(), result.value(), result.version(), result.remainingTtl(),
        result.items(), result.number(), true, result.token());
  }
  private static Result flagged(Result result, boolean flag) {
    return new Result(result.kind(), result.value(), result.version(), result.remainingTtl(), result.items(),
        result.number(), flag, result.token(), result.cursor());
  }
  private void reserveStream() {
    if (activeStreams >= bounds.maxEntries()) throw domain(IMapDomainException.Kind.CAPACITY, "stream capacity");
    budget(streamBytes + streamReservation(), limits.streams());
    streamBytes += streamReservation();
    activeStreams++;
  }
  private void releaseStream() { activeStreams--; streamBytes -= streamReservation(); }
  private long streamReservation() { return 64L + bounds.maxResponseBytes(); }
  private static long eventBytes(Event event) {
    return event.kind().equals("gap") ? 0 : 64L + event.key().size()
        + event.oldValue().map(BytesValue::size).orElse(0) + event.value().map(BytesValue::size).orElse(0);
  }
  private long listenerBytes() {
    return listeners.values().stream().mapToLong(state -> 128L + state.options.keyPrefix().size() + state.options.exactKey().map(BytesValue::size).orElse(0)
        + state.queue.stream().mapToLong(UnifiedIMapService::eventBytes).sum()).sum();
  }
  private static boolean matches(ListenerOptions options, BytesValue key) {
    byte[] prefix = options.keyPrefix().copy(), bytes = key.copy();
    if (prefix.length > bytes.length) return false;
    for (int index = 0; index < prefix.length; index++) if (prefix[index] != bytes[index]) return false;
    return options.exactKey().isEmpty() || options.exactKey().orElseThrow().equals(key);
  }
  private static void budget(long actual, long limit) {
    if (actual > limit) throw domain(IMapDomainException.Kind.AGGREGATE_BYTE_BUDGET, "aggregate_byte_budget");
  }
  private void checkTime(long time) {
    if (time < 0) invalid("logical time must be nonnegative");
    if (time < now) throw domain(IMapDomainException.Kind.CLOCK_REGRESSED, "logical time regressed");
  }
  public static long logicalResultBytes(Result result) {
    return 64L + result.value().map(BytesValue::size).orElse(0)
        + result.items().stream().mapToLong(item -> 64L + item.key().size() + item.value().map(BytesValue::size).orElse(0)).sum();
  }
  private static Item item(int index, BytesValue key, String kind, Optional<BoundedIMapApi.EntryView> view) {
    return new Item(index, key, kind, view.map(BoundedIMapApi.EntryView::value), view.isPresent()
        ? OptionalLong.of(view.orElseThrow().version()) : OptionalLong.empty(), view.isPresent()
        ? view.orElseThrow().remainingTtl() : OptionalLong.empty());
  }
  private static BytesValue required(Optional<BytesValue> value) {
    return value.orElseThrow(() -> domain(IMapDomainException.Kind.INVALID_ARGUMENT, "required value missing"));
  }
  private static IMapDomainException domain(IMapDomainException.Kind kind, String detail) {
    return new IMapDomainException(kind, detail);
  }
  private static void invalid(String detail) { throw domain(IMapDomainException.Kind.INVALID_ARGUMENT, detail); }
}
