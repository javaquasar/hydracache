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
  private final long maxOwnedBytes;
  private int pending;
  private long ownedBytes;

  public UnifiedIMapAsyncFacade(UnifiedIMapService service, Executor executor,
      LongSupplier clock, int maxPending) {
    this(service, executor, clock, maxPending, 64L * 1024 * 1024);
  }
  public UnifiedIMapAsyncFacade(UnifiedIMapService service, Executor executor,
      LongSupplier clock, int maxPending, long maxOwnedBytes) {
    this.service = Objects.requireNonNull(service); this.executor = Objects.requireNonNull(executor);
    this.clock = Objects.requireNonNull(clock);
    if (maxPending <= 0 || maxOwnedBytes <= 0) throw new IllegalArgumentException("pending/byte bound");
    this.maxPending = maxPending;
    this.maxOwnedBytes = maxOwnedBytes;
  }
  public synchronized int pending() { return pending; }
  public synchronized long ownedBytes() { return ownedBytes; }

  public Invocation invoke(UnifiedIMapService.DomainCommand input) {
    return invokeSnapshot(Objects.requireNonNull(input).freeze());
  }
  public Invocation invokeSnapshot(UnifiedIMapService.RequestSnapshot snapshot) {
    UnifiedIMapService.DomainCommand command = Objects.requireNonNull(snapshot).toBuilder();
    long bytes = service.requestOwnedBytes(snapshot); reserve(bytes);
    Invocation invocation = new Invocation(command, bytes);
    try {
      executor.execute(() -> {
        synchronized (invocation) {
          if (invocation.cancelled) { release(bytes); return; }
          try {
            Optional<UnifiedIMapService.Result> prior = invocation.recover();
            if (prior.isPresent()) { invocation.result.complete(prior.orElseThrow()); release(bytes); return; }
            command.now(clock.getAsLong());
            if (command.now >= command.deadline) {
              invocation.result.complete(observation("not_applied")); release(bytes); return;
            }
            if (UnifiedIMapService.isMutation(command.operation)) service.beginOperation(command);
            invocation.admitted = true;
          } catch (RuntimeException failure) {
            invocation.result.completeExceptionally(failure); release(bytes); return;
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
            finally { release(bytes); }
          });
        } catch (RuntimeException failure) {
          command.identity.ifPresent(service::abandonOperation);
          invocation.result.completeExceptionally(failure); release(bytes);
        }
      });
    } catch (RuntimeException failure) {
      invocation.result.completeExceptionally(failure); release(bytes);
    }
    return invocation;
  }

  private synchronized void reserve(long bytes) {
    if (pending >= maxPending || bytes <= 0 || bytes > maxOwnedBytes - ownedBytes) throw new RejectedExecutionException("pending payload capacity");
    service.reserveAsyncBytes(bytes);
    pending++; ownedBytes += bytes;
  }
  private synchronized void release(long bytes) { pending--; ownedBytes -= bytes; service.releaseAsyncBytes(bytes); }

  public final class Invocation {
    private final UnifiedIMapService.DomainCommand command;
    private final CompletableFuture<UnifiedIMapService.Result> result = new CompletableFuture<>();
    private boolean cancelled, admitted;
    private Invocation(UnifiedIMapService.DomainCommand command, long bytes) { this.command = command; }
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

  public <T> HelperInvocation<T> invokeHelper(IMapLocalHelper.Command<T> command) {
    Objects.requireNonNull(command); long bytes = service.helperOwnedBytes(command); reserve(bytes);
    var invocation = new HelperInvocation<T>(command, bytes);
    try { executor.execute(invocation::admit); }
    catch (RuntimeException failure) { invocation.result.completeExceptionally(failure); release(bytes); }
    return invocation;
  }

  /** Owns accepted request credits until executor work finishes; close also releases an undelivered lease. */
  public final class HelperInvocation<T> implements AutoCloseable {
    private final IMapLocalHelper.Command<T> command;
    private final long bytes;
    private final CompletableFuture<IMapLocalHelper.Outcome<T>> result = new CompletableFuture<>();
    private boolean cancelled, admitted, closed;
    private HelperInvocation(IMapLocalHelper.Command<T> command, long bytes) { this.command = command; this.bytes = bytes; }
    public CompletionStage<IMapLocalHelper.Outcome<T>> result() { return result.minimalCompletionStage(); }
    private void admit() {
      synchronized (this) {
        if (cancelled) { release(bytes); return; }
        try {
          var recovered = service.recoverLocalHelper(command);
          if (recovered.isPresent()) { complete(recovered.orElseThrow()); release(bytes); return; }
          long time = clock.getAsLong();
          if (time >= command.deadline()) { observation(IMapLocalHelper.Kind.NOT_APPLIED); release(bytes); return; }
          service.beginLocalHelper(command, time); admitted = true;
        } catch (RuntimeException failure) { result.completeExceptionally(failure); release(bytes); return; }
      }
      try { executor.execute(this::dispatch); }
      catch (RuntimeException failure) { abandon(); result.completeExceptionally(failure); release(bytes); }
    }
    private void dispatch() {
      try {
        synchronized (this) { if (cancelled && !IMapLocalHelper.mutation(command)) return; }
        long time = clock.getAsLong();
        if (time >= command.deadline()) {
          var recovered = service.recoverLocalHelper(command);
          if (recovered.isPresent()) complete(recovered.orElseThrow());
          else { abandon(); observation(IMapLocalHelper.Kind.NOT_APPLIED); }
        } else complete(service.executeLocalHelper(command, time));
      } catch (RuntimeException failure) { abandon(); result.completeExceptionally(failure); }
      finally { release(bytes); }
    }
    private void abandon() { IMapLocalHelper.identity(command).ifPresent(service::abandonOperation); }
    private void complete(T value) {
      if (!result.complete(new IMapLocalHelper.Outcome<>(IMapLocalHelper.Kind.COMPLETED, Optional.of(value)))) IMapLocalHelper.closeValue(value);
    }
    private void observation(IMapLocalHelper.Kind kind) { result.complete(new IMapLocalHelper.Outcome<>(kind, Optional.empty())); }
    public synchronized boolean cancel() {
      if (result.isDone()) return false;
      cancelled = true;
      try {
        var recovered = admitted ? Optional.<T>empty() : service.recoverLocalHelper(command);
        if (recovered.isPresent()) complete(recovered.orElseThrow());
        else observation(admitted && IMapLocalHelper.mutation(command) ? IMapLocalHelper.Kind.OUTCOME_UNKNOWN : IMapLocalHelper.Kind.NOT_APPLIED);
      } catch (RuntimeException failure) { result.completeExceptionally(failure); }
      return true;
    }
    @Override public synchronized void close() {
      if (!closed) { closed = true; cancel(); result.thenAccept(IMapLocalHelper.Outcome::close); }
    }
  }

  private static UnifiedIMapService.Result observation(String kind) {
    return new UnifiedIMapService.Result(kind, Optional.empty(), OptionalLong.empty(),
        OptionalLong.empty(), List.of(), 0, false, Optional.empty());
  }
}
