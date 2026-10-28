package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.*;

import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import java.util.Set;
import java.util.concurrent.Flow;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.Test;

final class IMapFlowPublisherTest {
  private static final BoundedIMapApi.Bounds BOUNDS = new BoundedIMapApi.Bounds(8, 8, 8, 2, 8, 64, 8);
  private static final class Sink<T> implements Flow.Subscriber<T> {
    Flow.Subscription subscription;
    final List<T> values = new ArrayList<>();
    Throwable error;
    int completed;
    @Override public void onSubscribe(Flow.Subscription subscription) { this.subscription = subscription; }
    @Override public void onNext(T item) { values.add(item); }
    @Override public void onError(Throwable failure) { error = failure; }
    @Override public void onComplete() { completed++; }
  }
  private static UnifiedIMapService.DomainCommand put(long id, String key) {
    return new UnifiedIMapService.DomainCommand("set").key(BytesValue.fromHex(key))
        .value(BytesValue.fromHex("31")).identity(new BoundedIMapApi.MutationIdentity("flow", id));
  }

  @Test
  void demandControlsPagesAndCancellationReleasesOwnership() {
    var service = new UnifiedIMapService(BOUNDS, 1);
    service.execute(put(1, "61")); service.execute(put(2, "62")); service.execute(put(3, "63"));
    var publisher = service.pages(BoundedIMapApi.Projection.ENTRIES, 2, () -> 0);
    var sink = new Sink<BoundedIMapApi.ScanItem>(); publisher.subscribe(sink);
    assertEquals(2, publisher.maxPrefetch()); assertTrue(sink.values.isEmpty());
    sink.subscription.request(1); assertEquals(1, sink.values.size());
    assertEquals(0, publisher.demand());
    sink.subscription.request(1); assertEquals(2, sink.values.size());
    assertEquals(1, service.stats(0).activeStreams());
    assertEquals(1, service.stats(0).activeCursors());
    sink.subscription.cancel(); sink.subscription.request(10); publisher.signal();
    assertTrue(publisher.isClosed()); assertEquals(2, sink.values.size());
    assertEquals(0, service.stats(0).activeStreams());
    assertEquals(0, service.stats(0).activeCursors());
    var complete = service.pages(BoundedIMapApi.Projection.KEYS, 1, () -> 0);
    var all = new Sink<BoundedIMapApi.ScanItem>(); complete.subscribe(all);
    all.subscription.request(Long.MAX_VALUE);
    assertEquals(3, all.values.size()); assertEquals(1, all.completed); assertNull(all.error);
  }

  @Test
  void liveListenerPendingIsNotCompletionAndDemandResumesOnPublish() {
    var service = new UnifiedIMapService(BOUNDS, 1);
    var handle = service.registerListener(1, new UnifiedIMapService.ListenerOptions(true, 8, 2, Optional.empty(), Set.of()));
    var publisher = service.events(handle); var sink = new Sink<UnifiedIMapService.Event>();
    publisher.subscribe(sink); sink.subscription.request(1);
    assertTrue(sink.values.isEmpty()); assertEquals(0, sink.completed);
    service.execute(put(1, "61")); assertEquals("added", sink.values.get(0).kind());
    service.execute(put(2, "62")); assertEquals(1, sink.values.size());
    sink.subscription.request(1); assertEquals(2, sink.values.size());
    sink.subscription.cancel(); assertEquals(0, service.stats(0).listeners());
    assertEquals(0, service.stats(0).queuedEvents());
    assertEquals(0, service.stats(0).activeStreams());
  }

  @Test
  void invalidDemandSourceFailureAndDuplicateSubscribersHaveOneTerminalSignal() {
    var closed = new AtomicInteger();
    var publisher = new IMapFlowPublisher<Integer>(new IMapFlowPublisher.Source<>() {
      @Override public IMapFlowPublisher.Pull<Integer> poll() { throw new IllegalStateException("broken"); }
      @Override public void close() { closed.incrementAndGet(); }
    }, 1);
    var sink = new Sink<Integer>(); publisher.subscribe(sink); sink.subscription.request(1);
    assertNotNull(sink.error); assertEquals(1, closed.get());
    sink.subscription.request(1); publisher.close(); assertEquals(1, closed.get());
    var duplicate = new Sink<Integer>(); publisher.subscribe(duplicate);
    assertNotNull(duplicate.error); duplicate.subscription.request(1); duplicate.subscription.cancel();
    var invalid = new IMapFlowPublisher<Integer>(new IMapFlowPublisher.Source<>() {
      @Override public IMapFlowPublisher.Pull<Integer> poll() { return IMapFlowPublisher.Pull.item(1); }
      @Override public void close() { throw new IllegalStateException("cleanup"); }
    }, 1);
    var bad = new Sink<Integer>(); invalid.subscribe(bad); bad.subscription.request(0);
    assertInstanceOf(IllegalArgumentException.class, bad.error);
    assertTrue(invalid.cleanupFailure().isPresent());
    assertThrows(IllegalArgumentException.class, () -> new IMapFlowPublisher.Pull<Integer>(IMapFlowPublisher.State.ITEM, Optional.empty()));
    assertThrows(IllegalArgumentException.class, () -> new IMapFlowPublisher<>(new IMapFlowPublisher.Source<Integer>() {
      public IMapFlowPublisher.Pull<Integer> poll() { return IMapFlowPublisher.Pull.end(); }
      public void close() {}
    }, 0));
  }

  @Test
  void reentrantDemandAndStalePageFailureReleaseBuffers() {
    var sourceIndex = new AtomicInteger();
    var publisher = new IMapFlowPublisher<Integer>(new IMapFlowPublisher.Source<>() {
      public IMapFlowPublisher.Pull<Integer> poll() { int value = sourceIndex.incrementAndGet();
        return value <= 3 ? IMapFlowPublisher.Pull.item(value) : IMapFlowPublisher.Pull.end(); }
      public void close() {}
    }, 1);
    var values = new ArrayList<Integer>();
    publisher.subscribe(new Flow.Subscriber<>() {
      Flow.Subscription subscription;
      public void onSubscribe(Flow.Subscription value) { subscription = value; value.request(1); }
      public void onNext(Integer value) { values.add(value); subscription.request(1); }
      public void onError(Throwable failure) { fail(failure); }
      public void onComplete() {}
    });
    assertEquals(List.of(1, 2, 3), values);
    var service = new UnifiedIMapService(BOUNDS, 1); service.execute(put(1, "61"));
    var pages = service.pages(BoundedIMapApi.Projection.KEYS, 1, () -> 0);
    var sink = new Sink<BoundedIMapApi.ScanItem>(); pages.subscribe(sink);
    service.execute(put(2, "62")); sink.subscription.request(1);
    assertInstanceOf(IMapDomainException.class, sink.error); assertTrue(pages.isClosed());
  }

  @Test
  void removingAnIdleListenerWakesAndTerminatesItsStreamWithoutLeakingOwners() {
    var service = new UnifiedIMapService(BOUNDS, 1);
    var handle = service.registerListener(1, UnifiedIMapService.ListenerOptions.metadata(2));
    var publisher = service.events(handle); var sink = new Sink<UnifiedIMapService.Event>();
    publisher.subscribe(sink); sink.subscription.request(1);
    assertThrows(IMapDomainException.class, () -> service.events(handle));
    assertTrue(service.removeListener(1));
    assertInstanceOf(IMapDomainException.class, sink.error);
    assertEquals("unknown_listener", ((IMapDomainException) sink.error).kind());
    assertEquals(0, service.stats(0).activeStreams());
    assertEquals(0, service.stats(0).listeners());
  }

  @Test
  void removingAListenerDiscardsPrefetchedEventsAndClosesItsSource() {
    var service = new UnifiedIMapService(BOUNDS, 1);
    var handle = service.registerListener(1, UnifiedIMapService.ListenerOptions.metadata(2));
    var publisher = service.events(handle); var sink = new Sink<UnifiedIMapService.Event>();
    publisher.subscribe(sink);
    service.execute(put(1, "61")); service.execute(put(2, "62"));
    sink.subscription.request(1);
    assertEquals(1, sink.values.size()); assertEquals(0, service.stats(0).queuedEvents());
    service.removeListener(1);
    sink.subscription.request(1);
    assertEquals(1, sink.values.size()); assertEquals(0, sink.completed);
    assertInstanceOf(IMapDomainException.class, sink.error); assertTrue(publisher.isClosed());
    assertEquals("unknown_listener", ((IMapDomainException) sink.error).kind());
    assertEquals(0, service.stats(0).activeStreams());
    sink.subscription.cancel(); publisher.close();
    assertEquals(0, service.stats(0).activeStreams());
  }

  @Test
  void closingAListenerWakesItsSubscriberOutsideTheServiceMonitor() {
    var service = new UnifiedIMapService(BOUNDS, 1);
    var handle = service.registerListener(1, UnifiedIMapService.ListenerOptions.metadata(2));
    var publisher = service.events(handle);
    var errors = new AtomicInteger();
    publisher.subscribe(new Flow.Subscriber<>() {
      public void onSubscribe(Flow.Subscription subscription) { subscription.request(1); }
      public void onNext(UnifiedIMapService.Event event) { fail("closed listener emitted an event"); }
      public void onError(Throwable failure) {
        assertFalse(Thread.holdsLock(service));
        assertInstanceOf(IMapDomainException.class, failure); errors.incrementAndGet();
      }
      public void onComplete() { fail("removed listener completed without its lifecycle error"); }
    });
    handle.close(); handle.close();
    assertEquals(1, errors.get()); assertTrue(publisher.isClosed());
    assertEquals(0, service.stats(0).activeStreams()); assertEquals(0, service.stats(0).listeners());
  }

  @Test
  void terminalEventsAndUnsubscribedSourcesReleaseStreamsAndCursorsExactlyOnce() {
    var service = new UnifiedIMapService(BOUNDS, 1);
    var publisher = service.events(service.registerListener(1, UnifiedIMapService.ListenerOptions.metadata(2)));
    var sink = new Sink<UnifiedIMapService.Event>(); publisher.subscribe(sink);
    sink.subscription.request(Long.MAX_VALUE);
    service.execute(put(1, "61"));
    service.execute(new UnifiedIMapService.DomainCommand("destroy")
        .identity(new BoundedIMapApi.MutationIdentity("flow", 2)));
    assertEquals("destroyed", sink.values.get(1).kind()); assertEquals(1, sink.completed);
    assertEquals(0, service.stats(0).activeStreams());
    var other = new UnifiedIMapService(BOUNDS, 1);
    var pages = other.pages(BoundedIMapApi.Projection.KEYS, 1, () -> 0);
    pages.close(); pages.close();
    assertEquals(0, other.stats(0).activeStreams()); assertEquals(0, other.stats(0).activeCursors());
    var closed = new AtomicInteger();
    assertThrows(IllegalArgumentException.class, () -> new IMapFlowPublisher<>(new IMapFlowPublisher.Source<Integer>() {
      public IMapFlowPublisher.Pull<Integer> poll() { return IMapFlowPublisher.Pull.pending(); }
      public void close() { closed.incrementAndGet(); throw new IllegalStateException("cleanup"); }
    }, 257));
    assertEquals(1, closed.get());
  }

  @Test
  void listenerCallbacksObserveRetainedCompletionAndMayReenterTheService() {
    var service = new UnifiedIMapService(BOUNDS, 1);
    var publisher = service.events(service.registerListener(1, UnifiedIMapService.ListenerOptions.metadata(2)));
    var callbacks = new AtomicInteger();
    publisher.subscribe(new Flow.Subscriber<>() {
      Flow.Subscription subscription;
      public void onSubscribe(Flow.Subscription value) { subscription = value; value.request(1); }
      public void onNext(UnifiedIMapService.Event event) {
        assertEquals("completed", service.queryOperationOutcome(new BoundedIMapApi.MutationIdentity("flow", 1)).kind());
        assertEquals("present", service.execute(new UnifiedIMapService.DomainCommand("get")
            .key(BytesValue.fromHex("61"))).kind());
        callbacks.incrementAndGet(); subscription.cancel();
      }
      public void onError(Throwable failure) { fail(failure); }
      public void onComplete() {}
    });
    service.execute(put(1, "61"));
    assertEquals(1, callbacks.get());
    assertEquals(0, service.stats(0).activeStreams());
  }
}
