package io.hydracache.imap.semantic;

import java.util.List;
import java.util.Objects;
import java.util.Optional;
import java.util.OptionalLong;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionStage;
import java.util.concurrent.Executor;
import java.util.concurrent.RejectedExecutionException;
import java.util.function.LongSupplier;
import java.util.function.Function;
import java.util.function.Supplier;

/** Local reference facade with bounded pending invocations and deterministic dispatch seams. */
public final class AsyncIMapFacade {
  private final BoundedIMapApi map;
  private final Executor executor;
  private final LongSupplier clock;
  private final int maxPending;
  private int pending;

  public AsyncIMapFacade(BoundedIMapApi map, Executor executor, LongSupplier clock, int maxPending) {
    this.map = Objects.requireNonNull(map, "map");
    this.executor = Objects.requireNonNull(executor, "executor");
    this.clock = Objects.requireNonNull(clock, "clock");
    if (maxPending <= 0) throw new IllegalArgumentException("maxPending must be positive");
    this.maxPending = maxPending;
  }

  public synchronized int pendingInvocations() { return pending; }

  public CompletionStage<BytesValue> getOrDefault(BytesValue key, BytesValue fallback,
      long deadline) {
    return read(deadline, () -> map.getOrDefault(key, fallback, clock.getAsLong()));
  }

  public CompletionStage<Boolean> containsEntry(BytesValue key, BytesValue expected, long deadline) {
    return read(deadline, () -> map.containsEntry(key, expected, clock.getAsLong()));
  }

  public CompletionStage<Optional<BoundedIMapApi.EntryView>> getEntryView(BytesValue key,
      long deadline) {
    return read(deadline, () -> map.getEntryView(key, clock.getAsLong()));
  }

  public MutationInvocation<BoundedIMapApi.BulkTryOutcome> replaceAllDetailed(
      BoundedIMapApi.MutationIdentity identity,
      List<BoundedIMapApi.ReplaceInput> replacements, long deadline) {
    List<BoundedIMapApi.ReplaceInput> request = List.copyOf(replacements);
    return mutate(identity, deadline,
        operationDeadline -> map.tryReplaceAllDetailed(
            identity, request, operationDeadline, clock.getAsLong(), false),
        status -> new BoundedIMapApi.BulkTryOutcome(
            new BoundedIMapApi.TryOutcome(status, identity, OptionalLong.empty()), List.of()));
  }

  public MutationInvocation<BoundedIMapApi.TryOutcome> set(
      BoundedIMapApi.MutationIdentity identity, BytesValue key,
      BytesValue value, Long expiresAt, long deadline) {
    return tryPut(identity, key, value, expiresAt, deadline);
  }

  public MutationInvocation<BoundedIMapApi.TryOutcome> tryPut(
      BoundedIMapApi.MutationIdentity identity, BytesValue key,
      BytesValue value, Long expiresAt, long deadline) {
    Objects.requireNonNull(key, "key");
    Objects.requireNonNull(value, "value");
    return mutate(identity, deadline,
        operationDeadline -> map.tryPut(
            identity, key, value, expiresAt, operationDeadline, clock.getAsLong(), false),
        status -> new BoundedIMapApi.TryOutcome(status, identity, OptionalLong.empty()));
  }

  public MutationInvocation<BoundedIMapApi.TryOutcome> tryRemove(
      BoundedIMapApi.MutationIdentity identity, BytesValue key,
      long deadline) {
    Objects.requireNonNull(key, "key");
    return mutate(identity, deadline,
        operationDeadline -> map.tryRemove(
            identity, key, operationDeadline, clock.getAsLong(), false),
        status -> new BoundedIMapApi.TryOutcome(status, identity, OptionalLong.empty()));
  }

  public CompletionStage<BoundedIMapApi.ListenerRemoval> removeEntryListener(long registration,
      long deadline) {
    return read(deadline, () -> map.removeEntryListener(registration));
  }

  private <T> CompletionStage<T> read(long deadline, Supplier<T> action) {
    reserve();
    var result = new CompletableFuture<T>();
    try {
      executor.execute(() -> {
        try {
          if (clock.getAsLong() >= deadline) {
            throw new IllegalStateException("deadline exceeded before dispatch");
          }
          result.complete(action.get());
        } catch (RuntimeException failure) {
          result.completeExceptionally(failure);
        } finally {
          release();
        }
      });
    } catch (RuntimeException failure) {
      release();
      result.completeExceptionally(failure);
    }
    return result.minimalCompletionStage();
  }

  private <T> MutationInvocation<T> mutate(BoundedIMapApi.MutationIdentity identity, long deadline,
      Function<Long, T> action, Function<BoundedIMapApi.TryStatus, T> observation) {
    Objects.requireNonNull(identity, "identity");
    reserve();
    MutationInvocation<T> invocation = new MutationInvocation<>(identity, observation,
        () -> action.apply(Long.MIN_VALUE));
    try {
      executor.execute(() -> {
        try {
          synchronized (invocation) {
            if (invocation.cancelled || clock.getAsLong() >= deadline) {
              invocation.completeNotApplied();
              release();
              return;
            }
            invocation.admitted = true;
          }
        } catch (RuntimeException failure) {
          invocation.result.completeExceptionally(failure);
          release();
          return;
        }
        try {
          executor.execute(() -> {
            try {
              T outcome = action.apply(deadline);
              invocation.result.complete(outcome);
            } catch (RuntimeException failure) {
              invocation.result.completeExceptionally(failure);
            } finally {
              release();
            }
          });
        } catch (RuntimeException failure) {
          invocation.result.completeExceptionally(failure);
          release();
        }
      });
    } catch (RuntimeException failure) {
      invocation.result.completeExceptionally(failure);
      release();
    }
    return invocation;
  }

  private synchronized void reserve() {
    if (pending >= maxPending) throw new RejectedExecutionException("pending invocation capacity");
    pending++;
  }

  private synchronized void release() { pending--; }

  /** Cancellation completes observation, while an admitted mutation keeps its original identity. */
  public static final class MutationInvocation<T> {
    private final BoundedIMapApi.MutationIdentity identity;
    private final CompletableFuture<T> result = new CompletableFuture<>();
    private final Function<BoundedIMapApi.TryStatus, T> observation;
    private final Supplier<T> replayOrNotApplied;
    private boolean admitted;
    private boolean cancelled;

    private MutationInvocation(BoundedIMapApi.MutationIdentity identity,
        Function<BoundedIMapApi.TryStatus, T> observation, Supplier<T> replayOrNotApplied) {
      this.identity = identity;
      this.observation = observation;
      this.replayOrNotApplied = replayOrNotApplied;
    }

    public CompletionStage<T> result() {
      return result.minimalCompletionStage();
    }

    public BoundedIMapApi.MutationIdentity identity() { return identity; }

    public synchronized boolean cancel() {
      if (result.isDone()) return false;
      cancelled = true;
      if (admitted) result.complete(observation.apply(BoundedIMapApi.TryStatus.OUTCOME_UNKNOWN));
      else completeNotApplied();
      return true;
    }

    private void completeNotApplied() {
      try {
        result.complete(replayOrNotApplied.get());
      } catch (RuntimeException failure) {
        result.completeExceptionally(failure);
      }
    }
  }
}
