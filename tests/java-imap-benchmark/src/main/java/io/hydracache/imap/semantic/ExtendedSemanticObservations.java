package io.hydracache.imap.semantic;

import static io.hydracache.imap.semantic.ExtendedSemanticScenario.*;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.Optional;
import java.util.TreeMap;
import java.util.function.Consumer;

/** Bounded JSON observation writer for development process comparison, not a production codec. */
public final class ExtendedSemanticObservations {
  public static final String FORMAT = "imap-extended-observations-v2";
  public static final int MAX_LINE_BYTES = 1024 * 1024;
  public static final int MAX_OUTPUT_BYTES = 16 * 1024 * 1024;
  private ExtendedSemanticObservations() {}
  public static String json(Observation observation) {
    var json = new Json(); json.raw("{\"result\":"); json.result(observation.result());
    json.raw(",\"snapshot\":"); json.array(observation.snapshot(), row -> { json.raw("{\"key\":"); json.string(row.key().hex()); json.raw(",\"value\":"); json.string(row.value().hex()); json.raw(",\"ttl\":"); json.ttl(row.ttl()); json.raw("}"); });
    json.raw(",\"events\":"); json.array(observation.events(), event -> { json.raw("{\"kind\":"); json.string(event.kind()); json.raw(",\"key\":"); json.string(event.key().hex()); json.raw(",\"old_value\":"); json.bytes(event.oldValue()); json.raw(",\"value\":"); json.bytes(event.value()); json.raw(",\"watermark\":" + event.watermark() + "}"); });
    json.raw(",\"token_relations\":{"); boolean comma = false;
    for (var entry : new TreeMap<>(observation.tokenRelations()).entrySet()) { if (comma) json.raw(","); comma = true; json.string(entry.getKey()); json.raw(":" + entry.getValue()); }
    var owners = observation.owners();
    json.raw("},\"owners\":{\"outcomes\":" + owners.outcomes() + ",\"pending\":" + owners.pending() + ",\"listeners\":" + owners.listeners() + ",\"snapshot_payload\":" + owners.snapshotPayload() + "},\"tick\":" + observation.tick() + "}");
    return json.text.toString();
  }
  private static final class Json {
    final StringBuilder text = new StringBuilder(); int bytes;
    void raw(String value) { int length = value.getBytes(StandardCharsets.UTF_8).length; if (length > MAX_LINE_BYTES - bytes) throw new IllegalArgumentException("observation byte bound"); text.append(value); bytes += length; }
    void string(String value) {
      ExtendedSemanticCorpus.textHex(value); raw("\"");
      for (int index = 0; index < value.length();) {
        int codepoint = value.codePointAt(index); index += Character.charCount(codepoint);
        if (codepoint == '"') raw("\\\""); else if (codepoint == '\\') raw("\\\\");
        else if (codepoint < 32) raw(String.format(java.util.Locale.ROOT, "\\u%04x", codepoint));
        else raw(new String(Character.toChars(codepoint)));
      }
      raw("\"");
    }
    void bytes(Optional<BytesValue> value) { if (value.isPresent()) string(value.orElseThrow().hex()); else raw("null"); }
    void ttl(TtlState ttl) { raw("{\"kind\":"); string(ttl.kind().name().toLowerCase(java.util.Locale.ROOT)); raw(",\"remaining_ticks\":" + ttl.remainingTicks() + "}"); }
    <T> void array(List<T> items, Consumer<T> write) { raw("["); boolean comma = false; for (var item : items) { if (comma) raw(","); comma = true; write.accept(item); } raw("]"); }
    void result(Result result) {
      raw("{\"kind\":"); string(result.kind()); raw(",\"value\":"); bytes(result.value()); raw(",\"ttl\":"); ttl(result.ttl()); raw(",\"items\":");
      array(result.items(), item -> { raw("{\"index\":" + item.index() + ",\"key\":"); string(item.key().hex()); raw(",\"kind\":"); string(item.kind()); raw(",\"value\":"); bytes(item.value()); raw(",\"ttl\":"); ttl(item.ttl()); raw("}"); });
      raw(",\"statuses\":"); array(result.statuses(), status -> { raw("{\"identity\":{\"client\":"); string(status.identity().client()); raw(",\"sequence\":" + status.identity().sequence() + "},\"kind\":"); string(status.kind()); raw(",\"result\":"); if (status.result().isPresent()) result(status.result().orElseThrow()); else raw("null"); raw("}"); });
      raw(",\"number\":" + result.number() + "}");
    }
  }
}
