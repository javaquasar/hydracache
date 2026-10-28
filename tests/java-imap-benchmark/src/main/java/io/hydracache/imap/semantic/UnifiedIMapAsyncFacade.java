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

/** Bounded two-phase async adapter over the same unified service and outcome registry. */
public final class UnifiedIMapAsyncFacade {
  private final UnifiedIMapService service;
  private final Executor executor;
  private final LongSupplier clock;
  private final int maxPending;
  private int pending;

  public UnifiedIMapAsyncFacade(UnifiedIMapService service, Executor executor,
      LongSupplier clock, int maxPending) {
    this.service = Objects.requireNonNull(service); this.executor = Objects.requireNonNull(executor);
    this.clock = Objects.requireNonNull(clock);
    if (maxPending <= 0) throw new IllegalArgumentException("pending bound");
    this.maxPending = maxPending;
  }
  public synchronized int pending() { return pending; }

  public Invocation invoke(UnifiedIMapService.DomainCommand input) {
    UnifiedIMapService.DomainCommand command = Objects.requireNonNull(input).copy();
    synchronized (this) {
      if (pending >= maxPending) throw new RejectedExecutionException("pending capacity");
      pending++;
    }
    Invocation invocation = new Invocation(command);
    try {
      executor.execute(() -> {
        synchronized (invocation) {
          if (invocation.cancelled) { release(); return; }
          try {
            Optional<UnifiedIMapService.Result> prior = invocation.recover();
            if (prior.isPresent()) { invocation.result.complete(prior.orElseThrow()); release(); return; }
            command.now(clock.getAsLong());
            if (command.now >= command.deadline) {
              invocation.result.complete(observation("not_applied")); release(); return;
            }
            if (UnifiedIMapService.isMutation(command.operation)) service.beginOperation(command);
            invocation.admitted = true;
          } catch (RuntimeException failure) {
            invocation.result.completeExceptionally(failure); release(); return;
          }
        }
        try {
          executor.execute(() -> {
            try {
              command.now(clock.getAsLong());
              invocation.result.complete(service.executeReserved(command));
            } catch (RuntimeException failure) {
              command.identity.ifPresent(service::abandonOperation);
              invocation.result.completeExceptionally(failure);
            }
            finally { release(); }
          });
        } catch (RuntimeException failure) {
          command.identity.ifPresent(service::abandonOperation);
          invocation.result.completeExceptionally(failure); release();
        }
      });
    } catch (RuntimeException failure) {
      invocation.result.completeExceptionally(failure); release();
    }
    return invocation;
  }

  private synchronized void release() { pending--; }

  public final class Invocation {
    private final UnifiedIMapService.DomainCommand command;
    private final CompletableFuture<UnifiedIMapService.Result> result = new CompletableFuture<>();
    private boolean cancelled, admitted;
    private Invocation(UnifiedIMapService.DomainCommand command) { this.command = command; }
    public CompletionStage<UnifiedIMapService.Result> result() { return result.minimalCompletionStage(); }
    public synchronized boolean cancel() {
      if (result.isDone()) return false;
      cancelled = true;
      try { result.complete(admitted ? observation("outcome_unknown")
          : recover().orElseGet(() -> observation("not_applied"))); }
      catch (RuntimeException failure) { result.completeExceptionally(failure); }
      return true;
    }
    private Optional<UnifiedIMapService.Result> recover() {
      return command.identity.isEmpty() || !UnifiedIMapService.isMutation(command.operation)
          ? Optional.empty() : service.recoverOperation(command);
    }
  }

  private static UnifiedIMapService.Result observation(String kind) {
    return new UnifiedIMapService.Result(kind, Optional.empty(), OptionalLong.empty(),
        OptionalLong.empty(), List.of(), 0, false, Optional.empty());
  }
}
