package io.hydracache.imap.hazelcast;

import static org.junit.jupiter.api.Assertions.*;
import com.hazelcast.client.HazelcastClient;
import com.hazelcast.client.config.ClientConfig;
import com.hazelcast.config.Config;
import com.hazelcast.config.InMemoryFormat;
import com.hazelcast.config.MapConfig;
import com.hazelcast.core.Hazelcast;
import com.hazelcast.core.HazelcastInstance;
import io.hydracache.imap.semantic.*;
import java.net.ServerSocket;
import java.nio.file.Files;
import java.util.*;
import java.util.concurrent.TimeUnit;
import org.junit.jupiter.api.*;

final class HazelcastProductAdapter075Test {
  private static HazelcastInstance member, client;
  private static final BytesValue KEY = BytesValue.fromHex("00ff");
  private static final BytesValue VALUE = BytesValue.fromHex("0080ff");
  private static HazelcastLoopback075 fixture;
  @BeforeAll static void startActualMemberAndOfficialClient() throws Exception {
    fixture=new HazelcastLoopback075(); member=fixture.member(); client=fixture.client();
    assertTrue(member.getCluster().getLocalMember().getAddress().getInetAddress().isLoopbackAddress());
  }
  @AfterAll static void stopOwnedProcesses() { if(fixture!=null) { fixture.close(); fixture.close(); } }
  private static String name() { return "imap075-"+UUID.randomUUID(); }
  private static ScenarioStep point(Operation operation, BytesValue value, Optional<BytesValue> expected) {
    boolean entry = Set.of(Operation.PUT,Operation.GET_AND_PUT,Operation.PUT_IF_ABSENT,Operation.REPLACE,Operation.REPLACE_IF_PRESENT).contains(operation);
    return new ScenarioStep("step",operation,entry?List.of():List.of(KEY),entry?List.of(new ScenarioEntry(KEY,value)):List.of(),expected,TtlDirective.eternal(),0);
  }
  private static List<io.hydracache.imap.semantic.MapEvent> awaitEvents(HazelcastProductAdapter075 adapter, int count) throws Exception {
    var events = new ArrayList<io.hydracache.imap.semantic.MapEvent>(); long deadline=System.nanoTime()+TimeUnit.SECONDS.toNanos(5);
    do { events.addAll(adapter.drainEvents()); if(events.size()>=count) return events; Thread.sleep(5); } while(System.nanoTime()<deadline);
    fail("native listener did not deliver the expected events"); return List.of();
  }
  @Test void actualOfficialClientMatchesIndependentEternalOracleAndNativeEvents() throws Exception {
    try (var adapter = new HazelcastProductAdapter075(client,name()); var oracle = InMemorySemanticAdapter.strict("independent")) {
      var steps = List.of(
          point(Operation.GET,VALUE,Optional.empty()), point(Operation.CONTAINS_KEY,VALUE,Optional.empty()),
          point(Operation.REPLACE_IF_PRESENT,VALUE,Optional.empty()), point(Operation.REPLACE,VALUE,Optional.of(VALUE)),
          point(Operation.REMOVE_IF_VALUE,VALUE,Optional.of(VALUE)), point(Operation.GET_AND_REMOVE,VALUE,Optional.empty()),
          point(Operation.PUT,VALUE,Optional.empty()), point(Operation.PUT,VALUE,Optional.empty()),
          point(Operation.CONTAINS_KEY,VALUE,Optional.empty()), point(Operation.PUT_IF_ABSENT,VALUE,Optional.empty()),
          point(Operation.REPLACE,VALUE,Optional.of(BytesValue.fromHex("09"))),
          point(Operation.REMOVE_IF_VALUE,VALUE,Optional.of(BytesValue.fromHex("09"))),
          point(Operation.REPLACE,BytesValue.fromHex("09"),Optional.of(VALUE)),
          point(Operation.REPLACE_IF_PRESENT,VALUE,Optional.empty()),
          point(Operation.GET_AND_PUT,BytesValue.fromHex("09"),Optional.empty()),
          new ScenarioStep("bulk",Operation.GET_ALL,List.of(KEY,BytesValue.fromHex("00")),List.of(),Optional.empty(),TtlDirective.eternal(),0),
          point(Operation.GET_AND_REMOVE,VALUE,Optional.empty()), point(Operation.PUT_IF_ABSENT,VALUE,Optional.empty()),
          point(Operation.REMOVE_IF_VALUE,VALUE,Optional.of(VALUE)));
      for (var step : steps) {
        assertEquals(oracle.execute(step),adapter.execute(step),step.operation().name());
        var expected = oracle.drainEvents(); assertEquals(expected,awaitEvents(adapter,expected.size()),step.operation().name());
        assertEquals(oracle.snapshot(),adapter.snapshot());
      }
      assertEquals(10,adapter.capabilities().size());
      assertTrue(adapter.identity().readiness().contains("not-release-qualified"));
    }
  }
  @Test void unsupportedModesAndRequestBoundsRefuseBeforeMutation() {
    try (var adapter = new HazelcastProductAdapter075(client,name())) {
      var initial=adapter.snapshot();
      assertEquals(ErrorClass.UNSUPPORTED,adapter.execute(new ScenarioStep("ttl",Operation.PUT,List.of(),List.of(new ScenarioEntry(KEY,VALUE)),Optional.empty(),TtlDirective.expireAfter(1),0)).errorClass());
      assertEquals(ErrorClass.UNSUPPORTED,adapter.execute(new ScenarioStep("advance",Operation.ADVANCE,List.of(),List.of(),Optional.empty(),TtlDirective.eternal(),1)).errorClass());
      assertEquals(ErrorClass.SIZE,adapter.execute(point(Operation.PUT,BytesValue.copyOf(new byte[65537]),Optional.empty())).errorClass());
      assertEquals(ErrorClass.SIZE,adapter.execute(ScenarioStep.get("big-key",BytesValue.copyOf(new byte[1025]))).errorClass());
      assertEquals(ErrorClass.SIZE,adapter.execute(new ScenarioStep("many",Operation.GET_ALL,Collections.nCopies(65,KEY),List.of(),Optional.empty(),TtlDirective.eternal(),0)).errorClass());
      assertEquals(ErrorClass.DUPLICATE_KEY,adapter.execute(new ScenarioStep("duplicate",Operation.GET_ALL,List.of(KEY,KEY),List.of(),Optional.empty(),TtlDirective.eternal(),0)).errorClass());
      assertEquals(ErrorClass.INVALID_ARGUMENT,adapter.execute(new ScenarioStep("preserve",Operation.PUT,List.of(),List.of(new ScenarioEntry(KEY,VALUE)),Optional.empty(),TtlDirective.preserve(),0)).errorClass());
      assertEquals(initial,adapter.snapshot());
    }
  }
  @Test void profileRefusesFiniteNativeTtlAndUnboundedExternalState() {
    String name=name();
    try(var adapter=new HazelcastProductAdapter075(client,name)) {
      com.hazelcast.map.IMap<byte[],byte[]> raw=client.getMap(name);
      raw.put(KEY.copy(),VALUE.copy(),3600,TimeUnit.SECONDS);
      assertEquals(ErrorClass.UNSUPPORTED,adapter.execute(ScenarioStep.get("finite",KEY)).errorClass());
      assertThrows(IllegalStateException.class,adapter::snapshot);
      raw.clear();
      for(int i=0;i<65;i++) raw.put(new byte[]{(byte)i},new byte[]{1});
      assertThrows(IllegalStateException.class,adapter::snapshot);
      assertEquals(ErrorClass.SIZE,adapter.execute(point(Operation.PUT,VALUE,Optional.empty())).errorClass());
    }
  }
  @Test void listenerLossIsExplicitAndCloseDoesNotDestroyClientOrMap() throws Exception {
    String name=name(); var adapter=new HazelcastProductAdapter075(client,name);
    com.hazelcast.map.IMap<byte[],byte[]> raw=client.getMap(name);
    adapter.execute(point(Operation.PUT,VALUE,Optional.empty())); awaitEvents(adapter,1);
    raw.clear();
    long deadline=System.nanoTime()+TimeUnit.SECONDS.toNanos(5); boolean gap=false;
    while(System.nanoTime()<deadline&&!gap) { try { adapter.drainEvents(); Thread.sleep(5); } catch(IllegalStateException expected){ gap=true; } }
    assertTrue(gap,"map-clear native event must expose a gap");
    adapter.close(); adapter.close();
    assertThrows(IllegalStateException.class,()->adapter.execute(ScenarioStep.get("closed",KEY)));
    assertThrows(IllegalStateException.class,adapter::snapshot); assertThrows(IllegalStateException.class,adapter::drainEvents);
    assertTrue(client.getLifecycleService().isRunning()); assertEquals(0,raw.size());
  }
  @Test void constructorAndArtifactTamperingCanariesAreFailClosed() throws Exception {
    assertThrows(IllegalStateException.class,()->HazelcastArtifactPin.requireVersion("future"));
    assertThrows(IllegalStateException.class,()->HazelcastArtifactPin.requireVersion(null));
    assertThrows(IllegalArgumentException.class,()->new HazelcastProductAdapter075(client,"foreign"));
    assertThrows(IllegalArgumentException.class,()->new HazelcastProductAdapter075(member,name()));
    String name=name(); client.<byte[],byte[]>getMap(name).put(new byte[]{1},new byte[]{2});
    assertThrows(IllegalArgumentException.class,()->new HazelcastProductAdapter075(client,name));
    var dir=Files.createTempDirectory("imap075-artifact-");
    try {
      assertThrows(IllegalStateException.class,()->HazelcastArtifactPin.verifyJar(dir));
      var fake=dir.resolve("fake.jar"); Files.write(fake,new byte[]{0,1,2});
      assertThrows(IllegalStateException.class,()->HazelcastArtifactPin.verifyJar(fake));
      Files.write(fake,new byte[0]); assertThrows(IllegalStateException.class,()->HazelcastArtifactPin.verifyJar(fake));
      Files.delete(fake); assertThrows(IllegalStateException.class,()->HazelcastArtifactPin.verifyJar(fake));
    } finally { Files.delete(dir); }
  }
  @Test void packagedConsumerProbeEmitsActualResultsAndRejectsUnknownOptions() throws Exception {
    String result=HazelcastProbeMain075.observe(fixture);
    assertTrue(result.contains("\"typed_port_kind\":\"inserted\""));
    assertTrue(result.contains("\"operation_count\":40"));
    assertTrue(result.contains("\"native_events\":[\"added\",\"updated\",\"removed\"]"));
    assertThrows(IllegalArgumentException.class,()->HazelcastProbeMain075.main(new String[]{"--skip"}));
    HazelcastProbeMain075.main(new String[0]);
  }
  @Test void failedClientStartupReleasesTheOwnedMember() {
    int members=Hazelcast.getAllHazelcastInstances().size();
    int clients=HazelcastClient.getAllHazelcastClients().size();
    assertThrows(RuntimeException.class,()->new HazelcastLoopback075("wrong-cluster"));
    assertEquals(members,Hazelcast.getAllHazelcastInstances().size());
    assertEquals(clients,HazelcastClient.getAllHazelcastClients().size());
  }
  @Test void nativeExpiryEvictionAndMapEvictionAlwaysExposeALossBoundary() throws Exception {
    for(int action=0;action<3;action++) {
      String name=name();
      try(var adapter=new HazelcastProductAdapter075(client,name)) {
        com.hazelcast.map.IMap<byte[],byte[]> raw=client.getMap(name);
        adapter.execute(point(Operation.PUT,VALUE,Optional.empty())); awaitEvents(adapter,1);
        if(action==0) raw.evict(KEY.copy());
        else if(action==1) raw.evictAll();
        else {
          raw.setTtl(KEY.copy(),1,TimeUnit.MILLISECONDS);
          long expiry=System.nanoTime()+TimeUnit.SECONDS.toNanos(5);
          while(raw.containsKey(KEY.copy())&&System.nanoTime()<expiry) Thread.sleep(5);
          assertFalse(raw.containsKey(KEY.copy()));
        }
        long deadline=System.nanoTime()+TimeUnit.SECONDS.toNanos(5); boolean gap=false;
        while(!gap&&System.nanoTime()<deadline) {try {adapter.drainEvents(); Thread.sleep(5);} catch(IllegalStateException expected){gap=true;}}
        assertTrue(gap,"native loss event "+action);
      }
    }
  }
  @Test void nativeQueueAndPayloadOverflowCannotBecomeAnEmptySuccessfulHistory() throws Exception {
    String name=name();
    try(var adapter=new HazelcastProductAdapter075(client,name)) {
      com.hazelcast.map.IMap<byte[],byte[]> raw=client.getMap(name);
      var delivered=new java.util.concurrent.CountDownLatch(70);
      UUID listener=raw.addEntryListener((com.hazelcast.map.listener.EntryAddedListener<byte[],byte[]>)event->delivered.countDown(),false);
      try {
        for(int i=0;i<70;i++) raw.put(new byte[]{(byte)i},new byte[]{1});
        assertTrue(delivered.await(5,TimeUnit.SECONDS));
        assertThrows(IllegalStateException.class,adapter::drainEvents);
      } finally {raw.removeEntryListener(listener);}
    }
    name=name();
    try(var adapter=new HazelcastProductAdapter075(client,name)) {
      com.hazelcast.map.IMap<byte[],byte[]> raw=client.getMap(name);
      raw.put(KEY.copy(),new byte[65537]);
      assertThrows(IllegalStateException.class,adapter::snapshot);
      long deadline=System.nanoTime()+TimeUnit.SECONDS.toNanos(5); boolean gap=false;
      while(!gap&&System.nanoTime()<deadline) {try {adapter.drainEvents();Thread.sleep(5);}catch(IllegalStateException expected){gap=true;}}
      assertTrue(gap);
    }
  }
}
