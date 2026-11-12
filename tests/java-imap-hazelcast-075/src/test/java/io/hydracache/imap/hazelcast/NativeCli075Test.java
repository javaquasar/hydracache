package io.hydracache.imap.hazelcast;

import java.nio.file.*;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import static org.junit.jupiter.api.Assertions.*;

class NativeCli075Test {
  @TempDir Path control;
  @Test void unsupportedOrOverflowingGcCountersStayUnknown() {
    assertEquals("null",NativeSoakWorker075.gcCollections());assertEquals("null",NativeSoakWorker075.gcCollections(0,-1));assertEquals("null",NativeSoakWorker075.gcCollections(Long.MAX_VALUE,1));assertEquals("0",NativeSoakWorker075.gcCollections(0,0));assertEquals("7",NativeSoakWorker075.gcCollections(3,4));
  }
  @Test void requiredDigestFailureIsExplicitWithoutChangingGlobalJdkProviders() {
    assertEquals("SHA-256",NativeDigests075.sha256().getAlgorithm());
    var missing=new java.security.NoSuchAlgorithmException("fixture provider unavailable");
    assertSame(missing,assertThrows(IllegalStateException.class,()->NativeDigests075.sha256(()->{throw missing;})).getCause());
  }
  @Test void packagedProbeEntrypointsRunTheirActualWorkloads() throws Exception {
    NativeExtendedProbeMain075.main(new String[]{});
    NativeConcurrentProbeMain075.main(new String[]{});
    NativeListenerRecoveryProbeMain075.main(new String[]{});
  }
  @Test void nativeSoakEntrypointUsesAnActualPersistentOwner() throws Exception {
    Files.writeString(control.resolve("start"),"start-v1");
    String[] args={"100","10","64","1","10000","117","a".repeat(40),control.toString()};
    assertEquals(117,NativeSoakWorker075.Config.parse(args).seed());
    NativeSoakWorker075.main(args);
  }
}
