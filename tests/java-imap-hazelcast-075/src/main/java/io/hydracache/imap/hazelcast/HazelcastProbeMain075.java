package io.hydracache.imap.hazelcast;

import io.hydracache.imap.semantic.*;
import java.util.*;
import java.util.concurrent.TimeUnit;

/** Executes packaged APIs and emits observations for an independent Rust verifier. */
public final class HazelcastProbeMain075 {
  private HazelcastProbeMain075() {}
  public static void main(String[] args) throws Exception {
    if(args.length!=0) throw new IllegalArgumentException("no probe options are accepted");
    try(var fixture=new HazelcastLoopback075()) { System.out.println(observe(fixture)); }
  }
  public static String observe(HazelcastLoopback075 fixture) throws Exception {
    var key=BytesValue.fromHex("00ff"); var value=BytesValue.fromHex("0080ff");
    String mapName="imap075-probe-"+UUID.randomUUID();
    try(var adapter=new HazelcastProductAdapter075(fixture.client(),mapName)) {
      var steps=List.of(ScenarioStep.get("absent",key),
          entry("put",Operation.PUT,key,value,Optional.empty()), ScenarioStep.get("present",key),
          entry("mismatch",Operation.REPLACE,key,BytesValue.fromHex("09"),Optional.of(BytesValue.fromHex("2a"))),
          entry("replace",Operation.REPLACE,key,BytesValue.fromHex("09"),Optional.of(value)),
          new ScenarioStep("remove",Operation.GET_AND_REMOVE,List.of(key),List.of(),Optional.empty(),TtlDirective.eternal(),0));
      var rows=new ArrayList<String>();
      for(var step:steps) {
        var outcome=adapter.execute(step);
        rows.add("{\"operation\":\""+step.id()+"\",\"kind\":\""+outcome.kind().name().toLowerCase(Locale.ROOT)
            +"\",\"value\":"+outcome.value().map(v->"\""+v.hex()+"\"").orElse("null")+"}");
      }
      var events=new ArrayList<io.hydracache.imap.semantic.MapEvent>();
      long deadline=System.nanoTime()+TimeUnit.SECONDS.toNanos(5);
      while(events.size()<3&&System.nanoTime()<deadline) { events.addAll(adapter.drainEvents()); if(events.size()<3) Thread.sleep(5); }
      if(events.size()!=3) throw new IllegalStateException("native event completeness");
      var eventNames=events.stream().map(e->"\""+e.kind().name().toLowerCase(Locale.ROOT)+"\"").toList();
      var service=new UnifiedIMapService(new BoundedIMapApi.Bounds(1024,4096,64,64,64,65536,64),3);
      var binding=new DevelopmentBackendPort075.Binding(1,2,3,0,4,5,"packaged-consumer");
      var port=new DevelopmentBackendPort075(binding,service,DevelopmentBackendPort075.Mode.LOCAL_REFERENCE);
      var command=new UnifiedIMapService.DomainCommand("put").key(key).value(value)
          .identity(new BoundedIMapApi.MutationIdentity("packaged-consumer",1));
      var result=port.execute(port.prepare(binding,command));
      var stored=service.execute(new UnifiedIMapService.DomainCommand("get").key(key)).value().orElseThrow();
      // Diagnostic samples are retained raw; they are not a paired product benchmark.
      com.hazelcast.map.IMap<byte[],byte[]> nativeMap=fixture.client().getMap(mapName);
      for(int i=0;i<8;i++) nativeMap.get(key.copy());
      var samples=new ArrayList<String>();
      for(int i=0;i<16;i++) {long before=System.nanoTime();nativeMap.get(key.copy());samples.add(Long.toString(System.nanoTime()-before));}
      var memory=java.lang.management.ManagementFactory.getMemoryMXBean();
      var heap=memory.getHeapMemoryUsage();
      long collections=java.lang.management.ManagementFactory.getGarbageCollectorMXBeans().stream().mapToLong(bean->Math.max(0,bean.getCollectionCount())).sum();
      String diagnostics="{\"scope\":\"single_jvm_member_client_diagnostic\",\"operation\":\"native_get_absent\",\"warmup_operations\":8,\"samples_ns\":["+String.join(",",samples)
          +"],\"heap_used_bytes\":"+heap.getUsed()+",\"heap_committed_bytes\":"+heap.getCommitted()+",\"non_heap_used_bytes\":"+memory.getNonHeapMemoryUsage().getUsed()
          +",\"threads\":"+java.lang.management.ManagementFactory.getThreadMXBean().getThreadCount()+",\"gc_collections\":"+collections+"}";
      return "{\"schema\":\"imap075-packaged-hazelcast-probe-v1\",\"profile\":\"exclusive_eternal_binary_no_tls_no_backups\",\"artifact_sha256\":\""
          +HazelcastArtifactPin.verifyLoadedArtifact()+"\",\"steps\":["+String.join(",",rows)+"],\"native_events\":["+String.join(",",eventNames)
          +"],\"snapshot_digest\":\""+adapter.snapshot().digest()+"\",\"typed_port_kind\":\""+result.kind()+"\",\"typed_port_value\":\""
          +stored.hex()+"\",\"operation_count\":"+DevelopmentBackendPort075.Operation.values().length+",\"diagnostics\":"+diagnostics+"}";
    }
  }
  private static ScenarioStep entry(String id,Operation operation,BytesValue key,BytesValue value,Optional<BytesValue> expected) {
    return new ScenarioStep(id,operation,List.of(),List.of(new ScenarioEntry(key,value)),expected,TtlDirective.eternal(),0);
  }
}
