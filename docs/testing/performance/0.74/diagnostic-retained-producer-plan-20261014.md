# Retained diagnostic request signing

Baseline is `e30740fe1bc746bf66d30368be0995538e3c3bf9`. The next local producer
step removes request-file reopening from an opt-in signing API. It does not
authenticate a signer executable, retrieve/verify a token or activate a job.
The approved dedicated-job design keeps those independent boundaries separate.

## Implementation scope

Add Linux `sign_prepared_files(prepared, observed_audience, key, output)`.
It consumes the original opaque PreparedDiagnosticRequest returned by
prepare_request/prepare_files; it takes no request, builder-policy or receipt
path. Before opening a signing key, recheck the retained canonical body digest,
builder identity, stored audience and exact observed audience, plus the existing
exclusive request time window. An observed audience is only a consistency input,
not token verification, trusted provenance or nonce authority.

Keep the selected controller pin, no-follow private key handling, explicit buffer
clearing, before/after signing clock checks and exclusive synced output unchanged.
The old sign_files/CLI route delegates through the same implementation after its
existing preparation. It remains a standalone one-shot file reader: two separate
audience/sign CLI invocations are not the retained coordinator. No new CLI,
handshake wire schema, HTTP producer, receipt, bootstrap trust root or server
route is introduced. Ownership consumption is not durable replay prevention.

## Tests and evidence

First capture the six existing producer library cases and a missing-API compile
refusal. Add portable model checks for audience/body/hash corruption and Linux
tests proving the original bytes are signed after public files are changed or
removed. A different valid preparation with the earlier audience must refuse
before opening a key. Wrong/empty/oversized audiences, expired request, wrong
controller key, unsafe key metadata and existing output retain refusal.

Use only owned temporary inputs and the existing explicit synthetic signing key,
never a real signing credential. Seed `0x07403b21` covers 32 body replacements
and four independent preparations. Add compile-fail no-clone/replacement/reuse
constraints. Run three focused repeats, producer/receiver OIDC regressions,
Linux public API/doctests, affected check and strict Clippy on Rust 1.94,
supervisor MSRV 1.88, root contracts/registry and local documentation gates.
Retain exact source, frozen/unchanged hashes, raw logs and negative outcomes in a
separate packet. No product performance measurement or full release milestone.

## Remaining bootstrap decision

The frozen observer builder statement cannot attest the controller signer and
its key purpose must not expand. Hashing a downloaded executable or a tool's own
reported hash is inventory, not independent custody. A future signing job needs
reviewed prebuilt executable/runtime custody before exposing token/signing
authority, with no compile/install/checkout after exposure. Actual artifact/pin
selection or a new signer-build receipt/key purpose requires a separate review;
this step neither chooses one nor pretends body consistency supplies it.

Production issuer/workflow/subject/expiry selection, credential transport,
immutable coordination through actual token verification, protected-job receipt,
authenticated handoff, original receiver/replay/fence/start and all-writer
cessation remain before server/pilot/qualification admission. Existing frozen
0.73/observer/qualification artifacts, workload, seed and thresholds stay exact.
