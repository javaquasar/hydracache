package io.hydracache.imap.provenance;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import java.nio.file.*;
import java.util.*;
import static org.junit.jupiter.api.Assertions.*;

class ProvenanceData075Test {
  @TempDir Path root;
  @Test void boundedCanonicalJsonAndCoordinates() {
    assertEquals("{\"a\":[true,null,\"\\u00ff\\u000a\"],\"z\":1}",ProvenanceData075.json(Map.of("z",1,"a",Arrays.asList(true,null,"ÿ\n"))));
    assertEquals("",ProvenanceData075.token("",true));
    for (String bad : List.of("","..",".","a/b","secret@host","x".repeat(257))) assertThrows(IllegalArgumentException.class,()->ProvenanceData075.token(bad,false));
    assertThrows(IllegalArgumentException.class,()->ProvenanceData075.json(1.5));
    assertThrows(IllegalArgumentException.class,()->ProvenanceData075.json(Map.of(1,1)));
    assertThrows(IllegalArgumentException.class,()->ProvenanceData075.json("x".repeat(8193)));
    Object nested = 1; for(int i=0;i<34;i++) nested=List.of(nested); Object deep=nested;
    assertThrows(IllegalArgumentException.class,()->ProvenanceData075.json(deep));
    assertThrows(IllegalStateException.class,()->ProvenanceData075.digest("not-a-digest"));
  }
  @Test void ownedPathsAndExactBytes() throws Exception {
    Path file=root.resolve("a.pom");Files.writeString(file,"abc");
    assertEquals("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",ProvenanceData075.hash(file));
    assertEquals("a.pom",ProvenanceData075.relative(root,file)); assertEquals(file,ProvenanceData075.resolve(root,"a.pom"));
    for(String bad:List.of("","../a.pom","./a.pom","C:/a.pom","a\\b")) assertThrows(Exception.class,()->ProvenanceData075.resolve(root,bad));
    assertThrows(Exception.class,()->ProvenanceData075.relative(root,root));
    Files.writeString(file,""); assertThrows(Exception.class,()->ProvenanceData075.hash(file));
  }
  @Test void pomAssertionsHaveExplicitOriginAndRejectExternalEntities() throws Exception {
    Path file=root.resolve("a.pom");
    Files.writeString(file,"<project xmlns='http://maven.apache.org/POM/4.0.0'><licenses><license><name>Apache-2.0</name><url>https://apache.org/license</url></license></licenses><parent><groupId>a</groupId><artifactId>b</artifactId><version>1</version></parent><scm><url>https://github.com/javaquasar/hydracache</url></scm></project>");
    var metadata=MavenInventoryMain075.pom(file); assertEquals("direct-pom-declaration",metadata.get("license_origin"));assertNotNull(metadata.get("parent"));
    Files.writeString(file,"<project xmlns='http://maven.apache.org/POM/4.0.0'/>"); assertEquals("unknown",MavenInventoryMain075.pom(file).get("license_origin"));
    Files.writeString(file,"<!DOCTYPE a [<!ENTITY external SYSTEM 'file:///private'>]><project>&external;</project>"); assertThrows(Exception.class,()->MavenInventoryMain075.pom(file));
    for(String bad:List.of("https://user:password@example.com/path","https://example.com/?token=x","bad uri","file:///private","scm:git:ssh://git@host/repo","relative/path")) assertEquals("",MavenInventoryMain075.publicUrl(bad));
    Files.writeString(file,"<project>".repeat(65)+"</project>".repeat(65));assertThrows(Exception.class,()->MavenInventoryMain075.pom(file));
    Files.writeString(file,"<foreign/>");assertThrows(Exception.class,()->MavenInventoryMain075.pom(file));
  }
}
