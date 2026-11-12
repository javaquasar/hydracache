package io.hydracache.imap.hazelcast;
import static org.junit.jupiter.api.Assertions.*;
import java.nio.file.*;
import java.io.*;
import java.util.*;
import org.junit.jupiter.api.Test;

final class NativeSoakWorker075Test {
  static NativeSoakWorker075.Config config(Path control) {return new NativeSoakWorker075.Config(80,5,64,2,100,117,"a".repeat(40),control);}
  @Test void oneLongLivedNativeOwnerCompletesMultipleRoundsAndReleasesItsFixture() throws Exception {
    Path control=Files.createTempDirectory("imap075-native-soak-");
    try {
      Files.writeString(control.resolve("start"),"start-v1");var bytes=new ByteArrayOutputStream();NativeSoakWorker075.run(config(control),new PrintStream(bytes));
      var lines=bytes.toString(java.nio.charset.StandardCharsets.UTF_8).lines().toList();
      assertTrue(lines.get(0).contains("\"phase\":\"ready\""));assertTrue(lines.stream().filter(s->s.contains("\"phase\":\"round\"")).count()>=2);
      String last=lines.get(lines.size()-1);assertTrue(last.contains("\"phase\":\"drained\""));assertTrue(last.contains("\"reason\":\"duration_complete\""));
      assertTrue(last.contains("\"values\":0,\"outcomes\":null,\"pending\":null,\"listeners\":0"));
      var identities=lines.stream().map(s->s.split("\"incarnation\":\"")[1].split("\"")[0]).distinct().toList();assertEquals(1,identities.size());
    } finally {Files.deleteIfExists(control.resolve("start"));Files.delete(control);}
  }
  @Test void invalidControlsBoundsAndHandshakeCannotBecomeComplete() throws Exception {
    assertThrows(IllegalArgumentException.class,()->NativeSoakWorker075.main(new String[]{}));Path control=Files.createTempDirectory("imap075-native-soak-");
    try {
      assertFalse(NativeSoakWorker075.marker(control,"cancel"));Files.writeString(control.resolve("cancel"),"foreign");assertThrows(IllegalArgumentException.class,()->NativeSoakWorker075.marker(control,"cancel"));Files.writeString(control.resolve("cancel"),"cancel-v1");assertTrue(NativeSoakWorker075.marker(control,"cancel"));
      var bytes=new ByteArrayOutputStream();NativeSoakWorker075.run(config(control),new PrintStream(bytes));assertFalse(bytes.toString().contains("\"phase\":\"round\""));Files.delete(control.resolve("cancel"));
      assertThrows(IllegalStateException.class,()->NativeSoakWorker075.run(config(control),new PrintStream(new ByteArrayOutputStream())));
      assertThrows(IllegalArgumentException.class,()->new NativeSoakWorker075.Config(Long.MAX_VALUE,1,64,1,100,117,"a".repeat(40),control));
      assertThrows(IllegalArgumentException.class,()->new NativeSoakWorker075.Config(80,0,64,1,100,117,"a".repeat(40),control));
      assertThrows(IllegalArgumentException.class,()->new NativeSoakWorker075.Config(80,5,2,1,100,117,"a".repeat(40),control));
      assertThrows(IllegalArgumentException.class,()->new NativeSoakWorker075.Config(80,5,64,0,100,117,"a".repeat(40),control));
      assertThrows(IllegalArgumentException.class,()->new NativeSoakWorker075.Config(80,5,64,1,100,117,"foreign",control));
    } finally {Files.deleteIfExists(control.resolve("cancel"));Files.delete(control);}
  }
}
