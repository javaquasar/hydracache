package io.hydracache.imap.hazelcast;

import java.util.Objects;
import java.util.Set;

/** Closed local observation profile. No product protocol capability is implied. */
public record NativeIMapProfile075(int version, Set<Field> observedFields) {
  public enum Field { RAW_RETURN, FINAL_DIGEST, NATIVE_EVENTS }
  public enum Operation { SIZE, IS_EMPTY, GET_OR_DEFAULT, CONTAINS_ENTRY, PUT_DETAILED,
    REMOVE_DETAILED, PUT_ALL_DIAGNOSTIC, ENTRY_VIEWS, SCAN, CLEAR, EVICT, EVICT_ALL }
  public static final int MAX_KEYS = 64, MAX_KEY_BYTES = 1024, MAX_VALUE_BYTES = 65536;
  public static final long MAX_REQUEST_BYTES = 64L * (1024 + 65536 + 32);
  public static final String ID = "native-exclusive-eternal-v1";
  public NativeIMapProfile075 {
    observedFields = Set.copyOf(Objects.requireNonNull(observedFields));
    if (version != 1 || !observedFields.equals(Set.of(Field.values())))
      throw new IllegalArgumentException("unsupported or weakened native observation profile");
  }
  public static NativeIMapProfile075 exclusive() { return new NativeIMapProfile075(1, Set.of(Field.values())); }
}
