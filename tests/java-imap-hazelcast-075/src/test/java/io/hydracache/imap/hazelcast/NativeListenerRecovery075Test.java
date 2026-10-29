package io.hydracache.imap.hazelcast;

import static org.junit.jupiter.api.Assertions.*;
import static io.hydracache.imap.hazelcast.NativeListenerRecovery075.*;
import com.hazelcast.core.EntryEvent;
import com.hazelcast.core.EntryListener;
import com.hazelcast.core.EntryView;
import com.hazelcast.map.IMap;
import com.hazelcast.core.EntryEventType;
import io.hydracache.imap.semantic.BytesValue;
import java.lang.reflect.Proxy;
import java.time.Duration;
import java.util.*;
import java.util.concurrent.*;
import org.junit.jupiter.api.Test;

final class NativeListenerRecovery075Test {
  static BytesValue b(String value) { return BytesValue.fromHex(value); }
  static final class Fake {
    final TreeMap<BytesValue,BytesValue> values = new TreeMap<>();
    final List<EntryListener<byte[],byte[]>> registrations = new ArrayList<>();
    final Map<UUID,EntryListener<byte[],byte[]>> active = new HashMap<>();
    boolean emit = true, removeFails; int removes, failAdd = -1; long ttl; Runnable removeHook = () -> {}, addHook = () -> {};
    @SuppressWarnings("unchecked") IMap<byte[],byte[]> map() {
      return (IMap<byte[],byte[]>) Proxy.newProxyInstance(getClass().getClassLoader(),new Class<?>[]{IMap.class},(proxy,method,args)->switch(method.getName()) {
        case "isEmpty" -> values.isEmpty();
        case "iterator" -> values.entrySet().stream().map(e->(Map.Entry<byte[],byte[]>)new AbstractMap.SimpleImmutableEntry<>(e.getKey().copy(),e.getValue().copy())).iterator();
        case "getEntryView" -> {
          var key=BytesValue.copyOf((byte[])args[0]); var value=values.get(key);
          yield value==null ? null : Proxy.newProxyInstance(getClass().getClassLoader(),new Class<?>[]{EntryView.class},(p,m,a)->switch(m.getName()) {case "getKey"->key.copy();case "getValue"->value.copy();case "getTtl"->ttl;default->0L;});
        }
        case "addEntryListener" -> {
          if(registrations.size()+1==failAdd) throw new IllegalStateException("injected registration failure");
          var listener=(EntryListener<byte[],byte[]>)args[0]; var id=UUID.randomUUID();registrations.add(listener);active.put(id,listener);addHook.run();yield id;
        }
        case "removeEntryListener" -> {removes++;if(removeFails)yield false;active.remove((UUID)args[0]);removeHook.run();yield true;}
        case "put" -> {
          var key=BytesValue.copyOf((byte[])args[0]); var value=BytesValue.copyOf((byte[])args[1]);var old=values.put(key,value);
          if(emit) for(var listener:List.copyOf(active.values())) event(listener,old==null,key,old,value);
          yield old==null?null:old.copy();
        }
        default -> throw new AssertionError("unexpected native map call "+method.getName());
      });
    }
    void event(EntryListener<byte[],byte[]> listener,boolean added,BytesValue key,BytesValue old,BytesValue value) {
      var event=new EntryEvent<byte[],byte[]>("fixture",null,added?EntryEventType.ADDED.getType():EntryEventType.UPDATED.getType(),key.copy(),old==null?null:old.copy(),value.copy());
      if(added)listener.entryAdded(event);else listener.entryUpdated(event);
    }
  }
  static NativeListenerRecovery075 overflow(Fake fake) {
    var fixture=new NativeListenerRecovery075(fake.map(),1,16);fixture.put(b("61"),b("31"));fixture.put(b("61"),b("32"));
    assertEquals(State.GAP,fixture.state());assertEquals(Optional.of(Reason.QUEUE_OVERFLOW),fixture.reason());return fixture;
  }
  @Test void overflowRecoversOnlyAfterExactCorrelationAndOldGenerationCannotReenterQueue() throws Exception {
    var fake=new Fake();var fixture=overflow(fake);var stale=fake.registrations.get(0);
    var recovery=fixture.recover(Bounds.smoke(),new Cancellation());assertEquals(1,recovery.oldGeneration());assertEquals(2,recovery.newGeneration());assertEquals(2,recovery.correlatedWrites());
    assertEquals(b("32"),recovery.baseline().entries().get(0).value());assertEquals(State.HEALTHY,fixture.state());assertEquals(2,fixture.generation());assertTrue(fixture.reason().isEmpty());
    fake.event(stale,false,b("61"),b("31"),b("32"));assertTrue(fixture.drainEvents().isEmpty());
    fixture.put(b("61"),b("33"));assertEquals(2,fixture.drainEvents().get(0).registrationGeneration());
    fixture.put(b("61"),b("34"));fixture.put(b("61"),b("35"));assertEquals(State.GAP,fixture.state());
    assertEquals(3,fixture.recover(Bounds.smoke(),new Cancellation()).newGeneration());
    fixture.close();fixture.close();assertEquals(State.CLOSED,fixture.state());assertEquals(3,fake.removes);assertTrue(fake.active.isEmpty());assertEquals(b("35"),fake.values.get(b("61")));
    assertThrows(IllegalStateException.class,fixture::drainEvents);assertThrows(IllegalStateException.class,()->fixture.put(b("61"),b("36")));
  }
  @Test void equalCountersWithOneMissingAndOneDuplicateCallbackMustNotRecover() {
    var fake=new Fake();fake.emit=false;try(var fixture=new NativeListenerRecovery075(fake.map(),1,16)) {
      fixture.put(b("61"),b("31"));fixture.put(b("61"),b("32"));var listener=fake.registrations.get(0);
      fake.event(listener,true,b("61"),null,b("31"));fake.event(listener,true,b("61"),null,b("31"));
      assertEquals(fixture.expectedCallbacks(),fixture.observedCallbacks());assertEquals(Optional.of(Reason.CORRELATION_MISMATCH),fixture.reason());
      assertThrows(IllegalStateException.class,()->fixture.recover(Bounds.smoke(),new Cancellation()));
    }
  }
  @Test void missingCallbackWaitIsBoundedAndCanCompleteOrBeCancelled() throws Exception {
    var fake=new Fake();try(var fixture=overflow(fake)) {
      fake.emit=false;fixture.put(b("61"),b("33"));var executor=Executors.newSingleThreadExecutor();
      try {
        var result=executor.submit(()->fixture.recover(Bounds.smoke(),new Cancellation()));
        long until=System.nanoTime()+TimeUnit.SECONDS.toNanos(2);while(!fixture.waitingForCallbacks()&&System.nanoTime()<until)Thread.yield();assertTrue(fixture.waitingForCallbacks());
        fake.event(fake.registrations.get(0),false,b("61"),b("32"),b("33"));assertEquals(3,result.get(2,TimeUnit.SECONDS).correlatedWrites());
      } finally {executor.shutdownNow();}
    }
    var missing=new Fake();try(var fixture=overflow(missing)) {
      missing.emit=false;fixture.put(b("61"),b("33"));
      assertThrows(IllegalStateException.class,()->fixture.recover(new Bounds(64,4096,Duration.ofMillis(20)),new Cancellation()));
      assertEquals(Optional.of(Reason.MISSING_CALLBACK),fixture.reason());
    }
    var cancelled=new Fake();try(var fixture=overflow(cancelled)) {
      var token=new Cancellation();token.cancel();assertThrows(IllegalStateException.class,()->fixture.recover(Bounds.smoke(),token));assertEquals(Optional.of(Reason.CANCELLED),fixture.reason());
    }
  }
  @Test void registrationBaselineForeignAndProfileFailuresNeverResumeWriters() {
    for(int mode=0;mode<5;mode++) {
      var fake=new Fake();try(var fixture=overflow(fake)) {
        Bounds bounds=Bounds.smoke();Reason expected;
        switch(mode) {
          case 0 -> {fake.failAdd=2;expected=Reason.REGISTRATION_FAILED;}
          case 1 -> {fake.removeFails=true;expected=Reason.REGISTRATION_FAILED;}
          case 2 -> {bounds=new Bounds(64,1,Duration.ofSeconds(1));expected=Reason.BASELINE_BOUND;}
          case 3 -> {fake.values.put(b("62"),b("33"));expected=Reason.FOREIGN_STATE;}
          default -> {fake.ttl=1;expected=Reason.BASELINE_BOUND;}
        }
        var chosen=bounds;assertThrows(IllegalStateException.class,()->fixture.recover(chosen,new Cancellation()));assertEquals(State.GAP,fixture.state());assertEquals(Optional.of(expected),fixture.reason());
        assertThrows(IllegalStateException.class,()->fixture.put(b("61"),b("34")));
        if(mode==1)fake.removeFails=false;
      }
    }
    var fake=new Fake();try(var fixture=new NativeListenerRecovery075(fake.map(),1,16)) {
      var event=new EntryEvent<byte[],byte[]>("fixture",null,EntryEventType.EXPIRED.getType(),new byte[]{1},new byte[]{2});fake.registrations.get(0).entryExpired(event);
      assertEquals(Optional.of(Reason.PROFILE_VIOLATION),fixture.reason());assertThrows(IllegalStateException.class,()->fixture.recover(Bounds.smoke(),new Cancellation()));
    }
  }
  @Test void cancellationAfterOldRemovalAndCloseDuringRecoveryReleaseOwnedSubscriptions() throws Exception {
    var fake=new Fake();try(var fixture=overflow(fake)) {
      var token=new Cancellation();fake.removeHook=token::cancel;
      assertThrows(IllegalStateException.class,()->fixture.recover(Bounds.smoke(),token));assertEquals(State.GAP,fixture.state());assertTrue(fake.active.isEmpty());
    }
    var closing=new Fake();var fixture=overflow(closing);var token=new Cancellation();var closer=Executors.newSingleThreadExecutor();
    try {
      var closed=new CompletableFuture<Void>();closing.addHook=()->{assertEquals(State.RECOVERING,fixture.state());closer.submit(()->{fixture.close();closed.complete(null);});token.cancel();};
      assertThrows(IllegalStateException.class,()->fixture.recover(Bounds.smoke(),token));closed.get(2,TimeUnit.SECONDS);assertEquals(State.CLOSED,fixture.state());assertTrue(closing.active.isEmpty());
    } finally {fixture.close();closer.shutdownNow();}
  }
  @Test void invalidBoundsAndAmbiguousWorkloadRefuseBeforeWritesAndInterruptedRecoveryIsExplicit() {
    assertThrows(IllegalArgumentException.class,()->new Bounds(0,1,Duration.ofSeconds(1)));assertThrows(IllegalArgumentException.class,()->new Bounds(1,0,Duration.ofSeconds(1)));
    assertThrows(IllegalArgumentException.class,()->new Bounds(1,1,Duration.ofSeconds(6)));assertThrows(IllegalArgumentException.class,()->new Bounds(1,1,Duration.ZERO));
    var fake=new Fake();try(var fixture=new NativeListenerRecovery075(fake.map(),1,16)) {
      assertThrows(IllegalStateException.class,()->fixture.recover(Bounds.smoke(),new Cancellation()));
      fixture.put(b("61"),b("31"));fixture.put(b("61"),b("32"));
      Thread.currentThread().interrupt();try {assertThrows(InterruptedException.class,()->fixture.recover(Bounds.smoke(),new Cancellation()));}finally{Thread.interrupted();}
      assertEquals(Optional.of(Reason.CANCELLED),fixture.reason());
    }
    var unique=new Fake();try(var fixture=new NativeListenerRecovery075(unique.map(),4,4)) {
      fixture.put(b("61"),b("31"));fixture.put(b("61"),b("31"));assertThrows(IllegalArgumentException.class,()->fixture.put(b("61"),b("31")));assertEquals(2,fixture.expectedCallbacks());
      assertThrows(IllegalArgumentException.class,()->fixture.put(BytesValue.copyOf(new byte[1025]),b("31")));assertThrows(IllegalArgumentException.class,()->fixture.put(b("61"),BytesValue.copyOf(new byte[65537])));
    }
    var ledger=new Fake();try(var fixture=new NativeListenerRecovery075(ledger.map(),1,1)) {fixture.put(b("61"),b("31"));assertThrows(IllegalStateException.class,()->fixture.put(b("61"),b("32")));assertEquals(Optional.of(Reason.LEDGER_BOUND),fixture.reason());}
  }
  @Test void actualNativeOverflowRecoveryProducesBoundedBaselineAndNewGenerationCallbacks() throws Exception {
    assertThrows(IllegalArgumentException.class,()->NativeListenerRecoveryProbeMain075.main(new String[]{"foreign"}));
    try(var cluster=new HazelcastLoopback075()) {
      String name="imap075-recovery-"+UUID.randomUUID();try(var fixture=new NativeListenerRecovery075(cluster.client(),name,2)) {
        for(int i=0;i<8;i++)fixture.put(b("61"),BytesValue.copyOf(new byte[]{(byte)i}));
        long until=System.nanoTime()+TimeUnit.SECONDS.toNanos(5);while(fixture.observedCallbacks()!=8&&System.nanoTime()<until)Thread.yield();
        assertEquals(8,fixture.observedCallbacks());assertThrows(IllegalStateException.class,fixture::drainEvents);
        var cut=fixture.recover(Bounds.smoke(),new Cancellation());assertEquals(8,cut.correlatedWrites());assertEquals(b("07"),cut.baseline().entries().get(0).value());
        fixture.put(b("61"),b("08"));until=System.nanoTime()+TimeUnit.SECONDS.toNanos(5);while(fixture.observedCallbacks()!=1&&System.nanoTime()<until)Thread.yield();
        assertEquals(2,fixture.drainEvents().get(0).registrationGeneration());
      }
      assertTrue(cluster.client().getLifecycleService().isRunning());assertEquals(1,cluster.client().getMap(name).size());
      var probe=NativeListenerRecoveryProbeMain075.observe(cluster);
      assertTrue(probe.contains("\"gap_before\":\"queue_overflow\""));assertTrue(probe.contains("\"removed_subscriptions\":2"));
    }
  }
}
