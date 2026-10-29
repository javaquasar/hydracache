# Official Hazelcast diagnostic adapter for 0.75 preparation

This separate consumer uses the packaged provisional Java semantic SPI and the real official
Hazelcast 5.5.0 client over a loopback network. Its member uses BINARY storage, no backups and no
TLS. The adapter admits exclusive sequential eternal-value scenarios only; it has no retained
mutation identity guarantee and makes no concurrent, production S2, timing or performance claim.
Finite native TTL and logical-time advancement are explicitly incomparable. Native listener
events come from Hazelcast; queue overflow, eviction, expiry or whole-map changes expose a gap.

The Maven coordinate is `com.hazelcast:hazelcast:5.5.0`, Apache-2.0, source tag `v5.5.0`, commit
`e6ef7a8a874e445754bf6b9ab04056d812d993da`. The JAR and POM checksums are recorded in
`HazelcastArtifactPin`; the loaded JAR is verified before client-map attachment. This GA source
identity differs from the development sources reviewed in `hazelcast-source-map.toml`.

```powershell
mvn -B -f tests/java-imap-benchmark/pom.xml install
mvn -B -f tests/java-imap-hazelcast-075/pom.xml verify
```

The consumer compiles against the installed JAR rather than adding benchmark source directories.
The core semantic harness keeps its default product adapters fail-closed. Selecting this explicit
diagnostic adapter cannot enable HydraCache production capabilities or release admission.
