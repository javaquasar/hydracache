package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.*;
import java.util.*;
import java.util.stream.Collectors;
import org.junit.jupiter.api.Test;

final class DevelopmentBackendPort075Test {
  private static final BytesValue KEY = BytesValue.fromHex("00ff");
  private static DevelopmentBackendPort075.Binding binding() { return new DevelopmentBackendPort075.Binding(1,2,3,0,4,5,"consumer"); }
  private static UnifiedIMapService service() { return new UnifiedIMapService(new BoundedIMapApi.Bounds(1024,4096,64,64,64,65536,64),3); }
  private static UnifiedIMapService.DomainCommand put(long sequence) {
    return new UnifiedIMapService.DomainCommand("put").key(KEY).value(BytesValue.fromHex("0080ff"))
        .identity(new BoundedIMapApi.MutationIdentity("consumer",sequence));
  }
  @Test void closedCatalogAndEveryCommandExecuteAgainstTheSameOwner() {
    assertEquals(IMapCapabilityDescriptor.unified(service().semanticBounds()).operations(), Arrays.stream(DevelopmentBackendPort075.Operation.values()).map(DevelopmentBackendPort075.Operation::id).collect(Collectors.toSet()));
    for (var operation : DevelopmentBackendPort075.Operation.values()) {
      assertEquals(operation, DevelopmentBackendPort075.Operation.parse(operation.id()));
      var service = service(); service.execute(put(1));
      var token = service.execute(new UnifiedIMapService.DomainCommand("get_entry_view").key(KEY)).token().orElseThrow();
      var port = new DevelopmentBackendPort075(binding(),service,DevelopmentBackendPort075.Mode.LOCAL_REFERENCE);
      var request = new UnifiedIMapService.DomainCommand(operation.id()).key(KEY).value(BytesValue.fromHex("09"))
          .expected(BytesValue.fromHex("0080ff")).keys(List.of(KEY)).entries(List.of(new BoundedIMapApi.ReplaceInput(KEY,BytesValue.fromHex("09"))))
          .identity(new BoundedIMapApi.MutationIdentity("consumer",2)).token(token).ttl(TtlDirective.eternal()).listenerId(1).limit(64);
      assertNotNull(port.execute(port.prepare(binding(),request)),operation.id());
    }
    assertThrows(IMapDomainException.class, () -> DevelopmentBackendPort075.Operation.parse("entry_processor"));
  }
  @Test void preparationOwnsPayloadAndDoesNotAdvanceOrReserve() {
    var service = service(); var port = new DevelopmentBackendPort075(binding(),service,DevelopmentBackendPort075.Mode.LOCAL_REFERENCE);
    var before = service.diagnosticStats(0); var usage = service.resourceUsage();
    var request = put(1).ttl(TtlDirective.expireAfter(10)).deadline(7);
    var prepared = port.prepare(binding(),request); request.value(BytesValue.fromHex("2a"));
    assertEquals(DevelopmentBackendPort075.Operation.PUT,prepared.operation()); assertEquals(7,prepared.deadline());
    assertEquals(before,service.diagnosticStats(0)); assertEquals(usage,service.resourceUsage());
    var result = port.execute(prepared);
    assertEquals(result,port.execute(port.prepare(binding(),put(1).ttl(TtlDirective.expireAfter(10)).deadline(7))));
    assertEquals(BytesValue.fromHex("0080ff"),service.execute(new UnifiedIMapService.DomainCommand("get").key(KEY)).value().orElseThrow());
  }
  @Test void contextIdentityAndProductionActivationAreFenced() {
    var service = service(); var port = new DevelopmentBackendPort075(binding(),service,DevelopmentBackendPort075.Mode.LOCAL_REFERENCE);
    var other = new DevelopmentBackendPort075.Binding(1,2,3,0,5,5,"consumer");
    assertEquals(DevelopmentBackendPort075.Failure.CONTEXT_MISMATCH,assertThrows(DevelopmentBackendPort075.BoundaryException.class,()->port.prepare(other,put(1))).failure());
    var second = new DevelopmentBackendPort075(other,service(),DevelopmentBackendPort075.Mode.LOCAL_REFERENCE);
    assertThrows(DevelopmentBackendPort075.BoundaryException.class,()->second.execute(port.prepare(binding(),put(1))));
    assertEquals(DevelopmentBackendPort075.Failure.IDENTITY_MISMATCH,assertThrows(DevelopmentBackendPort075.BoundaryException.class,()->port.prepare(binding(),put(1).identity(new BoundedIMapApi.MutationIdentity("other",1)))).failure());
    assertThrows(DevelopmentBackendPort075.BoundaryException.class,()->port.prepare(binding(),new UnifiedIMapService.DomainCommand("put").key(KEY).value(KEY)));
    assertEquals(DevelopmentBackendPort075.Failure.BLOCKED_BY_074,assertThrows(DevelopmentBackendPort075.BoundaryException.class,()->new DevelopmentBackendPort075(binding(),service,DevelopmentBackendPort075.Mode.PRODUCTION_MEMBER)).failure());
    assertThrows(DevelopmentBackendPort075.BoundaryException.class,()->new DevelopmentBackendPort075(binding(),new UnifiedIMapService(new BoundedIMapApi.Bounds(1024,4096,64,64,64,65536,64),4),DevelopmentBackendPort075.Mode.LOCAL_REFERENCE));
    assertEquals(0,service.diagnosticStats(0).requests());
  }
  @Test void bindingAndMalformedRequestRefusalsAreExplicit() {
    assertThrows(IllegalArgumentException.class,()->new DevelopmentBackendPort075.Binding(0,1,1,0,1,1,"c"));
    assertThrows(IllegalArgumentException.class,()->new DevelopmentBackendPort075.Binding(1,0,1,0,1,1,"c"));
    assertThrows(IllegalArgumentException.class,()->new DevelopmentBackendPort075.Binding(1,1,0,0,1,1,"c"));
    assertThrows(IllegalArgumentException.class,()->new DevelopmentBackendPort075.Binding(1,1,1,-1,1,1,"c"));
    assertThrows(IllegalArgumentException.class,()->new DevelopmentBackendPort075.Binding(1,1,1,0,0,1,"c"));
    assertThrows(IllegalArgumentException.class,()->new DevelopmentBackendPort075.Binding(1,1,1,0,1,0,"c"));
    assertThrows(IllegalArgumentException.class,()->new DevelopmentBackendPort075.Binding(1,1,1,0,1,1,"\u2003"));
    assertThrows(IllegalArgumentException.class,()->new DevelopmentBackendPort075.Binding(1,1,1,0,1,1,"я".repeat(65)));
    var service = service(); var port = new DevelopmentBackendPort075(binding(),service,DevelopmentBackendPort075.Mode.LOCAL_REFERENCE);
    assertThrows(IMapDomainException.class,()->port.prepare(binding(),put(1).key(BytesValue.copyOf(new byte[1025])).now(100)));
    assertEquals(0,service.diagnosticStats(0).requests());
  }
  @Test void cancellationAndExpiredRetriesKeepTheRetainedOwnerOutcome() {
    var service=service(); var port=new DevelopmentBackendPort075(binding(),service,DevelopmentBackendPort075.Mode.LOCAL_REFERENCE);
    assertEquals("not_applied",port.execute(port.prepare(binding(),put(1).cancelled(true))).kind());
    var original=port.execute(port.prepare(binding(),put(1)));
    assertEquals(original,port.execute(port.prepare(binding(),put(1).cancelled(true).deadline(0).now(100))));
  }
}
