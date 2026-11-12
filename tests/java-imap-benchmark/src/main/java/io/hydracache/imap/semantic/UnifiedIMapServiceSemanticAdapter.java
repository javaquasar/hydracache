package io.hydracache.imap.semantic;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.Map;
import java.util.List;
import java.util.Objects;
import java.util.Optional;
import java.util.OptionalLong;
import java.util.Set;

/** Local-reference SPI bridge. Values, receipts and queued events belong exclusively to the service. */
public final class UnifiedIMapServiceSemanticAdapter implements MapSemanticAdapter {
  private static final Map<Operation, String> OPERATIONS = Map.ofEntries(
      Map.entry(Operation.GET, "get"), Map.entry(Operation.CONTAINS_KEY, "contains_key"),
      Map.entry(Operation.PUT, "put"), Map.entry(Operation.PUT_IF_ABSENT, "put_if_absent"),
      Map.entry(Operation.REPLACE, "conditional_replace"), Map.entry(Operation.REPLACE_IF_PRESENT, "replace_if_present"),
      Map.entry(Operation.GET_AND_PUT, "get_and_put"), Map.entry(Operation.GET_AND_REMOVE, "get_and_remove"),
      Map.entry(Operation.REMOVE_IF_VALUE, "conditional_remove"), Map.entry(Operation.GET_ALL, "get_all"),
      Map.entry(Operation.PUT_ALL, "put_all_detailed"), Map.entry(Operation.REMOVE_ALL, "remove_all_detailed"),
      Map.entry(Operation.SET_TTL, "set_ttl"), Map.entry(Operation.REMAINING_TTL, "remaining_ttl"),
      Map.entry(Operation.LISTENER_GAP, "listener_gap"), Map.entry(Operation.ADVANCE, "advance"));
  private final UnifiedIMapService service;
  private final UnifiedIMapService.ListenerHandle listener;
  private final AdapterIdentity identity;
  private final String client;
  private final boolean dropSwap;
  private long tick, sequence;
  private boolean closed;

  public UnifiedIMapServiceSemanticAdapter(String name, UnifiedIMapService service) { this(name, service, false); }
  UnifiedIMapServiceSemanticAdapter(String name, UnifiedIMapService service, boolean dropSwap) {
    if (name.getBytes(StandardCharsets.UTF_8).length > 96) throw new IllegalArgumentException("adapter identity bound");
    identity = new AdapterIdentity("local-reference", name, "provisional-not-production");
    this.service = Objects.requireNonNull(service); client = "semantic-" + name; this.dropSwap = dropSwap;
    listener = service.registerListener(Long.MAX_VALUE, new UnifiedIMapService.ListenerOptions(true,
        service.semanticBounds().maxResponseBytes(), service.semanticBounds().maxPageItems(), Optional.empty(), Set.of()));
  }
  public static UnifiedIMapServiceSemanticAdapter fresh(String name) {
    return new UnifiedIMapServiceSemanticAdapter(name, new UnifiedIMapService(
        new BoundedIMapApi.Bounds(4096, 65536, 16384, 256, 16384, 1024 * 1024, 256), 1));
  }
  @Override public AdapterIdentity identity() { return identity; }
  @Override public Set<Operation> capabilities() { return OPERATIONS.keySet(); }
  @Override public synchronized OperationOutcome execute(ScenarioStep step) {
    Objects.requireNonNull(step); requireOpen();
    try {
      if (step.operation() == Operation.LISTENER_GAP) { service.signalListenerGap(listener.id()); return OperationOutcome.advanced(); }
      if (step.operation() == Operation.ADVANCE) {
        long target = Math.addExact(tick, step.advanceTicks());
        service.execute(new UnifiedIMapService.DomainCommand("advance").now(target)); tick = target;
        return OperationOutcome.advanced();
      }
      String operation = dropSwap && step.operation() == Operation.GET_AND_PUT ? "get" : OPERATIONS.get(step.operation());
      sequence = Math.incrementExact(sequence);
      var command = new UnifiedIMapService.DomainCommand(operation).now(tick).ttl(step.ttl())
          .identity(new BoundedIMapApi.MutationIdentity(client, sequence));
      if (step.operation() == Operation.PUT_ALL) command.entries(step.entries().stream()
          .map(entry -> new BoundedIMapApi.ReplaceInput(entry.key(), entry.value())).toList());
      else if (step.operation() == Operation.GET_ALL || step.operation() == Operation.REMOVE_ALL) command.keys(step.keys());
      else { command.key(step.key()); if (!step.entries().isEmpty()) command.value(step.value()); }
      step.expected().ifPresent(command::expected);
      var result = service.execute(command);
      if (result.kind().equals("bulk")) return OperationOutcome.bulk(result.items().stream()
          .map(item -> new ItemOutcome(item.index(), item.key(), kind(item.kind()), item.value(), ttl(item.kind(), item.version(), item.remainingTtl()))).toList());
      return OperationOutcome.point(step.operation() == Operation.SET_TTL && result.kind().equals("replaced")
          ? OutcomeKind.PRESENT : kind(result.kind()), result.value(), ttl(result.kind(), result.version(), result.remainingTtl()));
    } catch (IMapDomainException failure) {
      if (failure.kind().equals("invalid_argument") && failure.getMessage().equals("duplicate key")) return OperationOutcome.error(ErrorClass.DUPLICATE_KEY, "duplicate key rejected before mutation");
      return switch (failure.error()) {
        case INVALID_ARGUMENT -> OperationOutcome.error(ErrorClass.INVALID_ARGUMENT, failure.getMessage());
        case KEY_TOO_LARGE, VALUE_TOO_LARGE, REQUEST_TOO_LARGE, RESPONSE_TOO_LARGE, CAPACITY,
            IDEMPOTENCY_CAPACITY_EXHAUSTED, AGGREGATE_BYTE_BUDGET -> OperationOutcome.error(ErrorClass.SIZE, failure.kind());
        default -> OperationOutcome.error(ErrorClass.INTERNAL, failure.kind());
      };
    }
  }
  @Override public synchronized List<MapEvent> drainEvents() {
    requireOpen(); var events = listener.drain();
    if (events.stream().anyMatch(event -> event.kind().equals("gap"))) service.repairListener(listener.id());
    return events.stream().map(event -> new MapEvent(switch (event.kind()) {
      case "added" -> MapEvent.Kind.ADDED; case "updated" -> MapEvent.Kind.UPDATED;
      case "removed", "evicted" -> MapEvent.Kind.REMOVED; case "expired" -> MapEvent.Kind.EXPIRED;
      case "gap" -> MapEvent.Kind.GAP; default -> MapEvent.Kind.INVALIDATED;
    }, event.key(), event.value(), tick)).toList();
  }
  @Override public synchronized StateSnapshot snapshot() {
    requireOpen(); var entries = service.diagnosticSnapshot(tick); MessageDigest digest = sha256("SHA-256");
    for (var entry : entries) {
      update(digest, entry.key().copy()); update(digest, entry.value().copy());
      digest.update((byte) (entry.remainingTtl().isPresent() ? 1 : 0));
      if (entry.remainingTtl().isPresent()) digest.update(ByteBuffer.allocate(8).putLong(Math.addExact(tick, entry.remainingTtl().getAsLong())).array());
    }
    return new StateSnapshot(java.util.HexFormat.of().formatHex(digest.digest()), entries.size(), tick);
  }
  @Override public synchronized void close() { if (!closed) { listener.close(); closed = true; } }
  private void requireOpen() { if (closed) throw new IllegalStateException("semantic adapter is closed"); }
  private static OutcomeKind kind(String kind) { return OutcomeKind.valueOf(kind.toUpperCase(java.util.Locale.ROOT)); }
  private static TtlState ttl(String kind, OptionalLong version, OptionalLong remaining) {
    if (kind.equals("absent") || kind.equals("removed") || version.isEmpty()) return TtlState.absent();
    return remaining.isPresent() ? TtlState.expiring(remaining.getAsLong()) : TtlState.eternal();
  }
  private static void update(MessageDigest digest, byte[] value) { digest.update(ByteBuffer.allocate(4).putInt(value.length).array()); digest.update(value); }
  static MessageDigest sha256(String algorithm) {
    try { return MessageDigest.getInstance(algorithm); }
    catch (NoSuchAlgorithmException failure) { throw new IllegalStateException("required semantic digest unavailable", failure); }
  }
}
