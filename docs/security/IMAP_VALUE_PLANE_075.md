# Extended IMap value-plane security foundation (0.75)

Status: provisional and non-production. The 0.75 value-plane capability remains disabled and
fail-closed until the 0.74 release artifacts named in the W0 status contract are published.

## Trust boundaries

The client/proxy boundary is untrusted. Tenant identity comes from authenticated context rather
than request payloads. The owner/backup boundary is also adversarial for correctness: an ACK is
not valid until the required backup proves the same epoch and version. Redirects, retries,
listener repair, bulk inputs, and evidence decoding all have finite bounds.

Namespace identity is `(tenant, namespace, generation)`. Delete/recreate never reuses an old
generation, and reclamation requires both the delete and cutover watermarks. This prevents stale
requests or delayed records from crossing a namespace lifetime.

## Required controls

- Bind tenant, client, request id, canonical payload digest, epoch, and namespace generation at
  the owner before apply.
- Treat duplicate identities with different payload digests as terminal attacks, not retries.
- Require same-epoch replica proof before acknowledgement or promotion.
- Bound redirects, decodes, bulk collections, histories, listener queues, and per-tenant dedup.
- Rotate trust using explicit epochs and a finite overlap window.
- Emit audit metadata and stable hashes only; never retain raw keys, values, credentials, or
  bearer tokens in logs or evidence receipts.

The machine-readable threat inventory and its adapted test IDs are in
`docs/testing/imap/0.75/security-contract.json`. `cargo xtask imap-contract-check --release 0.75`
rejects missing threats, mitigations, failure behavior, proof/bound references, and test IDs.
