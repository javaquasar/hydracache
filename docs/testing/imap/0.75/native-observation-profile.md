# Native observation development profile v1

This profile qualifies bounded observations from the pinned Hazelcast 5.5.0 official
client and an isolated loopback member. It does not qualify HydraCache production
dispatch, distributed transactions, backup acknowledgements, TTL timing or S2.

The closed required field set is `raw_return`, `final_digest`, `native_events`.
Removing any field refuses the profile before I/O. Owners, Hydra version tokens,
logical ticks and partition watermarks are absent from this contract. EntryView's
native version and TTL remain native metadata with their original units.

| Family | Origin | Observable result |
| --- | --- | --- |
| size / isEmpty | Native single call | Native integer / boolean |
| getOrDefault / containsEntry | Client helper, exclusive map | Native get plus binary comparison/default |
| detailed put / remove | Client helper, bounded point calls | Actual previous value for every call |
| putAll diagnostic | Native aggregate call | Void return; no per-item success fabrication |
| entry views / scan | Client helper, quiescent map | Bounded native metadata and binary entries |
| clear / evict / evictAll | Native lifecycle calls | Actual return and aggregate/entry callbacks |

Requests admit at most 64 distinct keys, 1024 bytes per key and 65536 bytes per
value. The complete request and prospective map cardinality are validated before
the first mutation. Empty batches are explicitly refused. Byte equality determines
duplicates; Java array identity does not. Iterator bounds also apply when external
state violates the fixture's exclusive ownership.

Transport failure during a mutating call is an explicit uncertain observation.
Neither remaining items nor an interrupted aggregate call are labelled NotApplied.
Detailed helpers report native point-call counts separately from preflight reads
and diagnostic snapshots. They are not a native bulk performance comparison.

Native callbacks retain their actual kind, binary key and available old/new value.
Aggregate clear/eviction callbacks retain affected-entry counts and never expand
into invented per-key removals. Arrival order across different keys is not request
order. Recovery registration generations are local subscription identities only.

Concurrent histories use a separate fixed-key eternal cohort and preserve one
actual native call per recorded operation. In particular a failed conditional and
a subsequent get cannot be combined into an allegedly atomic result.
