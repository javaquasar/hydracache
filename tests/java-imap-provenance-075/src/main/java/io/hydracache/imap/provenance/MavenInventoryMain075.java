package io.hydracache.imap.provenance;

import java.io.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import org.w3c.dom.Element;

/** Selected public POM metadata only; inherited or absent license assertions remain unknown. */
public final class MavenInventoryMain075 {
  private MavenInventoryMain075() {}
  public static Map<String,Object> pom(Path path) throws Exception {
    Element project = ProvenanceData075.xml(path).getDocumentElement();
    if (!"project".equals(project.getLocalName())) throw new IOException("POM project required");
    var licenses = new ArrayList<Object>();
    for (var license : ProvenanceData075.children(ProvenanceData075.child(project,"licenses"),"license")) {
      String name = ProvenanceData075.text(license,"name"), url = publicUrl(ProvenanceData075.text(license,"url"));
      if (!name.isEmpty()) licenses.add(Map.of("name",name,"url",url));
    }
    var result = new TreeMap<String,Object>();
    result.put("sha256",ProvenanceData075.hash(path)); result.put("licenses",licenses);
    result.put("license_origin",licenses.isEmpty() ? "unknown" : "direct-pom-declaration");
    result.put("source_url", publicUrl(ProvenanceData075.text(ProvenanceData075.child(project,"scm"),"url")));
    var parent = ProvenanceData075.child(project,"parent");
    result.put("parent",parent == null ? null : Map.of("group",ProvenanceData075.text(parent,"groupId"),"name",ProvenanceData075.text(parent,"artifactId"),"version",ProvenanceData075.text(parent,"version")));
    return result;
  }
  static String publicUrl(String text) {
    if (text.isEmpty()) return "";
    try { var uri = java.net.URI.create(text); if (uri.getScheme() == null || !Set.of("https","http").contains(uri.getScheme()) || uri.getHost() == null || uri.getUserInfo() != null || uri.getQuery() != null || uri.getFragment() != null) return ""; return text; }
    catch (IllegalArgumentException invalid) { return ""; }
  }
  public static void main(String[] args) throws Exception {
    if (args.length != 2) throw new IllegalArgumentException("repository and material list required");
    Path root = Path.of(args[0]).toRealPath(), list = Path.of(args[1]);
    if (Files.size(list) > 1024 * 1024) throw new IOException("POM list byte bound");
    var result = new TreeMap<String,Object>();
    try (var input = Files.newBufferedReader(list,StandardCharsets.UTF_8)) {
      for (String line; (line = input.readLine()) != null;) {
        if (result.size() >= 4096 || result.containsKey(line)) throw new IOException("duplicate/work bound in POM list");
        Path path = ProvenanceData075.resolve(root,line);
        if (!line.endsWith(".pom") && !line.endsWith("/pom.xml")) throw new IOException("POM material suffix required");
        result.put(line,pom(path));
      }
    }
    System.out.println(ProvenanceData075.json(result));
  }
}
