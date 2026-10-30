package io.hydracache.imap.provenance;

import java.io.File;
import java.nio.file.*;
import java.util.*;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import static org.junit.jupiter.api.Assertions.*;
import org.apache.maven.eventspy.EventSpy;
import org.apache.maven.execution.*;
import org.apache.maven.plugin.MojoExecution;
import org.apache.maven.model.Plugin;
import org.apache.maven.project.MavenProject;
import org.eclipse.aether.*;
import org.eclipse.aether.artifact.DefaultArtifact;

class MavenResolutionSpy075Test {
  @TempDir Path root;
  record Fixture(MavenResolutionSpy075 spy,Path output,Path repository) {}
  private Fixture fixture(String name) throws Exception {
    Path directory=Files.createDirectories(root.resolve(name)), repo=Files.createDirectories(root.resolve("repository"));
    var props=new Properties();props.setProperty("hydracache.imap075.root",root.toString());props.setProperty("hydracache.imap075.repository",repo.toString());props.setProperty("hydracache.imap075.output",directory.toString());props.setProperty("hydracache.imap075.nonce",name);props.setProperty("hydracache.imap075.stage","artifact-build");
    Path maven=Files.createDirectories(root.resolve("maven")),jdk=Files.createDirectories(root.resolve("jdk"));
    for(String path:List.of("lib/core.jar","boot/boot.jar")){Path p=maven.resolve(path);Files.createDirectories(p.getParent());Files.writeString(p,"tool");}
    for(String path:List.of("release","lib/modules","bin/java","bin/javac","bin/jar")){if(File.separatorChar=='\\'&&path.startsWith("bin/"))path+=".exe";Path p=jdk.resolve(path);Files.createDirectories(p.getParent());Files.writeString(p,"tool");}
    Path vm=jdk.resolve(File.separatorChar=='\\'?"bin/server/jvm.dll":"lib/server/libjvm.so");Files.createDirectories(vm.getParent());Files.writeString(vm,"vm-tool");
    Path logging=maven.resolve("conf/logging/logger.properties");Files.createDirectories(logging.getParent());Files.writeString(logging,"public-tool-config");
    String oldMaven=System.getProperty("maven.home"),oldJava=System.getProperty("java.home");var spy=new MavenResolutionSpy075();
    try{System.setProperty("maven.home",maven.toString());System.setProperty("java.home",jdk.toString());spy.init(()->Map.of("userProperties",props));}
    finally{if(oldMaven==null)System.clearProperty("maven.home");else System.setProperty("maven.home",oldMaven);System.setProperty("java.home",oldJava);}
    return new Fixture(spy,directory.resolve("maven-resolution.jsonl"),repo);
  }
  private static ExecutionEvent event(ExecutionEvent.Type type,boolean failure){
    var result=new DefaultMavenExecutionResult();if(failure)result.addException(new Exception("fixture"));
    var session=new MavenSession(null,(org.eclipse.aether.RepositorySystemSession)null,new DefaultMavenExecutionRequest(),result);
    var plugin=new Plugin();plugin.setGroupId("org.example");plugin.setArtifactId("plugin");plugin.setVersion("1");var mojo=new MojoExecution(plugin,"test","fixture");
    return new ExecutionEvent(){public Type getType(){return type;}public MavenSession getSession(){return session;}public MavenProject getProject(){return new MavenProject();}public MojoExecution getMojoExecution(){return mojo;}public Exception getException(){return null;}};
  }
  private static RepositoryEvent material(Path file,Exception failure){return new RepositoryEvent.Builder(new DefaultRepositorySystemSession(),RepositoryEvent.EventType.ARTIFACT_RESOLVED).setArtifact(new DefaultArtifact("org.example:library:jar:1").setFile(file.toFile())).setException(failure).build();}
  @Test void actualMaterialsMojoTerminalAndIdempotentClose() throws Exception{
    var f=fixture("complete");Path jar=f.repository.resolve("library.jar");Files.writeString(jar,"library");
    f.spy.onEvent(material(jar,null));f.spy.onEvent(material(jar,null));f.spy.onEvent(event(ExecutionEvent.Type.MojoStarted,false));f.spy.onEvent(new Object());f.spy.onEvent(event(ExecutionEvent.Type.SessionEnded,false));f.spy.close();f.spy.close();
    String text=Files.readString(f.output);assertTrue(text.contains("\"complete\":true"));assertTrue(text.contains("\"materials\":1"));assertTrue(text.contains("\"event\":\"mojo\""));assertFalse(text.contains(root.toString()));
  }
  @Test void incompleteAndSwallowedResolverErrorsRemainIncomplete() throws Exception{
    for(int mode=0;mode<5;mode++){
      var f=fixture("failed"+mode);Path jar=f.repository.resolve("library.jar");Files.writeString(jar,"library");f.spy.onEvent(material(jar,null));
      switch(mode){case 0:break;case 1:f.spy.onEvent(material(jar,new Exception("fixture")));break;case 2:Files.writeString(jar,"substituted");f.spy.onEvent(material(jar,null));break;case 3:f.spy.onEvent(event(ExecutionEvent.Type.ProjectFailed,false));break;case 4:break;default:throw new AssertionError();}
      if(mode!=0)f.spy.onEvent(event(ExecutionEvent.Type.SessionEnded,mode==4));f.spy.close();assertTrue(Files.readString(f.output).contains("\"complete\":false"),"mode="+mode);
    }
  }
  @Test void missingContextRefusesBeforeAnyJournal() throws Exception{
    // Surefire promotes the parent's user properties, including a real spy context.
    // An explicit blank supplied value must still refuse instead of falling back to it.
    var properties=new Properties();properties.setProperty("hydracache.imap075.root","");
    var spy=new MavenResolutionSpy075();assertThrows(IllegalArgumentException.class,()->spy.init(()->Map.of("userProperties",properties)));spy.close();
  }
}
