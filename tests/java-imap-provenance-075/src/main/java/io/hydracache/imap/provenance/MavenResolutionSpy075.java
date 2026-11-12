package io.hydracache.imap.provenance;

import org.apache.maven.eventspy.EventSpy;
import org.apache.maven.execution.ExecutionEvent;
import org.eclipse.aether.RepositoryEvent;
import org.eclipse.aether.artifact.Artifact;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;

/** Records actual resolver inputs, including dynamically selected test-provider artifacts. */
public final class MavenResolutionSpy075 implements EventSpy {
  private BufferedWriter out; private Path root, repository, maven, jdk; private String nonce, stage;
  private boolean initialized, sessionEnded, buildSucceeded, failed, closed; private int records;
  private final Set<String> seen = new HashSet<>();
  private final Map<String,String> hashes = new HashMap<>();
  private static String property(Context context, String key) {
    Object supplied = context.getData().get("userProperties");
    String value = supplied instanceof Properties properties ? properties.getProperty(key) : null;
    if (value == null) value = System.getProperty(key);
    if (value == null || value.isBlank()) throw new IllegalArgumentException("required provenance property missing"); return value;
  }
  @Override public synchronized void init(Context context) throws Exception {
    root = Path.of(property(context,"hydracache.imap075.root")).toRealPath();
    repository = Path.of(property(context,"hydracache.imap075.repository")).toRealPath();
    Path directory = Path.of(property(context,"hydracache.imap075.output")).toRealPath();
    ProvenanceData075.relative(root, repository); ProvenanceData075.relative(root, directory);
    nonce = ProvenanceData075.token(property(context,"hydracache.imap075.nonce"),false);
    stage = property(context,"hydracache.imap075.stage");
    if (!Set.of("artifact-build","dependency-tree","effective-pom","tooling-test").contains(stage)) throw new IllegalArgumentException("unsupported provenance stage");
    maven = Path.of(System.getProperty("maven.home")).toRealPath(); jdk = Path.of(System.getProperty("java.home")).toRealPath();
    out = Files.newBufferedWriter(directory.resolve("maven-resolution.jsonl"),StandardCharsets.UTF_8,StandardOpenOption.CREATE_NEW);
    Object versions = context.getData().get("versionProperties"); String version = org.apache.maven.Maven.class.getPackage().getImplementationVersion();
    if(version == null) version = versions instanceof Properties props ? props.getProperty("version", "unknown") : "unknown";
    write(Map.of("event","init","schema","imap075-maven-resolution-v1","nonce",nonce,"stage",stage,"maven_version",version,"java_version",System.getProperty("java.runtime.version"),"java_vendor",System.getProperty("java.vendor")));
    for (String name : List.of("lib","boot")) {
      try (var files = Files.list(maven.resolve(name))) {
        var jars = files.filter(path -> path.getFileName().toString().endsWith(".jar")).sorted().toList();
        if (jars.size() > 256) throw new IOException("Maven distribution material bound");
        for (Path jar : jars) tool("maven",maven,jar);
      }
    }
    for (String path : List.of("release","lib/modules","bin/java" + (File.separatorChar == '\\' ? ".exe" : ""),"bin/javac" + (File.separatorChar == '\\' ? ".exe" : ""),"bin/jar" + (File.separatorChar == '\\' ? ".exe" : ""))) tool("jdk",jdk,jdk.resolve(path));
    tool("jdk",jdk,jdk.resolve(File.separatorChar=='\\'?"bin/server/jvm.dll":"lib/server/libjvm.so"));
    Path logging=maven.resolve("conf/logging");
    if(Files.isDirectory(logging))try(var files=Files.list(logging)){
      var configs=files.filter(Files::isRegularFile).sorted().toList();if(configs.size()>64)throw new IOException("Maven logging input bound");for(var config:configs)tool("maven",maven,config);
    }
    initialized = true;
  }
  private void tool(String kind,Path base,Path file) throws Exception {
    // Distribution entries may be system-package symlinks (e.g. /usr/share/maven/lib).
    // Record their fixed logical distribution name and hash the actual loaded bytes.
    String path=base.relativize(file).toString().replace('\\','/');
    if(path.startsWith("../") || path.contains(":")) throw new IOException("foreign distribution entry");
    write(Map.of("event","tool","kind",kind,"path",path,"sha256",ProvenanceData075.hash(file),"bytes",Files.size(file)));
  }
  private void write(Map<String,?> value) throws IOException {
    if (out == null || closed || ++records > 20000) throw new IOException("Maven provenance journal state/work bound");
    String json = ProvenanceData075.json(value); if (json.length() > 16384) throw new IOException("Maven material record bound"); out.write(json); out.newLine(); out.flush();
  }
  private void material(Artifact artifact) throws Exception {
    if (artifact == null || artifact.getFile() == null) throw new IOException("resolved material lacks file");
    Path file = artifact.getFile().toPath().toRealPath();
    String location, path;
    if (file.startsWith(repository)) {location="repository";path=ProvenanceData075.relative(repository,file);}
    else {location="workspace";path=ProvenanceData075.relative(root,file);}
    String id = String.join(":",ProvenanceData075.token(artifact.getGroupId(),false),ProvenanceData075.token(artifact.getArtifactId(),false),ProvenanceData075.token(artifact.getExtension(),false),ProvenanceData075.token(artifact.getClassifier(),true),ProvenanceData075.token(artifact.getVersion(),false));
    String hash = ProvenanceData075.hash(file);String previous = hashes.putIfAbsent(id,hash);
    if (previous != null && !previous.equals(hash)) throw new IOException("resolved coordinate changed bytes in one invocation");
    if (!seen.add(id + "|" + location + "|" + path)) return;
    var record = new TreeMap<String,Object>();
    record.put("event","material"); record.put("id",id); record.put("group",artifact.getGroupId()); record.put("name",artifact.getArtifactId()); record.put("version",artifact.getVersion());
    record.put("extension",artifact.getExtension()); record.put("classifier",artifact.getClassifier()); record.put("location",location); record.put("path",path); record.put("sha256",hash); record.put("bytes",Files.size(file)); write(record);
  }
  @Override public synchronized void onEvent(Object event) {
    try {
      if (event instanceof RepositoryEvent repositoryEvent && repositoryEvent.getType() == RepositoryEvent.EventType.ARTIFACT_RESOLVED) {
        if (repositoryEvent.getException() != null) throw new IOException("artifact resolution failed"); material(repositoryEvent.getArtifact());
      } else if (event instanceof ExecutionEvent execution) {
        if (execution.getType() == ExecutionEvent.Type.MojoStarted) {
          var mojo = execution.getMojoExecution(); var plugin = mojo.getPlugin();
          write(Map.of("event","mojo","group",ProvenanceData075.token(plugin.getGroupId(),false),"name",ProvenanceData075.token(plugin.getArtifactId(),false),"version",ProvenanceData075.token(plugin.getVersion(),false),"goal",ProvenanceData075.token(mojo.getGoal(),false)));
        } else if (execution.getType() == ExecutionEvent.Type.ProjectFailed) failed = true;
        else if (execution.getType() == ExecutionEvent.Type.SessionEnded) {sessionEnded=true;buildSucceeded=!execution.getSession().getResult().hasExceptions();}
      }
    } catch (Exception failure) {failed=true;}
  }
  @Override public synchronized void close() throws Exception {
    if (closed) return;
    try {if (out != null) write(Map.of("event","terminal","nonce",nonce,"stage",stage,"complete",initialized && sessionEnded && buildSucceeded && !failed,"records",records,"materials",seen.size()));}
    finally {closed=true;if(out != null) out.close();}
  }
}
