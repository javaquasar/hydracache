package io.hydracache.imap.provenance;

import java.io.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.security.*;
import java.util.*;
import javax.xml.XMLConstants;
import javax.xml.parsers.DocumentBuilderFactory;
import org.w3c.dom.*;

/** Bounded tooling data only. Never reads or exports the environment or Maven credentials. */
public final class ProvenanceData075 {
  private ProvenanceData075() {}
  public static final long MAX_FILE = 512L * 1024 * 1024;
  static MessageDigest digest(String algorithm) {
    try { return MessageDigest.getInstance(algorithm); }
    catch (NoSuchAlgorithmException missing) { throw new IllegalStateException("required digest unavailable", missing); }
  }
  public static String hash(Path file) throws IOException {
    long size = Files.size(file);
    if (!Files.isRegularFile(file) || size < 1 || size > MAX_FILE) throw new IOException("material file type/size bound");
    var digest = digest("SHA-256"); long read = 0; byte[] block = new byte[32768];
    try (var input = Files.newInputStream(file)) {
      for (int count; (count = input.read(block)) != -1;) { read += count; if (read > size) throw new IOException("material grew while hashing"); digest.update(block, 0, count); }
    }
    if (read != size) throw new IOException("material truncated while hashing"); return HexFormat.of().formatHex(digest.digest());
  }
  public static String token(String value, boolean empty) {
    if (value == null || value.length() > 256 || (!empty && value.isEmpty()) || !value.matches("[A-Za-z0-9_.+~\\-]*") || value.equals(".") || value.equals("..")) throw new IllegalArgumentException("material coordinate bound");
    return value;
  }
  public static String relative(Path root, Path file) throws IOException {
    Path base = root.toRealPath(), actual = file.toRealPath();
    if (!actual.startsWith(base) || actual.equals(base)) throw new IOException("foreign material path");
    String value = base.relativize(actual).toString().replace('\\', '/');
    if (value.length() > 1024 || value.chars().anyMatch(Character::isISOControl)) throw new IOException("material path bound"); return value;
  }
  public static Path resolve(Path root, String relative) throws IOException {
    if (relative.isEmpty() || relative.length() > 1024 || relative.contains("\\") || relative.contains(":") || relative.chars().anyMatch(Character::isISOControl)) throw new IOException("material relative path bound");
    Path requested = Path.of(relative);
    if (requested.isAbsolute()) throw new IOException("absolute material path refused");
    for (var part : requested) if (part.toString().equals("..") || part.toString().equals(".")) throw new IOException("material path traversal refused");
    Path result = root.resolve(requested); relative(root, result); return result;
  }
  public static String json(Object value) { var out = new StringBuilder(); int[] work = {0}; append(out, value, 0, work); return out.toString(); }
  private static void add(StringBuilder out, String value) { if ((long)out.length() + value.length() > 8 * 1024 * 1024) throw new IllegalArgumentException("provenance JSON byte bound"); out.append(value); }
  private static void append(StringBuilder out, Object value, int depth, int[] work) {
    if (depth > 32 || ++work[0] > 20000) throw new IllegalArgumentException("provenance JSON work bound");
    if (value == null) { add(out, "null"); return; }
    if (value instanceof Boolean || value instanceof Integer || value instanceof Long) { add(out, value.toString()); return; }
    if (value instanceof String text) {
      if (text.length() > 8192) throw new IllegalArgumentException("provenance string bound"); add(out, "\"");
      for (char c : text.toCharArray()) {
        if (c == '"' || c == '\\') add(out, "\\" + c);
        else if (c < 32 || c > 126) add(out, String.format(Locale.ROOT, "\\u%04x", (int)c));
        else add(out, Character.toString(c));
      }
      add(out, "\""); return;
    }
    if (value instanceof Map<?,?> map) {
      if (map.size() > 20000) throw new IllegalArgumentException("JSON object work bound");
      var sorted = new TreeMap<String,Object>();
      for (var entry : map.entrySet()) { if (!(entry.getKey() instanceof String key)) throw new IllegalArgumentException("JSON object string key required"); sorted.put(key, entry.getValue()); }
      add(out, "{"); boolean first = true;
      for (var entry : sorted.entrySet()) { if (!first) add(out, ","); first = false; append(out, entry.getKey(), depth + 1, work); add(out, ":"); append(out, entry.getValue(), depth + 1, work); }
      add(out, "}"); return;
    }
    if (value instanceof Collection<?> values) {
      if (values.size() > 20000) throw new IllegalArgumentException("JSON array bound"); add(out, "["); boolean first = true;
      for (var item : values) { if (!first) add(out, ","); first = false; append(out, item, depth + 1, work); }
      add(out, "]"); return;
    }
    throw new IllegalArgumentException("unsupported provenance JSON value");
  }
  public static Document xml(Path file) throws Exception {
    byte[] bytes; try (var input = Files.newInputStream(file)) { bytes = input.readNBytes(2 * 1024 * 1024 + 1); }
    if (bytes.length > 2 * 1024 * 1024) throw new IOException("POM XML byte bound");
    var factory = DocumentBuilderFactory.newInstance(); factory.setNamespaceAware(true);
    factory.setFeature(XMLConstants.FEATURE_SECURE_PROCESSING, true);
    factory.setFeature("http://apache.org/xml/features/disallow-doctype-decl", true);
    factory.setAttribute(XMLConstants.ACCESS_EXTERNAL_DTD, ""); factory.setAttribute(XMLConstants.ACCESS_EXTERNAL_SCHEMA, "");
    factory.setAttribute("http://www.oracle.com/xml/jaxp/properties/maxElementDepth", "64");
    factory.setAttribute("http://www.oracle.com/xml/jaxp/properties/elementAttributeLimit", "64");
    factory.setXIncludeAware(false); factory.setExpandEntityReferences(false);
    return factory.newDocumentBuilder().parse(new ByteArrayInputStream(bytes));
  }
  static Element child(Element parent, String name) {
    for (var node = parent.getFirstChild(); node != null; node = node.getNextSibling()) if (node instanceof Element element && name.equals(element.getLocalName())) return element;
    return null;
  }
  static List<Element> children(Element parent, String name) {
    if (parent == null) return List.of(); var result = new ArrayList<Element>();
    for (var node = parent.getFirstChild(); node != null; node = node.getNextSibling()) if (node instanceof Element element && name.equals(element.getLocalName())) { if (result.size() == 1024) throw new IllegalArgumentException("POM element bound"); result.add(element); }
    return List.copyOf(result);
  }
  static String text(Element parent, String name) {
    var node = parent == null ? null : child(parent, name); String value = node == null ? "" : node.getTextContent().trim();
    if (value.length() > 4096 || value.chars().anyMatch(c -> c < 32 && c != '\n' && c != '\t')) throw new IllegalArgumentException("POM metadata string bound"); return value;
  }
}
