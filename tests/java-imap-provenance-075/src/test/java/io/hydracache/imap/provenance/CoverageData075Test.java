package io.hydracache.imap.provenance;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import java.nio.file.*;
import java.util.*;
import static org.junit.jupiter.api.Assertions.*;

class CoverageData075Test {
  @TempDir Path root;
  private String report(String session,String line) {
    return "<report name='fixture'><sessioninfo id='"+session+"' start='1' dump='2'/><package name='io/hydracache'><sourcefile name='Fixture.java'>"+line+"</sourcefile></package></report>";
  }
  private Object read(String xml)throws Exception { Path file=root.resolve("coverage.xml");Files.writeString(file,xml);return CoverageDataMain075.read(file); }
  @Test void realJacocoShapeAndCli() throws Exception {
    String xml="<?xml version='1.0'?><!DOCTYPE report PUBLIC \"-//JACOCO//DTD Report 1.1//EN\" \"report.dtd\">"+report("fresh_1","<line nr='2' mi='0' ci='3' mb='0' cb='0'/>");
    var data=(Map<?,?>)read(xml);assertEquals("imap075-java-coverage-v1",data.get("schema"));
    assertEquals(Map.of("io/hydracache/Fixture.java",Map.of("2",3L)),data.get("sources"));
    CoverageDataMain075.main(new String[]{root.resolve("coverage.xml").toString()});
    assertThrows(IllegalArgumentException.class,()->CoverageDataMain075.main(new String[]{}));
    assertNotNull(read(report("fresh","")));
  }
  @Test void emptyMalformedForeignAndStaleInputsRefuse() throws Exception {
    String valid=report("fresh","<line nr='1' mi='1' ci='0'/>");
    for(String xml:List.of("","<foreign/>",valid.replace("<report ","<report xmlns='urn:foreign' "),
        "<!DOCTYPE report SYSTEM 'file:///private'>"+valid,
        valid.replace("fresh","bad/session"),valid.replace("dump='2'","dump='0'"),
        valid.replace("start='1'","start='-1'"),valid.replace("start='1'","start='9999999999999999999'"),
        valid.replace("<package","<sessioninfo id='fresh' start='1' dump='2'/><package"),
        valid.replace("io/hydracache","../foreign"),valid.replace("Fixture.java","../Fixture.java"),
        valid.replace("nr='1'","nr='0'"),valid.replace("ci='0'","ci='1000000001'"),
        valid.replace("mi='1'","mi='0'"),valid.replace("</sourcefile>","<line nr='1' mi='1' ci='0'/></sourcefile>"),
        valid.replace("</package>","<sourcefile name='Fixture.java'/></package>"),
        "<report><package name='a'><sourcefile name='A.java'/></package></report>",
        "<report><sessioninfo id='fresh' start='1' dump='2'/></report>")) {
      assertThrows(Exception.class,()->read(xml),xml);
    }
    Path file=root.resolve("invalid.xml");Files.write(file,new byte[]{(byte)0xff});assertThrows(Exception.class,()->CoverageDataMain075.read(file));
    Files.write(file,new byte[16*1024*1024+1]);assertThrows(Exception.class,()->CoverageDataMain075.read(file));
  }
  @Test void inventoryCliReadsOnlyBoundedUniquePomMaterials() throws Exception {
    Path pom=root.resolve("a.pom"),list=root.resolve("list");Files.writeString(pom,"<project xmlns='http://maven.apache.org/POM/4.0.0'/>");Files.writeString(list,"a.pom\n");
    MavenInventoryMain075.main(new String[]{root.toString(),list.toString()});
    assertThrows(Exception.class,()->MavenInventoryMain075.main(new String[]{}));
    for(String lines:List.of("a.pom\na.pom\n","../a.pom\n","list\n")) {Files.writeString(list,lines);assertThrows(Exception.class,()->MavenInventoryMain075.main(new String[]{root.toString(),list.toString()}));}
    Files.write(list,new byte[1024*1024+1]);assertThrows(Exception.class,()->MavenInventoryMain075.main(new String[]{root.toString(),list.toString()}));
  }
}
