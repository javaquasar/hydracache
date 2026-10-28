package io.hydracache.imap.semantic;

import java.util.List;
import java.util.Objects;
import java.util.Optional;

/** Typed, bounded local helper requests. These types allocate no SDK or protocol operation IDs. */
public final class IMapLocalHelper {
  private IMapLocalHelper() {}
  public sealed interface Command<T> permits VersionBulk, QueryOutcomes, ListenerSnapshot, RetireOutcomes, ExpireStep, Checkpoint {
    long deadline();
  }
  public record VersionBulk(UnifiedIMapService.VersionAction action, List<UnifiedIMapService.VersionInput> entries,
      BoundedIMapApi.MutationIdentity identity, long deadline, boolean loseResponse) implements Command<UnifiedIMapService.Result> {
    public VersionBulk { Objects.requireNonNull(action); Objects.requireNonNull(identity); bounded(entries.size()); entries = List.copyOf(entries); }
  }
  public record QueryOutcomes(List<BoundedIMapApi.MutationIdentity> identities, long maxBytes,
      long deadline) implements Command<List<UnifiedIMapService.OutcomeItem>> {
    public QueryOutcomes { bounded(identities.size()); identities = List.copyOf(identities); positive(maxBytes); }
  }
  public record ListenerSnapshot(long listenerId, int maxItems, long maxBytes,
      BoundedIMapApi.MutationIdentity identity, long deadline) implements Command<UnifiedIMapService.SnapshotLease> {
    public ListenerSnapshot { Objects.requireNonNull(identity); bounded(maxItems); positive(maxBytes); }
  }
  public record RetireOutcomes(String client, long generation, long through,
      BoundedIMapApi.MutationIdentity identity, long deadline) implements Command<UnifiedIMapService.RetirementReceipt> {
    public RetireOutcomes { Objects.requireNonNull(client); Objects.requireNonNull(identity); positive(generation); positive(through); }
  }
  public record ExpireStep(int maxWork, BoundedIMapApi.MutationIdentity identity,
      long deadline) implements Command<UnifiedIMapService.ExpiryProgress> {
    public ExpireStep { Objects.requireNonNull(identity); positive(maxWork); }
  }
  public record Checkpoint(long deadline) implements Command<UnifiedIMapService.RuntimeCheckpoint> {}
  public enum Kind { COMPLETED, NOT_APPLIED, OUTCOME_UNKNOWN }
  public record Outcome<T>(Kind kind, Optional<T> value) implements AutoCloseable {
    public Outcome {
      Objects.requireNonNull(kind); Objects.requireNonNull(value);
      if ((kind == Kind.COMPLETED) != value.isPresent()) throw new IllegalArgumentException("helper outcome shape");
    }
    @Override public void close() { value.ifPresent(IMapLocalHelper::closeValue); }
  }
  static Optional<BoundedIMapApi.MutationIdentity> identity(Command<?> command) {
    if (command instanceof VersionBulk request) return Optional.of(request.identity());
    if (command instanceof ListenerSnapshot request) return Optional.of(request.identity());
    if (command instanceof RetireOutcomes request) return Optional.of(request.identity());
    if (command instanceof ExpireStep request) return Optional.of(request.identity());
    return Optional.empty();
  }
  static boolean mutation(Command<?> command) { return identity(command).isPresent(); }
  static void closeValue(Object value) {
    if (value instanceof UnifiedIMapService.SnapshotLease lease) lease.close();
    if (value instanceof UnifiedIMapService.RuntimeCheckpoint checkpoint) checkpoint.close();
  }
  private static void bounded(int count) { if (count <= 0 || count > 256) throw new IllegalArgumentException("local helper batch bound"); }
  private static void positive(long value) { if (value <= 0) throw new IllegalArgumentException("positive local helper bound required"); }
}
