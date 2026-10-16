# Collector Starting byte record before publication

After the validation-frame fault gate, implement the byte-only first slice of
the approved durable Starting plan. It does not publish or change storage phase.
The original Reserved reader and its exact two-file inventory remain unchanged.

Register diagnostic-collector-starting-074-v1 with the fixed eventual filename
diagnostic-collector-starting-074.json and a canonical LF limit of 16384 bytes.
Keep a separate typed record containing schema, exact controller-attempt and
collector-Reserved LF digests, complete signed consent, its reconstructed exact
canonical LF digest, and the identical Starting state. Embed the typed envelope
rather than a byte array that expands every byte; accepting a noncanonical
original envelope is forbidden. Its existing 8192-byte limit remains exact.

Place the Linux-only byte API in a child of the existing collector Reserved
module. Reuse its private typed prior records and its existing paired validator;
do not introduce a looser alternative parser for earlier history. A pure encoder
and strict inspector may expose bytes or Result<(), Error>, never an owner,
acknowledgement, trusted key/peer/time input, recovery or start API. Inspecting
arbitrary valid signature-shaped bytes does not verify signatures or enroll
authority. The future consuming publisher must still use the original admitted
consent, actual kernel observations, live frame and original storage objects.

Require exact old request/controller/source/preset/worker-policy/epoch bindings,
canonical Reserved-state and collector-unit hashes and the unchanged adjacent
Reserved-to-Starting transition. Starting and the frozen signed consent state
must compare in full, with the original reservation origin retained. Neither
signed times nor an inspected record become a fresh runtime observation.

First retain an unresolved-entry compile failure. Add clock-free deterministic
unit tests for roundtrip, all strict prefixes, malformed/unknown/duplicate/
future/oversized/noncanonical input, prior-history substitution, invalid consent
grammar and every record/body/state binding leaf. Log the seed used to order
mutations and prove that signature-shaped bytes alone cannot provide authority.
Compile-fail examples must prevent turning byte results into publish/start rights.

Add a COMPAT row and exact narrow historical restoration for the new child
registration, row and static adapter. Existing predicate bodies and frozen test
assertions must remain; changed-registration and unapproved-tail canaries refuse.
Run current/MSRV tests, affected check/strict Clippy, static/docs/registry/local
contracts and formatting before the small implementation commit. Retain evidence
separately. Actual exclusive storage phase, write/seal/fsync/acknowledgement,
SIGKILL/lost-ack proof, replay fence, execution/all-writer closure and server
installation/rehearsal remain later steps, not claims of this byte slice.
